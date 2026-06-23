use crate::traits::AgentConnector;
use crate::types::{
    AgentConfig, AgentMcpConfig, FieldChange, SyncDiff, SyncEntry, SyncResult, SyncUpdate,
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use vault_core::agent::AgentType;
use vault_core::error::VaultError;
use vault_core::mcp::models::McpEntry;

pub struct OpenCodeConnector {
    config_path: PathBuf,
    backup_dir: PathBuf,
}

impl Default for OpenCodeConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenCodeConnector {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                if cfg!(target_os = "windows") {
                    dirs::config_dir().unwrap_or_else(|| home.join("AppData").join("Roaming"))
                } else {
                    home.join(".config")
                }
            });
        Self {
            config_path: config_dir.join("opencode").join("config.json"),
            backup_dir: home.join(".agentvault").join("backups").join("opencode"),
        }
    }

    pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_path,
            backup_dir,
        }
    }

    fn mcp_to_agent_config(entry: &McpEntry) -> AgentMcpConfig {
        AgentMcpConfig {
            command: entry.command.clone(),
            args: entry.args.clone(),
            env: entry.env_vars.clone(),
        }
    }
}

#[async_trait]
impl AgentConnector for OpenCodeConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::OpenCode
    }

    fn config_path(&self) -> &Path {
        &self.config_path
    }

    async fn read_config(&self) -> Result<AgentConfig, VaultError> {
        if !self.config_path.exists() {
            return Ok(AgentConfig {
                raw: serde_json::json!({}),
                mcp_servers: HashMap::new(),
            });
        }

        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(VaultError::Io)?;

        let raw: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| VaultError::Config {
                message: format!("Invalid JSON: {}", e),
            })?;

        let mcp_servers = raw
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(name, value)| {
                        serde_json::from_value::<AgentMcpConfig>(value.clone())
                            .ok()
                            .map(|config| (name.clone(), config))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(AgentConfig { raw, mcp_servers })
    }

    async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError> {
        let mut raw = config.raw.clone();
        let mcp_obj = serde_json::to_value(&config.mcp_servers)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        raw["mcpServers"] = mcp_obj;

        let temp_path = self.config_path.with_extension("vault-tmp");
        let content = serde_json::to_string_pretty(&raw)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(VaultError::Io)?;
        }

        tokio::fs::write(&temp_path, &content)
            .await
            .map_err(VaultError::Io)?;

        tokio::fs::rename(&temp_path, &self.config_path)
            .await
            .map_err(VaultError::Io)?;

        Ok(())
    }

    async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError> {
        let config = self.read_config().await?;
        let mut diff = SyncDiff::default();

        // Additions & Updates
        for entry in entries {
            match config.mcp_servers.get(&entry.name) {
                None => {
                    diff.additions.push(SyncEntry {
                        name: entry.name.clone(),
                        source: entry.source.to_string(),
                        version: entry.version.clone(),
                    });
                }
                Some(existing) => {
                    let mut changes = Vec::new();
                    if existing.command != entry.command {
                        changes.push(FieldChange {
                            field: "command".to_string(),
                            old_value: existing.command.clone(),
                            new_value: entry.command.clone(),
                        });
                    }
                    if existing.args != entry.args {
                        changes.push(FieldChange {
                            field: "args".to_string(),
                            old_value: format!("{:?}", existing.args),
                            new_value: format!("{:?}", entry.args),
                        });
                    }
                    if existing.env != entry.env_vars {
                        changes.push(FieldChange {
                            field: "env".to_string(),
                            old_value: format!("{:?}", existing.env),
                            new_value: format!("{:?}", entry.env_vars),
                        });
                    }
                    if !changes.is_empty() {
                        diff.updates.push(SyncUpdate {
                            name: entry.name.clone(),
                            changed_fields: changes,
                        });
                    }
                }
            }
        }

        // Removals (vault-managed entries in agent but not in registry)
        let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        for name in config.mcp_servers.keys() {
            if !vault_names.contains(name.as_str()) {
                diff.removals.push(SyncEntry {
                    name: name.clone(),
                    source: "vault-managed".to_string(),
                    version: "".to_string(),
                });
            }
        }

        Ok(diff)
    }

    async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError> {
        let diff = self.diff(entries).await?;
        let timestamp = chrono::Utc::now();

        if diff.is_empty() {
            return Ok(SyncResult {
                agent_type: self.agent_type().to_string(),
                timestamp,
                diff,
                success: true,
                backup_path: None,
                error: None,
            });
        }

        let backup_path = if self.config_path.exists() {
            Some(self.backup()?)
        } else {
            None
        };

        let mut config = self.read_config().await?;

        // Apply additions and updates
        for entry in entries {
            config
                .mcp_servers
                .insert(entry.name.clone(), Self::mcp_to_agent_config(entry));
        }

        // Apply removals
        for removal in &diff.removals {
            config.mcp_servers.remove(&removal.name);
        }

        self.write_config(&config).await?;

        let valid = self.verify()?;
        if !valid {
            if let Some(ref bp) = backup_path {
                std::fs::copy(bp, &self.config_path).map_err(VaultError::Io)?;
            }
            return Err(VaultError::McpInstall {
                source_type: self.agent_type().to_string(),
                message: "Sync verification failed after writing config".to_string(),
            });
        }

        Ok(SyncResult {
            agent_type: self.agent_type().to_string(),
            timestamp,
            diff,
            success: true,
            backup_path: backup_path.map(|p| p.display().to_string()),
            error: None,
        })
    }

    fn backup(&self) -> Result<PathBuf, VaultError> {
        if !self.config_path.exists() {
            return Ok(PathBuf::new());
        }

        std::fs::create_dir_all(&self.backup_dir).map_err(VaultError::Io)?;

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
        let backup_path = self.backup_dir.join(format!("{}.json", timestamp));

        std::fs::copy(&self.config_path, &backup_path).map_err(VaultError::Io)?;

        Ok(backup_path)
    }

    fn verify(&self) -> Result<bool, VaultError> {
        if !self.config_path.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&self.config_path).map_err(VaultError::Io)?;

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => Ok(v.is_object()),
            Err(_) => Ok(false),
        }
    }
}
