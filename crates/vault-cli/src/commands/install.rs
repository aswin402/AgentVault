use crate::cli::{InstallArgs, SourceType};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use vault_core::config::resolve_vault_dir;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::mcp::models::McpSource;
use vault_core::registry::SqliteRegistry;

use vault_core::skill::manager::SkillManager;
use vault_core::skill::models::SkillSource;
use vault_core::workflow::manager::WorkflowManager;
use vault_core::workflow::manager::WorkflowSource;

pub async fn handle(args: InstallArgs, vault_dir_override: Option<&str>) -> Result<()> {
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

    // ─── Skill Installation ───
    if args.skill {
        let source = parse_skill_source(&args.source)?;
        println!(
            "{} Skill from {}...",
            "Installing".bold().green(),
            args.source.bold().cyan()
        );
        let skill_manager =
            vault_core::skill::manager::DefaultSkillManager::new(registry, vault_dir);
        let entry = skill_manager
            .install(source, args.agents, args.tags)
            .await?;
        println!(
            "{} Skill {} successfully installed! (path: {})",
            "Success".bold().green(),
            entry.name.bold().cyan(),
            entry.path.display().yellow()
        );
        return Ok(());
    }

    // ─── Workflow Installation ───
    if args.workflow {
        let source = parse_workflow_source(&args.source)?;
        println!(
            "{} Workflow from {}...",
            "Installing".bold().green(),
            args.source.bold().cyan()
        );
        let workflow_manager =
            vault_core::workflow::manager::DefaultWorkflowManager::new(registry, vault_dir);
        let entry = workflow_manager.install(source).await?;
        println!(
            "{} Workflow {} successfully installed! (steps: {})",
            "Success".bold().green(),
            entry.name.bold().cyan(),
            entry.steps.len().to_string().yellow()
        );

        // Validate dependencies
        let issues = workflow_manager.validate(&entry.name)?;
        if !issues.is_empty() {
            println!(
                "\n{} Workflow installed with unresolved dependencies:",
                "Warning:".bold().yellow()
            );
            for issue in issues {
                println!(
                    "  - Step '{}': {}",
                    issue.step_name.bold(),
                    issue.message.yellow()
                );
            }
            println!("Please install the missing capabilities to run the workflow.");
        }
        return Ok(());
    }

    // ─── MCP Installation ───
    let manager = DefaultMcpManager::new(registry, vault_dir);

    let source = parse_source(&args.source, args.source_type.as_ref())?;
    let name = args
        .name
        .clone()
        .unwrap_or_else(|| get_default_name(&source));

    let mut env_map = HashMap::new();
    for env in args.env_vars {
        if let Some((k, v)) = env.split_once('=') {
            let k = k.trim().to_string();
            let v = v.trim();
            let resolved_value = if let Some(env_name) = v.strip_prefix("env:") {
                std::env::var(env_name).unwrap_or_default()
            } else {
                v.to_string()
            };
            env_map.insert(k, resolved_value);
        }
    }

    println!(
        "{} MCP server {}...",
        "Installing".bold().green(),
        name.bold().cyan()
    );

    let entry = manager
        .install(
            &name,
            source,
            &args.version,
            args.args,
            env_map,
            args.agents,
            args.tags,
            None,
        )
        .await?;

    println!(
        "{} MCP server {} successfully installed! (version: {}, command: {})",
        "Success".bold().green(),
        entry.name.bold().cyan(),
        entry.version.green(),
        entry.command.yellow()
    );

    Ok(())
}

fn parse_skill_source(source: &str) -> Result<SkillSource> {
    if let Some((prefix, rest)) = source.split_once(':') {
        match prefix.to_lowercase().as_str() {
            "local" => {
                return Ok(SkillSource::Local {
                    path: PathBuf::from(rest),
                });
            }
            "git" | "github" => {
                let parts: Vec<&str> = rest.split('@').collect();
                let repo_url = if parts[0].starts_with("http") || parts[0].starts_with("git@") {
                    parts[0].to_string()
                } else {
                    format!("https://github.com/{}.git", parts[0])
                };
                let ref_ = parts.get(1).map(|s| s.to_string());
                return Ok(SkillSource::Git {
                    repo: repo_url,
                    ref_,
                    subdirectory: None,
                });
            }
            _ => {}
        }
    }

    if source.starts_with('/')
        || source.starts_with("./")
        || source.starts_with("../")
        || std::path::Path::new(source).exists()
    {
        Ok(SkillSource::Local {
            path: PathBuf::from(source),
        })
    } else {
        let parts: Vec<&str> = source.split('@').collect();
        let repo_url = format!("https://github.com/{}.git", parts[0]);
        let ref_ = parts.get(1).map(|s| s.to_string());
        Ok(SkillSource::Git {
            repo: repo_url,
            ref_,
            subdirectory: None,
        })
    }
}

fn parse_workflow_source(source: &str) -> Result<WorkflowSource> {
    if let Some((prefix, rest)) = source.split_once(':') {
        match prefix.to_lowercase().as_str() {
            "local" => {
                return Ok(WorkflowSource::Local {
                    path: PathBuf::from(rest),
                });
            }
            "git" | "github" => {
                let parts: Vec<&str> = rest.split('@').collect();
                let repo_url = if parts[0].starts_with("http") || parts[0].starts_with("git@") {
                    parts[0].to_string()
                } else {
                    format!("https://github.com/{}.git", parts[0])
                };
                let ref_ = parts.get(1).map(|s| s.to_string());
                return Ok(WorkflowSource::Git {
                    repo: repo_url,
                    ref_,
                    subdirectory: None,
                });
            }
            _ => {}
        }
    }

    if source.starts_with('/')
        || source.starts_with("./")
        || source.starts_with("../")
        || std::path::Path::new(source).exists()
    {
        Ok(WorkflowSource::Local {
            path: PathBuf::from(source),
        })
    } else {
        let parts: Vec<&str> = source.split('@').collect();
        let repo_url = format!("https://github.com/{}.git", parts[0]);
        let ref_ = parts.get(1).map(|s| s.to_string());
        Ok(WorkflowSource::Git {
            repo: repo_url,
            ref_,
            subdirectory: None,
        })
    }
}

fn parse_source(source: &str, source_type_override: Option<&SourceType>) -> Result<McpSource> {
    if let Some(st) = source_type_override {
        return match st {
            SourceType::Npm => Ok(McpSource::Npm {
                package: source.to_string(),
            }),
            SourceType::Pypi => Ok(McpSource::PyPi {
                package: source.to_string(),
            }),
            SourceType::Local => Ok(McpSource::Local {
                path: PathBuf::from(source),
            }),
            SourceType::Github => {
                let parts: Vec<&str> = source.split('@').collect();
                let repo = parts[0].to_string();
                let ref_ = parts.get(1).map(|s| s.to_string());
                Ok(McpSource::GitHub { repo, ref_ })
            }
            SourceType::Docker => Ok(McpSource::Docker {
                image: source.to_string(),
            }),
        };
    }

    if let Some((prefix, rest)) = source.split_once(':') {
        match prefix.to_lowercase().as_str() {
            "npm" => {
                return Ok(McpSource::Npm {
                    package: rest.to_string(),
                })
            }
            "pypi" => {
                return Ok(McpSource::PyPi {
                    package: rest.to_string(),
                })
            }
            "local" => {
                return Ok(McpSource::Local {
                    path: PathBuf::from(rest),
                })
            }
            "github" => {
                let parts: Vec<&str> = rest.split('@').collect();
                let repo = parts[0].to_string();
                let ref_ = parts.get(1).map(|s| s.to_string());
                return Ok(McpSource::GitHub { repo, ref_ });
            }
            "docker" => {
                return Ok(McpSource::Docker {
                    image: rest.to_string(),
                })
            }
            _ => {}
        }
    }

    if source.starts_with('/')
        || source.starts_with("./")
        || source.starts_with("../")
        || (source.contains('/') && !source.contains(':') && std::path::Path::new(source).exists())
    {
        Ok(McpSource::Local {
            path: PathBuf::from(source),
        })
    } else if source.contains('/') {
        let parts: Vec<&str> = source.split('@').collect();
        let repo = parts[0].to_string();
        let ref_ = parts.get(1).map(|s| s.to_string());
        Ok(McpSource::GitHub { repo, ref_ })
    } else {
        Ok(McpSource::Npm {
            package: source.to_string(),
        })
    }
}

fn get_default_name(source: &McpSource) -> String {
    match source {
        McpSource::Npm { package } => package
            .split('/')
            .next_back()
            .unwrap_or(package)
            .to_string(),
        McpSource::PyPi { package } => package.to_string(),
        McpSource::GitHub { repo, .. } => repo.split('/').next_back().unwrap_or(repo).to_string(),
        McpSource::Local { path } => path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "local-mcp".to_string()),
        McpSource::Docker { image } => image
            .split(':')
            .next()
            .and_then(|i| i.split('/').next_back())
            .unwrap_or(image)
            .to_string(),
    }
}
