mod cli;
mod commands;

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands, CompletionShell};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Determine log level based on verbosity/quietness flags or env var
    let env_filter =
        EnvFilter::try_from_env("VAULT_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let args = Cli::parse();
    let vault_dir_override = args.vault_dir.as_deref();

    match args.command {
        Commands::Init(subargs) => commands::init::handle(subargs, vault_dir_override).await?,
        Commands::Install(subargs) => {
            commands::install::handle(subargs, vault_dir_override).await?
        }
        Commands::Remove(subargs) => commands::remove::handle(subargs, vault_dir_override).await?,
        Commands::Update(subargs) => commands::update::handle(subargs, vault_dir_override).await?,
        Commands::List(subargs) => commands::list::handle(subargs, vault_dir_override).await?,
        Commands::Search(subargs) => commands::search::handle(subargs, vault_dir_override).await?,
        Commands::Sync(subargs) => commands::sync::handle(subargs, vault_dir_override).await?,
        Commands::Status(subargs) => commands::status::handle(subargs, vault_dir_override).await?,
        Commands::Config(subargs) => commands::config::handle(subargs, vault_dir_override).await?,
        Commands::Doctor(subargs) => commands::doctor::handle(subargs, vault_dir_override).await?,
        Commands::Connector(subargs) => {
            commands::connector::handle(subargs, vault_dir_override).await?
        }
        Commands::Export(subargs) => commands::export::handle(subargs, vault_dir_override).await?,
        Commands::Import(subargs) => commands::import::handle(subargs, vault_dir_override).await?,
        Commands::Completions(subargs) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            match subargs.shell {
                CompletionShell::Bash => clap_complete::generate(
                    clap_complete::shells::Bash,
                    &mut cmd,
                    name,
                    &mut std::io::stdout(),
                ),
                CompletionShell::Zsh => clap_complete::generate(
                    clap_complete::shells::Zsh,
                    &mut cmd,
                    name,
                    &mut std::io::stdout(),
                ),
                CompletionShell::Fish => clap_complete::generate(
                    clap_complete::shells::Fish,
                    &mut cmd,
                    name,
                    &mut std::io::stdout(),
                ),
                CompletionShell::PowerShell => clap_complete::generate(
                    clap_complete::shells::PowerShell,
                    &mut cmd,
                    name,
                    &mut std::io::stdout(),
                ),
            }
        }
    }

    Ok(())
}
