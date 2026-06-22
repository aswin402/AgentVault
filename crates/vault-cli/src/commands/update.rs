use crate::cli::UpdateArgs;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::sync::Arc;
use vault_core::config::resolve_vault_dir;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::registry::{Registry, SqliteRegistry};

pub async fn handle(args: UpdateArgs, vault_dir_override: Option<&str>) -> Result<()> {
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
    let manager = DefaultMcpManager::new(registry.clone(), vault_dir);

    let mut names = Vec::new();
    if args.all {
        let mcps = registry.list_mcps().context("Failed to load MCPs")?;
        for mcp in mcps {
            names.push(mcp.name);
        }
    } else if let Some(ref name) = args.name {
        names.push(name.clone());
    } else {
        println!(
            "{} Must specify a name to update or use the {} flag.",
            "Error:".bold().red(),
            "--all".bold().yellow()
        );
        anyhow::bail!("No target specified for update");
    }

    if names.is_empty() {
        println!("{}", "No capabilities to update.".yellow());
        return Ok(());
    }

    if args.dry_run {
        println!(
            "{}",
            "Dry run enabled. The following updates would be performed:"
                .bold()
                .yellow()
        );
        for name in &names {
            println!("  • Update MCP server: {}", name.cyan());
        }
        return Ok(());
    }

    for name in names {
        println!(
            "{} MCP server {}...",
            "Updating".bold().green(),
            name.bold().cyan()
        );

        match manager.update(&name, args.force).await {
            Ok(entry) => {
                println!(
                    "{} MCP server {} successfully updated! (version: {})",
                    "Success".bold().green(),
                    entry.name.bold().cyan(),
                    entry.version.green()
                );
            }
            Err(e) => {
                println!(
                    "{} Failed to update {}: {}",
                    "Error:".bold().red(),
                    name.bold().cyan(),
                    e.to_string().red()
                );
            }
        }
    }

    Ok(())
}
