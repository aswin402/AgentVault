use crate::cli::StatusArgs;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use serde_json::json;
use vault_core::config::resolve_vault_dir;
use vault_core::registry::{Registry, SqliteRegistry};

pub async fn handle(args: StatusArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let vault_dir = resolve_vault_dir(vault_dir_override);
    let db_path = vault_dir.join("vault.db");

    // Check if vault is initialized
    if !db_path.exists() {
        if args.json {
            println!(
                "{}",
                json!({ "initialized": false, "vault_dir": vault_dir })
            );
            return Ok(());
        }
        println!(
            "{} Vault is not initialized. Run {} to start.",
            "Error:".bold().red(),
            "vault init".bold().yellow()
        );
        return Ok(());
    }

    let registry = SqliteRegistry::new(&db_path).context("Failed to open registry database")?;

    let mcps = registry.list_mcps().context("Failed to load MCPs")?;
    let skills = registry.list_skills().context("Failed to load skills")?;
    let workflows = registry
        .list_workflows()
        .context("Failed to load workflows")?;
    let agents = registry
        .list_agent_configs()
        .context("Failed to load agent connectors")?;

    if args.json {
        let out = json!({
            "initialized": true,
            "vault_dir": vault_dir,
            "counts": {
                "mcps": mcps.len(),
                "skills": skills.len(),
                "workflows": workflows.len(),
                "agent_connectors": agents.len(),
            },
            "agent_connectors": agents.iter().map(|a| {
                json!({
                    "agent_type": a.agent_type.to_string(),
                    "enabled": a.enabled,
                    "last_synced": a.last_synced,
                    "auto_sync": a.auto_sync,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", out);
        return Ok(());
    }

    println!("{}", "=== AgentVault Status ===".bold().cyan());
    println!("Vault Directory: {}", vault_dir.display().yellow());
    println!("Registry DB:     {}", db_path.display().yellow());
    println!();
    println!("{}", "Capability Counts:".bold().green());
    println!("  MCP Servers:   {}", mcps.len());
    println!("  Skills:        {}", skills.len());
    println!("  Workflows:     {}", workflows.len());
    println!();
    println!("{}", "Agent Connectors:".bold().green());
    if agents.is_empty() {
        println!(
            "  No agent connectors registered. Run {} to add one.",
            "vault connector add <agent>".yellow()
        );
    } else {
        for a in agents {
            let status_str = if a.enabled {
                "enabled".green().to_string()
            } else {
                "disabled".red().to_string()
            };
            let sync_str = match a.last_synced {
                Some(dt) => format!("last synced {}", dt.to_rfc3339()),
                None => "never synced".yellow().to_string(),
            };
            println!(
                "  • {} (Config: {}, {}, {})",
                a.agent_type.to_string().bold(),
                a.config_path.display(),
                status_str,
                sync_str
            );
        }
    }

    Ok(())
}
