use crate::cli::SyncArgs;
use anyhow::{anyhow, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::sync::Arc;
use vault_connectors::claude::ClaudeConnector;
use vault_connectors::codex::CodexConnector;
use vault_connectors::gemini::GeminiConnector;
use vault_connectors::opencode::OpenCodeConnector;
use vault_connectors::sync::SyncEngine;
use vault_connectors::traits::AgentConnector;
use vault_core::agent::AgentType;
use vault_core::registry::{Registry, SqliteRegistry};

pub async fn handle(args: SyncArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
    let vault_dir = vault_dir_override
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".agentvault"));

    let db_path = vault_dir.join("vault.db");
    if !db_path.exists() {
        return Err(anyhow!("Vault is not initialized. Run `vault init` first."));
    }

    let registry = Arc::new(SqliteRegistry::new(&db_path)?);
    let backup_dir = vault_dir.join("backups");
    let engine = SyncEngine::new(registry.clone(), backup_dir.clone());

    let target_agents = if args.all {
        let registered = registry.list_agent_configs()?;
        if registered.is_empty() {
            println!(
                "{}",
                "No agents registered. Add one using `vault connector add <agent>`.".yellow()
            );
            return Ok(());
        }
        registered
            .into_iter()
            .filter(|c| c.enabled)
            .map(|c| c.agent_type)
            .collect::<Vec<_>>()
    } else if let Some(ref agent_str) = args.agent {
        let agent_type: AgentType = agent_str.parse().map_err(|e: String| anyhow!(e))?;
        vec![agent_type]
    } else {
        return Err(anyhow!("Must specify an agent to sync or use --all"));
    };

    for agent_type in target_agents {
        // Look up registered config for this agent
        let registered = registry.get_agent_config(&agent_type.to_string()).ok();
        let config_path = registered.as_ref().map(|c| c.config_path.clone());

        // Skip if disabled (if explicitly targeting an agent, we can warn, but if --all we already filtered)
        if let Some(ref reg) = registered {
            if !reg.enabled {
                println!(
                    "{}",
                    format!("Skipping disabled agent: {}", agent_type).yellow()
                );
                continue;
            }
        }

        let connector: Option<Box<dyn AgentConnector>> = match agent_type {
            AgentType::ClaudeCode => {
                let path = config_path
                    .unwrap_or_else(|| home.join(".claude").join("claude_desktop_config.json"));
                Some(Box::new(ClaudeConnector::new_with_paths(
                    path,
                    backup_dir.join("claude"),
                )))
            }
            AgentType::GeminiCli => {
                let path = config_path
                    .unwrap_or_else(|| home.join(".gemini").join("config").join("settings.json"));
                Some(Box::new(GeminiConnector::new_with_paths(
                    path,
                    backup_dir.join("gemini"),
                )))
            }
            AgentType::OpenCode => {
                let path = config_path.unwrap_or_else(|| {
                    let config_dir = std::env::var("XDG_CONFIG_HOME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|_| home.join(".config"));
                    config_dir.join("opencode").join("config.json")
                });
                Some(Box::new(OpenCodeConnector::new_with_paths(
                    path,
                    backup_dir.join("opencode"),
                )))
            }
            AgentType::CodexCli => {
                let path = config_path.unwrap_or_else(|| home.join(".codex").join("config.json"));
                Some(Box::new(CodexConnector::new_with_paths(
                    path,
                    backup_dir.join("codex"),
                )))
            }
            _ => None,
        };

        if let Some(connector) = connector {
            let res = if args.dry_run {
                match engine.dry_run(connector.as_ref()).await {
                    Ok(diff) => {
                        println!("Dry run diff for {}:", agent_type.to_string().bold());
                        if diff.is_empty() {
                            println!("  No changes.");
                        } else {
                            for add in &diff.additions {
                                println!("  + {} (add)", add.name.green());
                            }
                            for upd in &diff.updates {
                                println!("  ~ {} (update)", upd.name.yellow());
                            }
                            if args.prune {
                                for rem in &diff.removals {
                                    println!("  - {} (remove)", rem.name.red());
                                }
                            }
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                println!("Syncing to {}...", agent_type.to_string().bold());
                match engine.sync_agent(connector.as_ref(), args.prune).await {
                    Ok(result) => {
                        if result.success {
                            println!(
                                "{} Successfully synced {}",
                                "✓".green(),
                                agent_type.to_string().bold()
                            );
                            if let Some(ref backup) = result.backup_path {
                                println!("  Backup created: {}", backup);
                            }
                        } else {
                            println!(
                                "{} Failed to sync {}: {:?}",
                                "✗".red(),
                                agent_type.to_string().bold(),
                                result.error
                            );
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            };
            if let Err(e) = res {
                eprintln!(
                    "{} Error syncing agent {}: {}",
                    "✗".red(),
                    agent_type.to_string().bold(),
                    e
                );
            }
        } else {
            println!("{} Unknown or unsupported agent: {}", "✗".red(), agent_type);
        }
    }

    Ok(())
}
