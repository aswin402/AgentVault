use crate::traits::AgentConnector;
use crate::types::{SyncDiff, SyncResult};
use std::path::PathBuf;
use std::sync::Arc;
use vault_core::agent::SyncHistoryEntry;
use vault_core::error::VaultError;
use vault_core::mcp::models::McpEntry;
use vault_core::registry::Registry;

pub struct SyncEngine {
    registry: Arc<dyn Registry>,
    _backup_dir: PathBuf,
}

impl SyncEngine {
    pub fn new(registry: Arc<dyn Registry>, backup_dir: PathBuf) -> Self {
        Self { registry, _backup_dir: backup_dir }
    }

    pub async fn sync_agent(&self, connector: &dyn AgentConnector, prune: bool) -> Result<SyncResult, VaultError> {
        let all_mcps = self.registry.list_mcps()?;
        let agent_str = connector.agent_type().to_string();
        let filtered_mcps: Vec<McpEntry> = all_mcps
            .into_iter()
            .filter(|mcp| mcp.agents.contains(&agent_str))
            .collect();

        let mut diff = connector.diff(&filtered_mcps).await?;
        if !prune {
            diff.removals.clear();
        }

        // To support !prune, since connector.sync() internally recalculates diff and removes them:
        // if prune is false, we can read the existing configuration first.
        // Then, we can construct dummy McpEntry items for the servers that are in the connector's
        // configuration but not in `filtered_mcps`, so they are not pruned by the connector.
        let mut entries_to_sync = filtered_mcps;
        if !prune {
            if let Ok(config) = connector.read_config().await {
                let existing_names: std::collections::HashSet<String> = entries_to_sync
                    .iter()
                    .map(|e| e.name.clone())
                    .collect();
                for (name, server_config) in config.mcp_servers {
                    if !existing_names.contains(&name) {
                        // Create a dummy McpEntry to preserve this server
                        entries_to_sync.push(McpEntry {
                            id: "".to_string(),
                            name: name.clone(),
                            display_name: None,
                            version: "".to_string(),
                            source: vault_core::mcp::models::McpSource::Local {
                                path: PathBuf::new(),
                            },
                            install_path: PathBuf::new(),
                            command: server_config.command,
                            args: server_config.args,
                            env_vars: server_config.env,
                            transport: vault_core::mcp::models::McpTransport::Stdio,
                            status: vault_core::mcp::models::McpStatus::Active,
                            installed_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            checksum: None,
                            agents: vec![agent_str.clone()],
                            tags: vec![],
                            description: None,
                        });
                    }
                }
            }
        }

        let mut result = connector.sync(&entries_to_sync).await?;
        result.diff = diff; 

        let history_entry = SyncHistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            agent_type: agent_str.clone(),
            action: "sync".to_string(),
            diff_json: serde_json::to_string(&result.diff).unwrap_or_default(),
            synced_at: result.timestamp,
            success: result.success,
            error: result.error.clone(),
        };
        self.registry.log_sync(&history_entry)?;

        if result.success {
            if let Ok(mut config) = self.registry.get_agent_config(&agent_str) {
                config.last_synced = Some(result.timestamp);
                let _ = self.registry.update_agent_config(&config);
            }
        }

        Ok(result)
    }

    pub async fn dry_run(&self, connector: &dyn AgentConnector) -> Result<SyncDiff, VaultError> {
        let all_mcps = self.registry.list_mcps()?;
        let agent_str = connector.agent_type().to_string();
        let filtered_mcps: Vec<McpEntry> = all_mcps
            .into_iter()
            .filter(|mcp| mcp.agents.contains(&agent_str))
            .collect();

        connector.diff(&filtered_mcps).await
    }
}
