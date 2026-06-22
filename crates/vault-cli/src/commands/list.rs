use crate::cli::ListArgs;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::sync::Arc;
use tabled::builder::Builder;
use tabled::settings::Style;
use vault_core::config::resolve_vault_dir;
use vault_core::mcp::models::{McpSource, McpStatus};
use vault_core::registry::{Registry, SqliteRegistry};
use vault_core::skill::models::SkillSource;

pub async fn handle(args: ListArgs, vault_dir_override: Option<&str>) -> Result<()> {
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

    let list_all = args.all || (!args.mcps && !args.skills && !args.workflows);
    let show_mcps = args.mcps || list_all;
    let show_skills = args.skills || list_all;
    let show_workflows = args.workflows || list_all;

    let mcps = if show_mcps {
        registry.list_mcps().context("Failed to load MCPs")?
    } else {
        Vec::new()
    };

    let skills = if show_skills {
        registry.list_skills().context("Failed to load skills")?
    } else {
        Vec::new()
    };

    let workflows = if show_workflows {
        registry.list_workflows().context("Failed to load workflows")?
    } else {
        Vec::new()
    };

    if args.json {
        let mut out = serde_json::Map::new();
        if show_mcps {
            out.insert("mcps".to_string(), serde_json::to_value(&mcps)?);
        }
        if show_skills {
            out.insert("skills".to_string(), serde_json::to_value(&skills)?);
        }
        if show_workflows {
            out.insert("workflows".to_string(), serde_json::to_value(&workflows)?);
        }
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    if mcps.is_empty() && skills.is_empty() && workflows.is_empty() {
        println!("{}", "No capabilities installed in the vault.".yellow());
        return Ok(());
    }

    if show_mcps && !mcps.is_empty() {
        println!("\n{}", "=== MCP Servers ===".bold().cyan());
        let mut builder = Builder::new();
        if args.detail {
            builder.push_record([
                "Name",
                "Version",
                "Source",
                "Transport",
                "Command",
                "Args",
                "Env Vars",
                "Status",
                "Description",
            ]);
            for mcp in &mcps {
                let source_str = format_mcp_source(&mcp.source);
                let transport_str = format!("{:?}", mcp.transport);
                let args_str = mcp.args.join(" ");
                let env_str = mcp
                    .env_vars
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");
                let status_str = format_mcp_status(&mcp.status);
                let desc_str = mcp.description.as_deref().unwrap_or("");

                builder.push_record([
                    &mcp.name,
                    &mcp.version,
                    &source_str,
                    &transport_str,
                    &mcp.command,
                    &args_str,
                    &env_str,
                    &status_str,
                    desc_str,
                ]);
            }
        } else {
            builder.push_record(["Name", "Version", "Source", "Status", "Description"]);
            for mcp in &mcps {
                let source_str = format_mcp_source(&mcp.source);
                let status_str = format_mcp_status(&mcp.status);
                let desc_str = mcp.description.as_deref().unwrap_or("");

                builder.push_record([
                    &mcp.name,
                    &mcp.version,
                    &source_str,
                    &status_str,
                    desc_str,
                ]);
            }
        }
        let mut table = builder.build();
        table.with(Style::rounded());
        println!("{}", table);
    }

    if show_skills && !skills.is_empty() {
        println!("\n{}", "=== Skills ===".bold().cyan());
        let mut builder = Builder::new();
        if args.detail {
            builder.push_record(["Name", "Source", "Path", "Tags", "Agents", "Description"]);
            for skill in &skills {
                let source_str = format_skill_source(&skill.source);
                let path_str = skill.path.to_string_lossy().to_string();
                let tags_str = skill.tags.join(", ");
                let agents_str = skill.agents.join(", ");
                let desc_str = skill.description.as_deref().unwrap_or("");

                builder.push_record([
                    &skill.name,
                    &source_str,
                    &path_str,
                    &tags_str,
                    &agents_str,
                    desc_str,
                ]);
            }
        } else {
            builder.push_record(["Name", "Source", "Description"]);
            for skill in &skills {
                let source_str = format_skill_source(&skill.source);
                let desc_str = skill.description.as_deref().unwrap_or("");

                builder.push_record([&skill.name, &source_str, desc_str]);
            }
        }
        let mut table = builder.build();
        table.with(Style::rounded());
        println!("{}", table);
    }

    if show_workflows && !workflows.is_empty() {
        println!("\n{}", "=== Workflows ===".bold().cyan());
        let mut builder = Builder::new();
        if args.detail {
            builder.push_record(["Name", "Steps", "Dependencies", "Description"]);
            for wf in &workflows {
                let steps_str = wf
                    .steps
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let deps_str = wf.dependencies.join(", ");
                let desc_str = wf.description.as_deref().unwrap_or("");

                builder.push_record([&wf.name, &steps_str, &deps_str, desc_str]);
            }
        } else {
            builder.push_record(["Name", "Steps Count", "Description"]);
            for wf in &workflows {
                let steps_count = wf.steps.len().to_string();
                let desc_str = wf.description.as_deref().unwrap_or("");

                builder.push_record([&wf.name, &steps_count, desc_str]);
            }
        }
        let mut table = builder.build();
        table.with(Style::rounded());
        println!("{}", table);
    }

    println!();
    Ok(())
}

fn format_mcp_source(source: &McpSource) -> String {
    match source {
        McpSource::Npm { package } => format!("npm:{}", package),
        McpSource::PyPi { package } => format!("pypi:{}", package),
        McpSource::GitHub { repo, ref_ } => {
            if let Some(r) = ref_ {
                format!("github:{}@{}", repo, r)
            } else {
                format!("github:{}", repo)
            }
        }
        McpSource::Local { path } => format!("local:{}", path.display()),
        McpSource::Docker { image } => format!("docker:{}", image),
    }
}

fn format_mcp_status(status: &McpStatus) -> String {
    match status {
        McpStatus::Active => "Active".green().to_string(),
        McpStatus::Disabled => "Disabled".yellow().to_string(),
        McpStatus::Error { message } => format!("Error: {}", message).red().to_string(),
    }
}

fn format_skill_source(source: &SkillSource) -> String {
    match source {
        SkillSource::Git {
            repo,
            ref_,
            subdirectory,
        } => {
            let mut s = format!("git:{}", repo);
            if let Some(r) = ref_ {
                s = format!("{}@{}", s, r);
            }
            if let Some(sub) = subdirectory {
                s = format!("{}/{}", s, sub);
            }
            s
        }
        SkillSource::Local { path } => format!("local:{}", path.display()),
    }
}
