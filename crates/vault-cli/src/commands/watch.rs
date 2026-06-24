use crate::cli::WatchArgs;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use vault_connectors::claude::ClaudeConnector;
use vault_connectors::codex::CodexConnector;
use vault_connectors::gemini::GeminiConnector;
use vault_connectors::opencode::OpenCodeConnector;
use vault_connectors::traits::AgentConnector;
use vault_core::agent::AgentType;
use vault_core::config::resolve_vault_dir;
use vault_core::registry::{Registry, SqliteRegistry};
use vault_core::watcher::ConfigWatcher;

pub async fn handle(args: WatchArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let vault_dir = resolve_vault_dir(vault_dir_override);
    let db_path = vault_dir.join("vault.db");

    if !db_path.exists() {
        return Err(anyhow!("Vault is not initialized. Run `vault init` first."));
    }

    if args.daemon {
        return run_daemon(&vault_dir, vault_dir_override);
    }

    println!("Starting AgentVault File Watcher in foreground...");
    let registry = Arc::new(SqliteRegistry::new(&db_path)?);
    let mut watcher = ConfigWatcher::new()?;

    // Gather active agent configs and watch them
    let configs = registry.list_agent_configs()?;
    let mut watched_paths = Vec::new();

    for cfg in &configs {
        if cfg.enabled {
            let path = cfg.config_path.clone();
            if path.exists() {
                watcher.watch(path.clone())?;
                watched_paths.push((cfg.agent_type.clone(), path));
            }
        }
    }

    if watched_paths.is_empty() {
        println!("No active agent configuration paths found to watch.");
        return Ok(());
    }

    for (agent_type, path) in &watched_paths {
        println!("Watching {} configuration: {}", agent_type, path.display());
    }

    let backup_dir = vault_dir.join("backups");
    let sync_engine = vault_connectors::sync::SyncEngine::new(registry.clone(), backup_dir.clone());

    // Event loop with debouncing
    loop {
        if let Some(Ok(event)) = watcher.next_event().await {
            if event.kind.is_modify() {
                // Sleep briefly to debounce double-writes (common in editors/agents writing files)
                sleep(Duration::from_millis(250)).await;

                // Clear any queued extra events during the debounce window
                while let Ok(Some(_)) =
                    tokio::time::timeout(Duration::from_millis(10), watcher.next_event()).await
                {
                }

                println!("Modification detected. Re-synchronizing configurations...");
                for (agent_type, path) in &watched_paths {
                    let connector: Option<Box<dyn AgentConnector>> = match agent_type {
                        AgentType::ClaudeCode => Some(Box::new(ClaudeConnector::new_with_paths(
                            path.clone(),
                            backup_dir.join("claude"),
                        ))),
                        AgentType::GeminiCli => Some(Box::new(GeminiConnector::new_with_paths(
                            path.clone(),
                            backup_dir.join("gemini"),
                        ))),
                        AgentType::OpenCode => Some(Box::new(OpenCodeConnector::new_with_paths(
                            path.clone(),
                            backup_dir.join("opencode"),
                        ))),
                        AgentType::CodexCli => Some(Box::new(CodexConnector::new_with_paths(
                            path.clone(),
                            backup_dir.join("codex"),
                        ))),
                        _ => None,
                    };

                    if let Some(conn) = connector {
                        match sync_engine.sync_agent(conn.as_ref(), true).await {
                            Ok(res) => {
                                if res.success {
                                    println!("✓ Re-synchronized configuration for {}", agent_type);
                                } else if let Some(err) = res.error {
                                    eprintln!("✗ Failed to sync {}: {}", agent_type, err);
                                }
                            }
                            Err(e) => eprintln!("✗ Sync error for {}: {}", agent_type, e),
                        }
                    }
                }
            }
        }
    }
}

fn run_daemon(vault_dir: &Path, vault_dir_override: Option<&str>) -> Result<()> {
    let log_dir = vault_dir.join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("watcher.log");

    println!(
        "Spawning background watcher daemon. Logs: {}",
        log_path.display()
    );

    let current_exe = std::env::current_exe().context("Failed to find current executable path")?;
    let mut cmd = std::process::Command::new(current_exe);
    cmd.arg("watch");

    if let Some(over) = vault_dir_override {
        cmd.arg("--vault-dir").arg(over);
    }

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .context("Failed to open watcher daemon log file")?;

    cmd.stdout(log_file.try_clone()?);
    cmd.stderr(log_file);
    cmd.stdin(std::process::Stdio::null());

    cmd.spawn().context("Failed to spawn background process")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_watch_fails_when_uninitialized() {
        let dir = tempdir().unwrap();
        let path_str = dir.path().to_str().unwrap();
        let args = WatchArgs { daemon: false };
        let res = handle(args, Some(path_str)).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Vault is not initialized"));
    }

    #[tokio::test]
    async fn test_watch_exits_early_when_no_watched_paths() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("vault.db");
        {
            let _registry = SqliteRegistry::new(&db_path).unwrap();
        }
        let path_str = dir.path().to_str().unwrap();
        let args = WatchArgs { daemon: false };
        let res = handle(args, Some(path_str)).await;
        assert!(res.is_ok());
    }
}
