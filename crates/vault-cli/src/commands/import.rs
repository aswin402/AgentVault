use crate::cli::ImportArgs;
use anyhow::{anyhow, Context, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::sync::Arc;
use tabled::builder::Builder;
use tabled::settings::Style;
use vault_connectors::claude::ClaudeConnector;
use vault_connectors::codex::CodexConnector;
use vault_connectors::gemini::GeminiConnector;
use vault_connectors::opencode::OpenCodeConnector;
use vault_connectors::sync::SyncEngine;
use vault_connectors::traits::AgentConnector;
use vault_core::agent::AgentType;
use vault_core::config::resolve_vault_dir;
use vault_core::manifest::VaultManifest;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::mcp::models::{McpEntry, McpSource};
use vault_core::registry::{Registry, SqliteRegistry};

pub async fn handle(args: ImportArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
    let vault_dir = resolve_vault_dir(vault_dir_override);
    let db_path = vault_dir.join("vault.db");

    if !db_path.exists() {
        println!(
            "{} Vault is not initialized. Run {} to start.",
            "Error:".bold().red(),
            "vault init".bold().yellow()
        );
        anyhow::bail!("Vault not initialized");
    }

    let registry =
        Arc::new(SqliteRegistry::new(&db_path).context("Failed to open registry database")?);
    let manager = DefaultMcpManager::new(registry.clone(), vault_dir.clone());

    println!(
        "{} manifest from {}...",
        "Reading".bold().green(),
        args.file.bold().cyan()
    );

    let content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read manifest file from {}", args.file))?;
    let manifest = VaultManifest::parse(&content).context("Failed to parse manifest file")?;

    // Compute diff
    let installed = registry.list_mcps().context("Failed to load MCPs")?;
    let mut installed_map: std::collections::HashMap<String, McpEntry> =
        installed.into_iter().map(|m| (m.name.clone(), m)).collect();

    let mut to_install = Vec::new();
    let mut to_update = Vec::new();
    let mut to_remove = Vec::new();

    for manifest_mcp in &manifest.mcp {
        if let Some(installed_mcp) = installed_map.remove(&manifest_mcp.name) {
            let source_matches = installed_mcp.source.to_string() == manifest_mcp.source;

            let version_matches = if manifest_mcp.version == "latest" {
                true
            } else if let Ok(req) = semver::VersionReq::parse(&manifest_mcp.version) {
                if let Ok(ver) = semver::Version::parse(&installed_mcp.version) {
                    req.matches(&ver)
                } else {
                    installed_mcp.version == manifest_mcp.version
                }
            } else {
                installed_mcp.version == manifest_mcp.version
            };

            let args_match = installed_mcp.args == manifest_mcp.args;
            let env_match = installed_mcp.env_vars == manifest_mcp.env;

            if !source_matches || !version_matches || !args_match || !env_match {
                to_update.push((installed_mcp, manifest_mcp.clone()));
            }
        } else {
            to_install.push(manifest_mcp.clone());
        }
    }

    if args.replace {
        for (_, mcp) in installed_map {
            to_remove.push(mcp);
        }
    }

    if args.dry_run {
        println!("\n{}", "=== Reconciliation Dry Run ===".bold().cyan());
        let mut builder = Builder::new();
        builder.push_record(["Action", "Name", "Source", "Version Constraint"]);

        for mcp in &to_install {
            builder.push_record([
                "Install".green().to_string(),
                mcp.name.clone(),
                mcp.source.clone(),
                mcp.version.clone(),
            ]);
        }
        for (old, mcp) in &to_update {
            builder.push_record([
                "Update".yellow().to_string(),
                mcp.name.clone(),
                mcp.source.clone(),
                format!("{} -> {}", old.version, mcp.version),
            ]);
        }
        for mcp in &to_remove {
            builder.push_record([
                "Remove (Prune)".red().to_string(),
                mcp.name.clone(),
                mcp.source.to_string(),
                mcp.version.clone(),
            ]);
        }

        let mut table = builder.build();
        table.with(Style::rounded());
        println!("{}", table);

        // Dry run sync agents
        let backup_dir = vault_dir.join("backups");
        let sync_engine = SyncEngine::new(registry.clone(), backup_dir.clone());

        for agent_str in &manifest.agents.sync {
            let agent_type: AgentType = agent_str.parse().map_err(|e: String| anyhow!(e))?;
            let registered = registry.get_agent_config(&agent_type.to_string()).ok();
            let config_path = registered.as_ref().map(|c| c.config_path.clone());

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
                    let path = config_path.unwrap_or_else(|| {
                        home.join(".gemini").join("config").join("settings.json")
                    });
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
                    let path =
                        config_path.unwrap_or_else(|| home.join(".codex").join("config.json"));
                    Some(Box::new(CodexConnector::new_with_paths(
                        path,
                        backup_dir.join("codex"),
                    )))
                }
                _ => None,
            };

            if let Some(connector) = connector {
                match sync_engine.dry_run(connector.as_ref()).await {
                    Ok(diff) => {
                        println!(
                            "\nDry run diff for sync agent {}:",
                            agent_type.to_string().bold()
                        );
                        if diff.is_empty() {
                            println!("  No changes.");
                        } else {
                            for add in &diff.additions {
                                println!("  + {} (add)", add.name.green());
                            }
                            for upd in &diff.updates {
                                println!("  ~ {} (update)", upd.name.yellow());
                            }
                            if args.replace {
                                for rem in &diff.removals {
                                    println!("  - {} (remove)", rem.name.red());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "{} Error during dry-run sync for {}: {}",
                            "✗".red(),
                            agent_type.to_string().bold(),
                            e
                        );
                    }
                }
            }
        }

        return Ok(());
    }

    // Apply changes
    if to_install.is_empty() && to_update.is_empty() && to_remove.is_empty() {
        println!("{}", "No changes required. Vault is up to date.".green());
    } else {
        // 1. Remove (Prune)
        for entry in to_remove {
            println!(
                "{} MCP server {}...",
                "Removing".bold().red(),
                entry.name.bold().cyan()
            );
            manager
                .remove(&entry.name, false)
                .await
                .context("Failed to remove pruned MCP server")?;
        }

        // 2. Update (Uninstall + Reinstall)
        for (old, entry) in to_update {
            println!(
                "{} MCP server {}...",
                "Updating".bold().yellow(),
                entry.name.bold().cyan()
            );
            manager
                .remove(&old.name, false)
                .await
                .context("Failed to uninstall MCP for update")?;

            let source: McpSource = entry
                .source
                .parse()
                .map_err(|e: String| anyhow!(e))
                .context("Invalid source format")?;
            manager
                .install(
                    &entry.name,
                    source,
                    &entry.version,
                    entry.args.clone(),
                    entry.env.clone(),
                    vec![],
                    vec![],
                    None,
                )
                .await
                .context("Failed to install updated MCP server")?;
        }

        // 3. Install
        for entry in to_install {
            println!(
                "{} MCP server {}...",
                "Installing".bold().green(),
                entry.name.bold().cyan()
            );
            let source: McpSource = entry
                .source
                .parse()
                .map_err(|e: String| anyhow!(e))
                .context("Invalid source format")?;
            manager
                .install(
                    &entry.name,
                    source,
                    &entry.version,
                    entry.args.clone(),
                    entry.env.clone(),
                    vec![],
                    vec![],
                    None,
                )
                .await
                .context("Failed to install new MCP server")?;
        }
    }

    // Sync agents
    let backup_dir = vault_dir.join("backups");
    let sync_engine = SyncEngine::new(registry.clone(), backup_dir.clone());

    for agent_str in &manifest.agents.sync {
        let agent_type: AgentType = agent_str.parse().map_err(|e: String| anyhow!(e))?;
        let registered = registry.get_agent_config(&agent_type.to_string()).ok();
        let config_path = registered.as_ref().map(|c| c.config_path.clone());

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
            println!(
                "Syncing capabilities to {}...",
                agent_type.to_string().bold()
            );
            match sync_engine
                .sync_agent(connector.as_ref(), args.replace)
                .await
            {
                Ok(result) => {
                    if result.success {
                        println!(
                            "{} Successfully synced {}",
                            "✓".green(),
                            agent_type.to_string().bold()
                        );
                    } else {
                        println!(
                            "{} Failed to sync {}: {:?}",
                            "✗".red(),
                            agent_type.to_string().bold(),
                            result.error
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "{} Error syncing {}: {}",
                        "✗".red(),
                        agent_type.to_string().bold(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ExportArgs, ExportFormat};
    use crate::commands::export;
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::tempdir;
    use vault_core::mcp::models::{McpEntry, McpSource, McpStatus, McpTransport};
    use vault_core::store::initialize_vault_directories;

    #[tokio::test]
    async fn test_import_roundtrip() {
        let temp_vault = tempdir().unwrap();
        let vault_dir = temp_vault.path().to_path_buf();
        initialize_vault_directories(&vault_dir).unwrap();

        // 1. Create registry and insert some dummy local MCPs (so we don't hit external npm/pypi downloads in tests)
        let db_path = vault_dir.join("vault.db");
        let registry = SqliteRegistry::new(&db_path).unwrap();

        let local_dir1 = tempdir().unwrap();
        let script_path1 = local_dir1.path().join("mcp1.sh");
        std::fs::write(&script_path1, "#!/bin/sh\necho 'mcp1'").unwrap();

        let local_dir2 = tempdir().unwrap();
        let script_path2 = local_dir2.path().join("mcp2.sh");
        std::fs::write(&script_path2, "#!/bin/sh\necho 'mcp2'").unwrap();

        let mcp1 = McpEntry {
            id: "mcp1".to_string(),
            name: "mcp1".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Local {
                path: local_dir1.path().to_path_buf(),
            },
            install_path: local_dir1.path().to_path_buf(),
            command: "sh".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
            agents: vec!["claude".to_string()],
            tags: vec![],
            description: None,
        };
        registry.insert_mcp(&mcp1).unwrap();

        // 2. Export the manifest
        let manifest_path = vault_dir.join("vault_export.toml");
        let export_args = ExportArgs {
            output: Some(manifest_path.to_str().unwrap().to_string()),
            format: ExportFormat::Toml,
        };
        export::handle(export_args, Some(vault_dir.to_str().unwrap()))
            .await
            .unwrap();

        // 3. Clear database or use a fresh vault directory to import
        let temp_vault_new = tempdir().unwrap();
        let vault_dir_new = temp_vault_new.path().to_path_buf();
        initialize_vault_directories(&vault_dir_new).unwrap();

        // Copy the export file to the new vault folder for ease
        let new_manifest_path = vault_dir_new.join("vault_import.toml");
        std::fs::copy(&manifest_path, &new_manifest_path).unwrap();

        // Initialize the new database file so that it exists
        let new_db_path = vault_dir_new.join("vault.db");
        let _new_registry = SqliteRegistry::new(&new_db_path).unwrap();

        // Import the manifest
        let import_args = ImportArgs {
            file: new_manifest_path.to_str().unwrap().to_string(),
            dry_run: false,
            merge: true,
            replace: false,
        };
        handle(import_args, Some(vault_dir_new.to_str().unwrap()))
            .await
            .unwrap();

        // Verify the MCP is installed in the new registry
        let new_registry = SqliteRegistry::new(&new_db_path).unwrap();
        let list = new_registry.list_mcps().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "mcp1");
    }

    #[tokio::test]
    async fn test_import_replace_prunes_unlisted() {
        let temp_vault = tempdir().unwrap();
        let vault_dir = temp_vault.path().to_path_buf();
        initialize_vault_directories(&vault_dir).unwrap();

        let db_path = vault_dir.join("vault.db");
        let registry = SqliteRegistry::new(&db_path).unwrap();

        let local_dir1 = tempdir().unwrap();
        let script_path1 = local_dir1.path().join("mcp1.sh");
        std::fs::write(&script_path1, "#!/bin/sh\necho 'mcp1'").unwrap();

        let local_dir2 = tempdir().unwrap();
        let script_path2 = local_dir2.path().join("mcp2.sh");
        std::fs::write(&script_path2, "#!/bin/sh\necho 'mcp2'").unwrap();

        let mcp1 = McpEntry {
            id: "mcp1".to_string(),
            name: "mcp1".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Local {
                path: local_dir1.path().to_path_buf(),
            },
            install_path: local_dir1.path().to_path_buf(),
            command: "sh".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec![],
            description: None,
        };
        let mcp2 = McpEntry {
            id: "mcp2".to_string(),
            name: "mcp2".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Local {
                path: local_dir2.path().to_path_buf(),
            },
            install_path: local_dir2.path().to_path_buf(),
            command: "sh".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec![],
            description: None,
        };
        registry.insert_mcp(&mcp1).unwrap();
        registry.insert_mcp(&mcp2).unwrap();

        // Create a manifest with only mcp1
        let manifest_content = format!(
            r#"
            [vault]
            name = "test-vault"
            version = "1.0.0"

            [[mcp]]
            name = "mcp1"
            source = "local:{}"
            version = "1.0.0"

            [agents]
            sync = []
            "#,
            local_dir1.path().to_str().unwrap()
        );
        let manifest_path = vault_dir.join("vault.toml");
        std::fs::write(&manifest_path, manifest_content).unwrap();

        // Dry run first
        let import_args_dry = ImportArgs {
            file: manifest_path.to_str().unwrap().to_string(),
            dry_run: true,
            merge: false,
            replace: true,
        };
        handle(import_args_dry, Some(vault_dir.to_str().unwrap()))
            .await
            .unwrap();

        // Verify mcp2 is still there after dry run
        let list_before = registry.list_mcps().unwrap();
        assert_eq!(list_before.len(), 2);

        // Run with replace=true (which prunes)
        let import_args = ImportArgs {
            file: manifest_path.to_str().unwrap().to_string(),
            dry_run: false,
            merge: false,
            replace: true,
        };
        handle(import_args, Some(vault_dir.to_str().unwrap()))
            .await
            .unwrap();

        // Verify mcp2 is gone and only mcp1 remains
        let list_after = registry.list_mcps().unwrap();
        assert_eq!(list_after.len(), 1);
        assert_eq!(list_after[0].name, "mcp1");
    }

    #[tokio::test]
    async fn test_import_idempotent() {
        let temp_vault = tempdir().unwrap();
        let vault_dir = temp_vault.path().to_path_buf();
        initialize_vault_directories(&vault_dir).unwrap();

        let db_path = vault_dir.join("vault.db");
        let registry = SqliteRegistry::new(&db_path).unwrap();

        let local_dir1 = tempdir().unwrap();
        let script_path1 = local_dir1.path().join("mcp1.sh");
        std::fs::write(&script_path1, "#!/bin/sh\necho 'mcp1'").unwrap();

        let manifest_content = format!(
            r#"
            [vault]
            name = "test-vault"
            version = "1.0.0"

            [[mcp]]
            name = "mcp1"
            source = "local:{}"
            version = "1.0.0"

            [agents]
            sync = []
            "#,
            local_dir1.path().to_str().unwrap()
        );
        let manifest_path = vault_dir.join("vault.toml");
        std::fs::write(&manifest_path, manifest_content).unwrap();

        // 1st import
        let import_args = ImportArgs {
            file: manifest_path.to_str().unwrap().to_string(),
            dry_run: false,
            merge: true,
            replace: false,
        };
        handle(import_args.clone(), Some(vault_dir.to_str().unwrap()))
            .await
            .unwrap();

        let list_1 = registry.list_mcps().unwrap();
        assert_eq!(list_1.len(), 1);
        let installed_time_1 = list_1[0].installed_at;

        // 2nd import
        handle(import_args, Some(vault_dir.to_str().unwrap()))
            .await
            .unwrap();

        let list_2 = registry.list_mcps().unwrap();
        assert_eq!(list_2.len(), 1);
        assert_eq!(list_2[0].installed_at, installed_time_1);
    }
}
