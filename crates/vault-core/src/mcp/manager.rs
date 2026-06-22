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
            match std::process::Command::new(npm_cmd)
                .arg("--version")
                .output()
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
            let output = std::process::Command::new(npm_cmd)
                .arg("install")
                .arg("--prefix")
                .arg(&target_dir)
                .arg(&package_spec)
                .output();

            match output {
                Ok(out) if out.status.success() => {}
                Ok(out) => {
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

        Err(VaultError::NotFound {
            kind: "mcp".to_string(),
            name: name.to_string(),
        })
    }

    async fn remove(&self, _name: &str, _keep_files: bool) -> Result<(), VaultError> {
        Err(VaultError::NotFound {
            kind: "mcp".to_string(),
            name: _name.to_string(),
        })
    }

    async fn update(&self, _name: &str, _force: bool) -> Result<McpEntry, VaultError> {
        Err(VaultError::NotFound {
            kind: "mcp".to_string(),
            name: _name.to_string(),
        })
    }

    fn get(&self, name: &str) -> Result<McpEntry, VaultError> {
        self.registry.get_mcp(name)
    }

    fn list(&self) -> Result<Vec<McpEntry>, VaultError> {
        self.registry.list_mcps()
    }
}

fn clean_target_dir(path: &std::path::Path) -> Result<(), VaultError> {
    if path.symlink_metadata().is_ok() {
        let meta = path.symlink_metadata()?;
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
    let script_path_str = script_path.to_string_lossy().to_string();

    Ok(("node".to_string(), vec![script_path_str]))
}
