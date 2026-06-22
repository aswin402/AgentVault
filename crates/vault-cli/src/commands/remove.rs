use crate::cli::RemoveArgs;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::sync::Arc;
use vault_core::config::resolve_vault_dir;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::registry::SqliteRegistry;

pub async fn handle(args: RemoveArgs, vault_dir_override: Option<&str>) -> Result<()> {
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

    let registry = Arc::new(SqliteRegistry::new(&db_path).context("Failed to open registry database")?);
    let manager = DefaultMcpManager::new(registry, vault_dir);

    // Verify it exists in registry
    if manager.get(&args.name).is_err() {
        println!(
            "{} MCP server {} not found in registry.",
            "Error:".bold().red(),
            args.name.bold().cyan()
        );
        anyhow::bail!("MCP server not found");
    }

    println!(
        "{} MCP server {}...",
        "Removing".bold().green(),
        args.name.bold().cyan()
    );

    manager.remove(&args.name, args.keep_files).await?;

    println!(
        "{} MCP server {} successfully removed.",
        "Success".bold().green(),
        args.name.bold().cyan()
    );

    Ok(())
}
