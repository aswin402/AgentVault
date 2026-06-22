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
        _version_req: &str,
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
            if target_link.symlink_metadata().is_ok() {
                let meta = target_link.symlink_metadata()?;
                if meta.is_dir() {
                    if meta.file_type().is_symlink() {
                        std::fs::remove_dir(&target_link)?;
                    } else {
                        std::fs::remove_dir_all(&target_link)?;
                    }
                } else {
                    std::fs::remove_file(&target_link)?;
                }
            }

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
