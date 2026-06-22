use crate::cli::InitArgs;
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use vault_core::config::{resolve_vault_dir, VaultConfig};
use vault_core::registry::SqliteRegistry;
use vault_core::store::initialize_vault_directories;

pub async fn handle(args: InitArgs, vault_dir_override: Option<&str>) -> Result<()> {
    // Resolve target vault directory (either command-line arg, global flag, env, or default)
    let dir = args.dir.as_deref().or(vault_dir_override);
    let vault_dir = resolve_vault_dir(dir);

    println!(
        "{} AgentVault workspace at {}...",
        "Initializing".bold().green(),
        vault_dir.display().bold().cyan()
    );

    // Create filesystem directory structure
    initialize_vault_directories(&vault_dir)
        .context("Failed to initialize vault directory structure")?;

    let config_path = vault_dir.join("config.toml");
    let mut config_created = false;

    if !config_path.exists() || args.force {
        let config = VaultConfig {
            vault_dir: vault_dir.clone(),
            ..Default::default()
        };
        config
            .save(&config_path)
            .context("Failed to save default config.toml")?;
        config_created = true;
    }

    // Initialize database registry (which automatically runs migrations)
    let db_path = vault_dir.join("vault.db");
    let _registry =
        SqliteRegistry::new(&db_path).context("Failed to initialize SQLite registry")?;

    println!(
        "{} workspace successfully initialized!",
        "Success".bold().green()
    );
    if config_created {
        println!(
            "  Created default configuration at {}",
            config_path.display().cyan()
        );
    } else {
        println!(
            "  Existing configuration kept at {}",
            config_path.display().cyan()
        );
    }
    println!(
        "  SQLite registry initialized at {}",
        db_path.display().cyan()
    );

    Ok(())
}
