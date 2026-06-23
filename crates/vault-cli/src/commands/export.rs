use crate::cli::{ExportArgs, ExportFormat};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::sync::Arc;
use vault_core::config::resolve_vault_dir;
use vault_core::manifest::VaultManifest;
use vault_core::registry::SqliteRegistry;

pub async fn handle(args: ExportArgs, vault_dir_override: Option<&str>) -> Result<()> {
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

    println!("{}", "Exporting vault state...".bold().green());

    let manifest = VaultManifest::from_registry(&*registry)
        .context("Failed to generate manifest from registry database")?;

    let output_path = match &args.output {
        Some(path) => std::path::PathBuf::from(path),
        None => match args.format {
            ExportFormat::Toml => std::path::PathBuf::from("vault.toml"),
            ExportFormat::Json => std::path::PathBuf::from("vault.json"),
        },
    };

    let content = match args.format {
        ExportFormat::Toml => manifest
            .to_toml_string()
            .context("Failed to serialize manifest to TOML")?,
        ExportFormat::Json => serde_json::to_string_pretty(&manifest)
            .context("Failed to serialize manifest to JSON")?,
    };

    std::fs::write(&output_path, content)
        .with_context(|| format!("Failed to write manifest to {}", output_path.display()))?;

    println!(
        "{} Manifest successfully exported to {} (format: {:?})",
        "Success:".bold().green(),
        output_path.display().bold().cyan(),
        args.format
    );

    Ok(())
}
