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
        _name: &str,
        _source: McpSource,
        _version_req: &str,
        _args: Vec<String>,
        _env_vars: std::collections::HashMap<String, String>,
        _agents: Vec<String>,
        _tags: Vec<String>,
        _description: Option<String>,
    ) -> Result<McpEntry, VaultError> {
        Err(VaultError::NotFound {
            kind: "mcp".to_string(),
            name: _name.to_string(),
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
