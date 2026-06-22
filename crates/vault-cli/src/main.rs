mod cli;
mod commands;

use clap::{Parser, CommandFactory};
use cli::{Cli, Commands, CompletionShell};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Determine log level based on verbosity/quietness flags or env var
    let env_filter = EnvFilter::try_from_env("VAULT_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    let args = Cli::parse();

    match args.command {
        Commands::Init(args) => commands::init::handle(args).await?,
        Commands::Install(args) => commands::install::handle(args).await?,
        Commands::Remove(args) => commands::remove::handle(args).await?,
        Commands::Update(args) => commands::update::handle(args).await?,
        Commands::List(args) => commands::list::handle(args).await?,
        Commands::Search(args) => commands::search::handle(args).await?,
        Commands::Sync(args) => commands::sync::handle(args).await?,
        Commands::Status(args) => commands::status::handle(args).await?,
        Commands::Config(args) => commands::config::handle(args).await?,
        Commands::Doctor(args) => commands::doctor::handle(args).await?,
        Commands::Connector(args) => commands::connector::handle(args).await?,
        Commands::Export(args) => commands::export::handle(args).await?,
        Commands::Import(args) => commands::import::handle(args).await?,
        Commands::Completions(args) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            match args.shell {
                CompletionShell::Bash => clap_complete::generate(clap_complete::shells::Bash, &mut cmd, name, &mut std::io::stdout()),
                CompletionShell::Zsh => clap_complete::generate(clap_complete::shells::Zsh, &mut cmd, name, &mut std::io::stdout()),
                CompletionShell::Fish => clap_complete::generate(clap_complete::shells::Fish, &mut cmd, name, &mut std::io::stdout()),
                CompletionShell::PowerShell => clap_complete::generate(clap_complete::shells::PowerShell, &mut cmd, name, &mut std::io::stdout()),
            }
        }
    }

    Ok(())
}
