use crate::error::VaultError;
use crate::mcp::models::{McpEntry, McpSource};
use crate::registry::Registry;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

#[async_trait]
pub trait McpManager: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn install(
        &self,
        name: &str,
        source: McpSource,
        version_req: &str,
        args: Vec<String>,
        env_vars: std::collections::HashMap<String, String>,
        agents: Vec<String>,
        tags: Vec<String>,
        description: Option<String>,
    ) -> Result<McpEntry, VaultError>;
    async fn remove(&self, name: &str, keep_files: bool) -> Result<(), VaultError>;
    async fn update(&self, name: &str, force: bool) -> Result<McpEntry, VaultError>;
    fn get(&self, name: &str) -> Result<McpEntry, VaultError>;
    fn list(&self) -> Result<Vec<McpEntry>, VaultError>;
}

pub struct DefaultMcpManager {
    registry: Arc<dyn Registry>,
    #[allow(dead_code)]
    vault_dir: PathBuf,
}

impl DefaultMcpManager {
    pub fn new(registry: Arc<dyn Registry>, vault_dir: PathBuf) -> Self {
        Self {
            registry,
            vault_dir,
        }
    }
}

#[async_trait]
impl McpManager for DefaultMcpManager {
    #[allow(clippy::too_many_arguments)]
    async fn install(
        &self,
        name: &str,
        source: McpSource,
        version_req: &str,
        args: Vec<String>,
        env_vars: std::collections::HashMap<String, String>,
        agents: Vec<String>,
        tags: Vec<String>,
        description: Option<String>,
    ) -> Result<McpEntry, VaultError> {
        if let McpSource::Local { ref path } = source {
            if !path.exists() {
                return Err(VaultError::NotFound {
                    kind: "local_path".to_string(),
                    name: path.display().to_string(),
                });
            }
            let target_link = self.vault_dir.join("mcps").join(name);
            if let Some(parent) = target_link.parent() {
                std::fs::create_dir_all(parent)?;
            }
            clean_target_dir(&target_link)?;

            #[cfg(unix)]
            std::os::unix::fs::symlink(path, &target_link)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(path, &target_link)?;

            let entry = McpEntry {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                display_name: Some(name.to_string()),
                version: "1.0.0".to_string(), // Local defaults to 1.0.0 or parses package file if available
                source: source.clone(),
                install_path: target_link,
                command: "node".to_string(), // Local entry could define custom script runner, placeholder for now
                args: args.clone(),
                env_vars: env_vars.clone(),
                transport: crate::mcp::models::McpTransport::Stdio,
                status: crate::mcp::models::McpStatus::Active,
                installed_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                checksum: None,
                agents: agents.clone(),
                tags: tags.clone(),
                description: description.clone(),
            };

            self.registry.insert_mcp(&entry)?;
            return Ok(entry);
        }

        if let McpSource::Npm { ref package } = source {
            // 1. Validate npm is available
            let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
            match tokio::process::Command::new(npm_cmd)
                .arg("--version")
                .output()
                .await
            {
                Ok(output) if output.status.success() => {}
                _ => {
                    return Err(VaultError::McpInstall {
                        source_type: "npm".to_string(),
                        message: "npm executable not found in PATH".to_string(),
                    });
                }
            }

            // 2. Create the target folder
            let target_dir = self.vault_dir.join("mcps").join(name);
            if let Some(parent) = target_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }
            clean_target_dir(&target_dir)?;
            std::fs::create_dir_all(&target_dir)?;

            // 3. Formulate package specification
            let package_spec = if version_req == "latest" || version_req.is_empty() {
                package.to_string()
            } else {
                format!("{}@{}", package, version_req)
            };

            // 4. Run npm install
            let output = tokio::process::Command::new(npm_cmd)
                .arg("install")
                .arg("--prefix")
                .arg(&target_dir)
                .arg(&package_spec)
                .output()
                .await;

            match output {
                Ok(out) if out.status.success() => {}
                Ok(out) => {
                    let _ = std::fs::remove_dir_all(&target_dir);
                    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                    return Err(VaultError::McpInstall {
                        source_type: "npm".to_string(),
                        message: format!(
                            "npm install failed with status {}: {}",
                            out.status, stderr
                        ),
                    });
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&target_dir);
                    return Err(VaultError::McpInstall {
                        source_type: "npm".to_string(),
                        message: format!("Failed to run npm: {}", e),
                    });
                }
            }

            // 5. Resolve binary script
            let (cmd_name, mut resolved_args) = resolve_npm_bin(&target_dir, package)?;
            resolved_args.extend(args.clone());

            // 6. Get package version from package.json
            let pkg_json_path = target_dir
                .join("node_modules")
                .join(package)
                .join("package.json");
            let version = if pkg_json_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&pkg_json_path) {
                    if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
                        pkg.get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("1.0.0")
                            .to_string()
                    } else {
                        "1.0.0".to_string()
                    }
                } else {
                    "1.0.0".to_string()
                }
            } else {
                "1.0.0".to_string()
            };

            // 7. Construct entry
            let entry = McpEntry {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                display_name: Some(name.to_string()),
                version,
                source: source.clone(),
                install_path: target_dir,
                command: cmd_name,
                args: resolved_args,
                env_vars: env_vars.clone(),
                transport: crate::mcp::models::McpTransport::Stdio,
                status: crate::mcp::models::McpStatus::Active,
                installed_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                checksum: None,
                agents: agents.clone(),
                tags: tags.clone(),
                description: description.clone(),
            };

            self.registry.insert_mcp(&entry)?;
            return Ok(entry);
        }

        if let McpSource::PyPi { ref package } = source {
            // Check if uv or python is present
            let has_uv = match tokio::process::Command::new("uv")
                .arg("--version")
                .output()
                .await
            {
                Ok(output) => output.status.success(),
                _ => false,
            };

            let python_cmd = if match tokio::process::Command::new("python3")
                .arg("--version")
                .output()
                .await
            {
                Ok(output) => output.status.success(),
                _ => false,
            } {
                Some("python3".to_string())
            } else if match tokio::process::Command::new("python")
                .arg("--version")
                .output()
                .await
            {
                Ok(output) => output.status.success(),
                _ => false,
            } {
                Some("python".to_string())
            } else {
                None
            };

            if !has_uv && python_cmd.is_none() {
                return Err(VaultError::McpInstall {
                    source_type: "pypi".to_string(),
                    message: "Neither 'uv' nor 'python3'/'python' executable found in PATH"
                        .to_string(),
                });
            }

            let target_dir = self.vault_dir.join("mcps").join(name);
            if let Some(parent) = target_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }
            clean_target_dir(&target_dir)?;
            std::fs::create_dir_all(&target_dir)?;

            let venv_dir = target_dir.join("venv");
            let package_spec = if version_req == "latest" || version_req.is_empty() {
                package.to_string()
            } else {
                format!("{}=={}", package, version_req)
            };

            let run_install = async {
                if has_uv {
                    run_cmd("uv", &["venv", &venv_dir.to_string_lossy()], "pypi").await?;
                    let venv_python = if cfg!(windows) {
                        venv_dir.join("Scripts").join("python.exe")
                    } else {
                        venv_dir.join("bin").join("python")
                    };
                    run_cmd(
                        "uv",
                        &[
                            "pip",
                            "install",
                            "--python",
                            &venv_python.to_string_lossy(),
                            &package_spec,
                        ],
                        "pypi",
                    )
                    .await?;
                } else if let Some(ref py_cmd) = python_cmd {
                    run_cmd(py_cmd, &["-m", "venv", &venv_dir.to_string_lossy()], "pypi").await?;
                    let pip_path = if cfg!(windows) {
                        venv_dir.join("Scripts").join("pip.exe")
                    } else {
                        venv_dir.join("bin").join("pip")
                    };
                    run_cmd(
                        &pip_path.to_string_lossy(),
                        &["install", &package_spec],
                        "pypi",
                    )
                    .await?;
                }
                Ok::<(), VaultError>(())
            };

            if let Err(e) = run_install.await {
                let _ = std::fs::remove_dir_all(&target_dir);
                return Err(e);
            }

            // Resolve python version from the installed package in venv
            let venv_python = if cfg!(windows) {
                venv_dir.join("Scripts").join("python.exe")
            } else {
                venv_dir.join("bin").join("python")
            };

            let version = match tokio::process::Command::new(&venv_python)
                .arg("-c")
                .arg("import sys, importlib.metadata; print(importlib.metadata.version(sys.argv[1]))")
                .arg(package)
                .output()
                .await
            {
                Ok(output) if output.status.success() => {
                    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if ver.is_empty() {
                        if version_req == "latest" || version_req.is_empty() {
                            "1.0.0".to_string()
                        } else {
                            version_req.to_string()
                        }
                    } else {
                        ver
                    }
                }
                _ => {
                    match tokio::process::Command::new(&venv_python)
                        .arg("-c")
                        .arg("import sys, pkg_resources; print(pkg_resources.get_distribution(sys.argv[1]).version)")
                        .arg(package)
                        .output()
                        .await
                    {
                        Ok(output) if output.status.success() => {
                            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            if ver.is_empty() {
                                if version_req == "latest" || version_req.is_empty() {
                                    "1.0.0".to_string()
                                } else {
                                    version_req.to_string()
                                }
                            } else {
                                ver
                            }
                        }
                        _ => {
                            if version_req == "latest" || version_req.is_empty() {
                                "1.0.0".to_string()
                            } else {
                                version_req.to_string()
                            }
                        }
                    }
                }
            };

            let (cmd_name, mut resolved_args) = resolve_pypi_cmd(&venv_dir, package);
            resolved_args.extend(args.clone());

            let entry = McpEntry {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                display_name: Some(name.to_string()),
                version,
                source: source.clone(),
                install_path: target_dir,
                command: cmd_name,
                args: resolved_args,
                env_vars: env_vars.clone(),
                transport: crate::mcp::models::McpTransport::Stdio,
                status: crate::mcp::models::McpStatus::Active,
                installed_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                checksum: None,
                agents: agents.clone(),
                tags: tags.clone(),
                description: description.clone(),
            };

            self.registry.insert_mcp(&entry)?;
            return Ok(entry);
        }

        Err(VaultError::NotFound {
            kind: "mcp".to_string(),
            name: name.to_string(),
        })
    }

    async fn remove(&self, name: &str, keep_files: bool) -> Result<(), VaultError> {
        let entry = self.registry.get_mcp(name)?;
        if !keep_files {
            clean_target_dir(&entry.install_path)?;
        }
        self.registry.delete_mcp(name)?;
        Ok(())
    }

    async fn update(&self, name: &str, force: bool) -> Result<McpEntry, VaultError> {
        let entry = self.registry.get_mcp(name)?;
        let version_req = if force {
            "latest".to_string()
        } else {
            entry.version.clone()
        };

        // Temporarily delete to avoid insert_mcp's AlreadyExists check
        self.registry.delete_mcp(name)?;

        let result = self
            .install(
                &entry.name,
                entry.source.clone(),
                &version_req,
                entry.args.clone(),
                entry.env_vars.clone(),
                entry.agents.clone(),
                entry.tags.clone(),
                entry.description.clone(),
            )
            .await;

        match result {
            Ok(new_entry) => Ok(new_entry),
            Err(e) => {
                // Try to restore registry entry on failure
                let _ = self.registry.insert_mcp(&entry);
                Err(e)
            }
        }
    }

    fn get(&self, name: &str) -> Result<McpEntry, VaultError> {
        self.registry.get_mcp(name)
    }

    fn list(&self) -> Result<Vec<McpEntry>, VaultError> {
        self.registry.list_mcps()
    }
}

fn clean_target_dir(path: &std::path::Path) -> Result<(), VaultError> {
    if let Ok(meta) = path.symlink_metadata() {
        if meta.is_dir() {
            if meta.file_type().is_symlink() {
                std::fs::remove_dir(path)?;
            } else {
                std::fs::remove_dir_all(path)?;
            }
        } else {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn resolve_npm_bin(
    install_path: &std::path::Path,
    package_name: &str,
) -> Result<(String, Vec<String>), VaultError> {
    let pkg_json_path = install_path
        .join("node_modules")
        .join(package_name)
        .join("package.json");

    if !pkg_json_path.exists() {
        return Err(VaultError::McpInstall {
            source_type: "npm".to_string(),
            message: format!("package.json not found at {}", pkg_json_path.display()),
        });
    }

    let file_content = std::fs::read_to_string(&pkg_json_path)?;
    let pkg: serde_json::Value = serde_json::from_str(&file_content)
        .map_err(|e| VaultError::Serialization(e.to_string()))?;

    let mut bin_path: Option<String> = None;
    if let Some(bin) = pkg.get("bin") {
        if let Some(bin_str) = bin.as_str() {
            bin_path = Some(bin_str.to_string());
        } else if let Some(bin_map) = bin.as_object() {
            let clean_pkg_name = package_name.split('/').next_back().unwrap_or(package_name);
            if let Some(val) = bin_map.get(package_name).and_then(|v| v.as_str()) {
                bin_path = Some(val.to_string());
            } else if let Some(val) = bin_map.get(clean_pkg_name).and_then(|v| v.as_str()) {
                bin_path = Some(val.to_string());
            } else if let Some((_, val)) = bin_map.iter().next() {
                if let Some(val_str) = val.as_str() {
                    bin_path = Some(val_str.to_string());
                }
            }
        }
    }

    let resolved_subpath = if let Some(bin_str) = bin_path {
        bin_str
    } else if let Some(main) = pkg.get("main").and_then(|m| m.as_str()) {
        main.to_string()
    } else {
        "index.js".to_string()
    };

    let script_path = install_path
        .join("node_modules")
        .join(package_name)
        .join(&resolved_subpath);

    let script_path = std::fs::canonicalize(&script_path).unwrap_or(script_path);

    if !script_path.exists() {
        return Err(VaultError::McpInstall {
            source_type: "npm".to_string(),
            message: format!(
                "Resolved script path does not exist: {}",
                script_path.display()
            ),
        });
    }

    let script_path_str = script_path.to_string_lossy().to_string();

    Ok(("node".to_string(), vec![script_path_str]))
}

async fn run_cmd<I, S>(cmd: &str, args: I, source_type: &str) -> Result<(), VaultError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    match tokio::process::Command::new(cmd).args(args).output().await {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            Err(VaultError::McpInstall {
                source_type: source_type.to_string(),
                message: format!(
                    "Command '{}' failed with status {}. stdout: {}, stderr: {}",
                    cmd, out.status, stdout, stderr
                ),
            })
        }
        Err(e) => Err(VaultError::McpInstall {
            source_type: source_type.to_string(),
            message: format!("Failed to execute command '{}': {}", cmd, e),
        }),
    }
}

fn resolve_pypi_cmd(venv_dir: &std::path::Path, package_name: &str) -> (String, Vec<String>) {
    let clean_name = package_name.replace('_', "-");
    let bin_dir = if cfg!(windows) {
        venv_dir.join("Scripts")
    } else {
        venv_dir.join("bin")
    };

    let possible_names = if cfg!(windows) {
        vec![
            clean_name.clone(),
            format!("{}.exe", clean_name),
            format!("{}.cmd", clean_name),
            format!("{}.bat", clean_name),
        ]
    } else {
        vec![clean_name.clone()]
    };

    for name in possible_names {
        let path = bin_dir.join(&name);
        if path.exists() && path.is_file() {
            return (path.to_string_lossy().to_string(), vec![]);
        }
    }

    let python_path = if cfg!(windows) {
        bin_dir.join("python.exe")
    } else {
        bin_dir.join("python")
    };

    (
        python_path.to_string_lossy().to_string(),
        vec!["-m".to_string(), package_name.to_string()],
    )
}
