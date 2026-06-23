use crate::cli::{ConnectorArgs, ConnectorCommands};
use anyhow::{anyhow, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::sync::Arc;
use tabled::{Table, Tabled};
use vault_core::agent::{AgentConnectorConfig, AgentType};
use vault_core::registry::{Registry, SqliteRegistry};

#[derive(Tabled)]
struct ConnectorTableEntry {
    #[tabled(rename = "Agent Type")]
    agent_type: String,
    #[tabled(rename = "Config Path")]
    config_path: String,
    #[tabled(rename = "Enabled")]
    enabled: bool,
    #[tabled(rename = "Auto Sync")]
    auto_sync: bool,
    #[tabled(rename = "Last Synced")]
    last_synced: String,
}

pub async fn handle(args: ConnectorArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
    let vault_dir = vault_dir_override
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".agentvault"));

    let db_path = vault_dir.join("vault.db");
    if !db_path.exists() {
        return Err(anyhow!("Vault is not initialized. Run `vault init` first."));
    }

    let registry = Arc::new(SqliteRegistry::new(&db_path)?);

    match args.command {
        ConnectorCommands::Add(subargs) => {
            let agent_type: AgentType =
                subargs.agent_type.parse().map_err(|e: String| anyhow!(e))?;

            let config_path = if let Some(path) = subargs.config_path {
                PathBuf::from(path)
            } else {
                match agent_type {
                    AgentType::ClaudeCode => home.join(".claude").join("claude_desktop_config.json"),
                    AgentType::GeminiCli => home.join(".gemini").join("config").join("settings.json"),
                    AgentType::OpenCode => {
                        let config_dir = std::env::var("XDG_CONFIG_HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|_| home.join(".config"));
                        config_dir.join("opencode").join("config.json")
                    }
                    AgentType::CodexCli => home.join(".codex").join("config.json"),
                    _ => {
                        return Err(anyhow!(
                            "Cannot resolve default config path for custom agent, please provide --config-path"
                        ))
                    }
                }
            };

            let config = AgentConnectorConfig {
                id: uuid::Uuid::new_v4().to_string(),
                agent_type,
                config_path,
                enabled: true,
                last_synced: None,
                auto_sync: subargs.auto_sync,
            };

            registry.insert_agent_config(&config)?;
            println!(
                "{} Successfully added agent connector: {}",
                "✓".green(),
                subargs.agent_type.bold()
            );
        }
        ConnectorCommands::List(subargs) => {
            let configs = registry.list_agent_configs()?;
            if subargs.json {
                println!("{}", serde_json::to_string_pretty(&configs)?);
            } else if configs.is_empty() {
                println!("No agent connectors registered. Add one with `vault connector add`.");
            } else {
                let table_entries: Vec<ConnectorTableEntry> = configs
                    .into_iter()
                    .map(|c| ConnectorTableEntry {
                        agent_type: c.agent_type.to_string(),
                        config_path: c.config_path.to_string_lossy().to_string(),
                        enabled: c.enabled,
                        auto_sync: c.auto_sync,
                        last_synced: c
                            .last_synced
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_else(|| "Never".to_string()),
                    })
                    .collect();
                println!("{}", Table::new(table_entries));
            }
        }
        ConnectorCommands::Remove(subargs) => {
            registry.delete_agent_config(&subargs.agent_type)?;
            println!(
                "{} Successfully removed agent connector: {}",
                "✓".green(),
                subargs.agent_type.bold()
            );
        }
    }
    Ok(())
}
