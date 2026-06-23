use crate::cli::{SearchArgs, SearchSource};
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::sync::Arc;
use tabled::builder::Builder;
use tabled::settings::Style;
use vault_core::config::resolve_vault_dir;
use vault_core::registry::SqliteRegistry;
use vault_core::search::SearchEngine;

pub async fn handle(args: SearchArgs, vault_dir_override: Option<&str>) -> Result<()> {
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

    let mut search_local = false;
    let mut search_npm = false;

    match args.source {
        Some(SearchSource::Registry) => {
            search_local = true;
        }
        Some(SearchSource::Npm) => {
            search_npm = true;
        }
        Some(SearchSource::Pypi) | Some(SearchSource::Github) => {
            println!(
                "{} Search for PyPI/GitHub is not yet implemented. Searching local registry instead.",
                "Warning:".bold().yellow()
            );
            search_local = true;
        }
        None => {
            search_local = true;
            search_npm = true;
        }
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    let engine = SearchEngine::new(registry);
    let mut local_results = Vec::new();
    let mut npm_results = Vec::new();

    if search_local {
        pb.set_message("Searching local registry...");
        match engine.search_local(&args.query) {
            Ok(res) => local_results = res,
            Err(e) => {
                pb.println(format!(
                    "{} Local search failed: {}",
                    "Error:".bold().red(),
                    e
                ));
            }
        }
    }

    if search_npm {
        pb.set_message("Searching npm registry...");
        match engine.search_npm(&args.query).await {
            Ok(res) => npm_results = res,
            Err(e) => {
                pb.println(format!(
                    "{} npm registry search failed: {}",
                    "Error:".bold().red(),
                    e
                ));
            }
        }
    }

    pb.finish_and_clear();

    // Combine and deduplicate
    let mut seen_names = std::collections::HashSet::new();
    let mut combined_results = Vec::new();

    for res in local_results {
        seen_names.insert(res.name.clone());
        combined_results.push(res);
    }

    for res in npm_results {
        if !seen_names.contains(&res.name) {
            combined_results.push(res);
        }
    }

    // Sort by relevance score desc, name asc
    combined_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });

    if combined_results.len() > args.limit {
        combined_results.truncate(args.limit);
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&combined_results)?);
        return Ok(());
    }

    if combined_results.is_empty() {
        println!("No capabilities found matching '{}'.", args.query.yellow());
        return Ok(());
    }

    let mut builder = Builder::new();
    builder.push_record(["Installed", "Name", "Source", "Relevance", "Description"]);

    for res in &combined_results {
        let installed_str = if res.source == "local" {
            "✓".bold().green().to_string()
        } else {
            "".to_string()
        };

        let name_str = res.name.bold().to_string();

        let source_str = if res.source == "local" {
            "local".dimmed().to_string()
        } else {
            "npm".yellow().to_string()
        };

        let score_str = if res.score >= 0.8 {
            format!("{:.1}", res.score).green().to_string()
        } else if res.score >= 0.5 {
            format!("{:.1}", res.score).yellow().to_string()
        } else {
            format!("{:.1}", res.score).dimmed().to_string()
        };

        let desc_str = res.description.as_deref().unwrap_or("");

        builder.push_record([
            installed_str,
            name_str,
            source_str,
            score_str,
            desc_str.to_string(),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::rounded());
    println!("{}", table);

    Ok(())
}
