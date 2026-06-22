use crate::types::{AgentConfig, SyncDiff, SyncResult};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use vault_core::agent::AgentType;
use vault_core::error::VaultError;
use vault_core::mcp::models::McpEntry;

#[async_trait]
pub trait AgentConnector: Send + Sync {
    fn agent_type(&self) -> AgentType;
    fn config_path(&self) -> &Path;
    async fn read_config(&self) -> Result<AgentConfig, VaultError>;
    async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError>;
    async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError>;
    async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError>;
    fn backup(&self) -> Result<PathBuf, VaultError>;
    fn verify(&self) -> Result<bool, VaultError>;
}
