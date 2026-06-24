use crate::cli::RemoveArgs;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::sync::Arc;
use vault_core::config::resolve_vault_dir;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::registry::SqliteRegistry;

use vault_core::registry::Registry;
use vault_core::skill::manager::SkillManager;
use vault_core::workflow::manager::WorkflowManager;

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

    let registry =
        Arc::new(SqliteRegistry::new(&db_path).context("Failed to open registry database")?);

    // 1. Try matching Workflow
    if registry.get_workflow(&args.name).is_ok() {
        println!(
            "{} Workflow {}...",
            "Removing".bold().green(),
            args.name.bold().cyan()
        );
        let workflow_manager =
            vault_core::workflow::manager::DefaultWorkflowManager::new(registry, vault_dir);
        workflow_manager.remove(&args.name).await?;
        println!(
            "{} Workflow {} successfully removed.",
            "Success".bold().green(),
            args.name.bold().cyan()
        );
        return Ok(());
    }

    // 2. Try matching Skill
    if registry.get_skill(&args.name).is_ok() {
        println!(
            "{} Skill {}...",
            "Removing".bold().green(),
            args.name.bold().cyan()
        );
        let skill_manager =
            vault_core::skill::manager::DefaultSkillManager::new(registry, vault_dir);
        skill_manager.remove(&args.name).await?;
        println!(
            "{} Skill {} successfully removed.",
            "Success".bold().green(),
            args.name.bold().cyan()
        );
        return Ok(());
    }

    // 3. Try matching MCP
    if registry.get_mcp(&args.name).is_ok() {
        println!(
            "{} MCP server {}...",
            "Removing".bold().green(),
            args.name.bold().cyan()
        );
        let manager = DefaultMcpManager::new(registry, vault_dir);
        manager.remove(&args.name, args.keep_files).await?;
        println!(
            "{} MCP server {} successfully removed.",
            "Success".bold().green(),
            args.name.bold().cyan()
        );
        return Ok(());
    }

    println!(
        "{} Capability '{}' not found in vault registry.",
        "Error:".bold().red(),
        args.name.bold().cyan()
    );
    anyhow::bail!("Capability not found");
}
