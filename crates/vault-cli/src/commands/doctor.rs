use crate::cli::DoctorArgs;
use anyhow::Result;
use owo_colors::OwoColorize;
use std::time::Duration;
use vault_core::config::{resolve_vault_dir, VaultConfig};
use vault_core::mcp::health::ping_mcp_server;
use vault_core::registry::{Registry, SqliteRegistry};
use vault_core::store::initialize_vault_directories;

pub async fn handle(args: DoctorArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let vault_dir = resolve_vault_dir(vault_dir_override);
    let mut clean = true;

    println!("{}", "=== AgentVault Diagnostics ===".bold().cyan());
    println!("Checking environment health...");
    println!();

    // 1. Check directory structure
    print!("  Checking directory structure... ");
    let mcps_dir = vault_dir.join("mcps");
    let skills_dir = vault_dir.join("skills");
    let workflows_dir = vault_dir.join("workflows");
    let backups_dir = vault_dir.join("backups");
    let logs_dir = vault_dir.join("logs");

    let dirs_ok = vault_dir.exists()
        && mcps_dir.exists()
        && skills_dir.exists()
        && workflows_dir.exists()
        && backups_dir.exists()
        && logs_dir.exists();

    if dirs_ok {
        println!("{}", "OK".green());
    } else {
        println!("{}", "FAILED".red());
        clean = false;
        if args.fix {
            println!("    Attempting to fix directory structure...");
            if let Err(e) = initialize_vault_directories(&vault_dir) {
                println!("    {} Failed to initialize: {}", "Error:".red(), e);
            } else {
                println!("    {} Recreated directories", "Success:".green());
            }
        } else {
            println!(
                "    {} Run {} to fix.",
                "Hint:".yellow(),
                "vault init".bold().yellow()
            );
        }
    }

    // 2. Check config file
    print!("  Checking configuration... ");
    let config_path = vault_dir.join("config.toml");
    if config_path.exists() {
        match VaultConfig::load(&config_path) {
            Ok(_) => println!("{}", "OK".green()),
            Err(e) => {
                println!("{}", "FAILED".red());
                println!("    {} Failed to load configuration: {}", "Error:".red(), e);
                clean = false;
                if args.fix {
                    println!("    Restoring default config...");
                    let default_cfg = VaultConfig {
                        vault_dir: vault_dir.clone(),
                        ..Default::default()
                    };
                    if let Err(e) = default_cfg.save(&config_path) {
                        println!(
                            "    {} Failed to save default config: {}",
                            "Error:".red(),
                            e
                        );
                    } else {
                        println!("    {} Wrote default config", "Success:".green());
                    }
                }
            }
        }
    } else {
        println!("{}", "WARNING (Not Found)".yellow());
        println!("    No config.toml exists. Using defaults.");
        if args.fix {
            println!("    Creating default config...");
            let default_cfg = VaultConfig {
                vault_dir: vault_dir.clone(),
                ..Default::default()
            };
            if let Err(e) = default_cfg.save(&config_path) {
                println!(
                    "    {} Failed to save default config: {}",
                    "Error:".red(),
                    e
                );
            } else {
                println!("    {} Wrote default config", "Success:".green());
            }
        }
    }

    // 3. Check SQLite registry DB
    print!("  Checking SQLite registry... ");
    let db_path = vault_dir.join("vault.db");
    if db_path.exists() {
        match SqliteRegistry::new(&db_path) {
            Ok(registry) => {
                println!("{}", "OK".green());
                // Test queries
                if let Err(e) = registry.list_mcps() {
                    println!("    {} Database query error: {}", "Error:".red(), e);
                    clean = false;
                }
            }
            Err(e) => {
                println!("{}", "FAILED".red());
                println!(
                    "    {} Database connection or migration error: {}",
                    "Error:".red(),
                    e
                );
                clean = false;
            }
        }
    } else {
        println!("{}", "FAILED (Not Found)".red());
        clean = false;
        if args.fix {
            println!("    Initializing registry DB...");
            if let Err(e) = SqliteRegistry::new(&db_path) {
                println!("    {} Failed to initialize DB: {}", "Error:".red(), e);
            } else {
                println!("    {} Database initialized", "Success:".green());
            }
        }
    }

    // 4. Check external command-line tools
    println!();
    println!("{}", "Checking external tools:".bold().green());
    let tools = ["git", "npm", "npx", "python", "pip", "uv"];
    for tool in &tools {
        let ok = check_tool_in_path(tool);
        let status = if ok {
            "Found".green().to_string()
        } else {
            "Not Found".red().to_string()
        };
        println!("  • {:<10} {}", tool.bold(), status);
    }

    // 5. Check Agent configuration files
    if db_path.exists() {
        if let Ok(registry) = SqliteRegistry::new(&db_path) {
            if let Ok(agents) = registry.list_agent_configs() {
                if !agents.is_empty() {
                    println!();
                    println!("{}", "Checking agent configurations:".bold().green());
                    for a in agents {
                        let path_exists = a.config_path.exists();
                        let status = if path_exists {
                            "Exists".green().to_string()
                        } else {
                            "Missing".red().to_string()
                        };
                        println!(
                            "  • {} config at {} - {}",
                            a.agent_type.to_string().bold(),
                            a.config_path.display(),
                            status
                        );
                        if !path_exists {
                            clean = false;
                        }
                    }
                }
            }
        }
    }

    // 4.5. Check MCP server responsiveness
    if args.check_mcps && db_path.exists() {
        if let Ok(registry) = SqliteRegistry::new(&db_path) {
            if let Ok(mcps) = registry.list_mcps() {
                println!();
                println!("{}", "Checking installed MCP servers:".bold().green());
                if mcps.is_empty() {
                    println!("  No installed MCP servers found.");
                } else {
                    for mcp in mcps {
                        print!("  • {:<20} ... ", mcp.name.bold());
                        match ping_mcp_server(&mcp, Duration::from_secs(5)).await {
                            Ok(()) => println!("{}", "ONLINE".green()),
                            Err(e) => {
                                println!("{} ({})", "OFFLINE".red(), e);
                                clean = false;
                            }
                        }
                    }
                }
            }
        }
    }

    println!();
    if clean {
        println!(
            "{}",
            "No issues detected. Your vault is healthy!".bold().green()
        );
    } else {
        println!(
            "{}",
            "Issues were detected. Review warnings above."
                .bold()
                .yellow()
        );
        if !args.fix {
            println!(
                "Run {} to attempt automatic fixes.",
                "vault doctor --fix".bold().yellow()
            );
        }
    }

    Ok(())
}

fn check_tool_in_path(tool: &str) -> bool {
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let p = path.join(tool);

            #[cfg(windows)]
            {
                if p.with_extension("exe").exists()
                    || p.with_extension("cmd").exists()
                    || p.with_extension("bat").exists()
                {
                    return true;
                }
            }

            #[cfg(not(windows))]
            {
                if p.exists() {
                    // Check if executable
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if let Ok(metadata) = std::fs::metadata(&p) {
                            if metadata.is_file() && (metadata.mode() & 0o111) != 0 {
                                return true;
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        if let Ok(metadata) = std::fs::metadata(&p) {
                            if metadata.is_file() {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
