use clap::{Parser, Subcommand, ValueEnum};

/// AgentVault — Local-first capability management for AI agents.
///
/// Install, manage, and synchronize MCP servers, skills, and workflows
/// across multiple AI coding agents from a single command line.
#[derive(Parser, Debug)]
#[command(
    name = "vault",
    version,
    author = "Aswinkumar GP",
    about = "Local-first capability management for AI agents",
    long_about = "AgentVault is a unified CLI for managing MCP servers, skills, and \
        workflows across AI coding agents like Claude Code, Gemini CLI, OpenCode, \
        and Codex CLI. Install capabilities once, sync everywhere.",
    after_help = "Examples:\n  \
        vault init                              # Initialize a new vault\n  \
        vault install npm:@anthropic/mcp-filesystem  # Install from npm\n  \
        vault sync claude                       # Sync to Claude Code\n  \
        vault sync --all                        # Sync to all agents\n  \
        vault list --table                      # List all capabilities\n  \
        vault doctor                            # Check vault health"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output (show debug-level logs).
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all output except errors.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Override the vault directory (default: ~/.agentvault/).
    #[arg(long, global = true, env = "AGENTVAULT_DIR")]
    pub vault_dir: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new AgentVault workspace.
    Init(InitArgs),

    /// Install an MCP server, skill, or workflow.
    Install(InstallArgs),

    /// Remove an installed capability.
    Remove(RemoveArgs),

    /// Update installed capabilities.
    Update(UpdateArgs),

    /// List installed capabilities.
    List(ListArgs),

    /// Search for capabilities.
    Search(SearchArgs),

    /// Synchronize capabilities to agent configurations.
    Sync(SyncArgs),

    /// Show vault status and health summary.
    Status(StatusArgs),

    /// View or modify configuration.
    Config(ConfigArgs),

    /// Diagnose vault and environment issues.
    Doctor(DoctorArgs),

    /// Manage agent connectors.
    Connector(ConnectorArgs),

    /// Export vault state to a manifest file.
    Export(ExportArgs),

    /// Import capabilities from a manifest file.
    Import(ImportArgs),

    /// Generate shell completions.
    Completions(CompletionsArgs),

    /// Start AgentVault as a Model Context Protocol (MCP) server.
    Serve(ServeArgs),

    /// Start the interactive TUI dashboard.
    Ui(UiArgs),

    /// Watch agent configurations and re-synchronize on change.
    Watch(WatchArgs),
}

// ─── vault init ──────────────────────────────────────────────────

/// Initialize a new AgentVault workspace.
///
/// Creates the ~/.agentvault/ directory structure, initializes the SQLite
/// registry, and generates a default config.toml.
///
/// Safe to run multiple times — will not overwrite existing data.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault init                    # Initialize with defaults\n  \
        vault init --dir ~/my-vault   # Custom vault directory")]
pub struct InitArgs {
    /// Override the vault directory location.
    #[arg(short, long)]
    pub dir: Option<String>,

    /// Force re-initialization (resets config, preserves installed capabilities).
    #[arg(short, long)]
    pub force: bool,
}

// ─── vault install ───────────────────────────────────────────────

/// Install an MCP server, skill, or workflow into the vault.
///
/// The source format determines the install strategy:
///   npm:<package>          Install from npm registry
///   pypi:<package>         Install from PyPI
///   github:<owner/repo>    Clone from GitHub
///   local:<path>           Link from local filesystem
///   docker:<image>         Pull Docker image
///   <bare-name>            Auto-detect from registry
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault install npm:@anthropic/mcp-filesystem\n  \
        vault install pypi:mcp-server-memory\n  \
        vault install github:anthropics/mcp-servers\n  \
        vault install local:/home/user/my-mcp\n  \
        vault install npm:@anthropic/mcp-github --env GITHUB_TOKEN=env:GITHUB_TOKEN\n  \
        vault install npm:@anthropic/mcp-filesystem --name fs --args '/home/user/projects'")]
pub struct InstallArgs {
    /// Source specifier. Format: [type:]<identifier>
    /// Examples: npm:@anthropic/mcp-filesystem, pypi:mcp-server-memory
    pub source: String,

    /// Explicit source type override (auto-detected if omitted).
    #[arg(short = 'S', long, value_enum)]
    pub source_type: Option<SourceType>,

    /// Version constraint. Default: "latest".
    /// Supports semver: "0.6.0", "^1.0", "~1.2", ">=1.0,<2.0"
    #[arg(short = 'V', long, default_value = "latest")]
    pub version: String,

    /// Display name for this MCP (shown in `vault list`).
    #[arg(short, long)]
    pub name: Option<String>,

    /// Environment variables (can be specified multiple times).
    /// Format: KEY=VALUE or KEY=env:ENV_VAR_NAME
    #[arg(short, long = "env", value_name = "KEY=VALUE")]
    pub env_vars: Vec<String>,

    /// Additional arguments passed to the MCP server command.
    #[arg(short, long, value_name = "ARGS")]
    pub args: Vec<String>,

    /// Transport protocol.
    #[arg(short, long, value_enum, default_value = "stdio")]
    pub transport: TransportType,

    /// URL for SSE or HTTP transport (required if transport is not stdio).
    #[arg(long)]
    pub url: Option<String>,

    /// Agents to sync this MCP to (can be specified multiple times).
    /// If omitted, synced to all registered agents.
    #[arg(long = "agent", value_name = "AGENT")]
    pub agents: Vec<String>,

    /// Tags for categorization (can be specified multiple times).
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,

    /// Skip confirmation prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Install as a skill.
    #[arg(long, conflicts_with_all = &["workflow", "version", "env_vars", "args", "transport", "url"])]
    pub skill: bool,

    /// Install as a workflow.
    #[arg(long, conflicts_with_all = &["skill", "version", "env_vars", "args", "transport", "url", "agents", "tags"])]
    pub workflow: bool,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum SourceType {
    Npm,
    Pypi,
    Github,
    Local,
    Docker,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum TransportType {
    Stdio,
    Sse,
    Http,
}

// ─── vault remove ────────────────────────────────────────────────

/// Remove an installed capability from the vault.
///
/// Deletes the capability's files from disk and removes its registry entry.
/// Does NOT automatically update agent configs — run `vault sync` after removal.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault remove filesystem\n  \
        vault remove github --force\n  \
        vault remove my-mcp --keep-files")]
pub struct RemoveArgs {
    /// Name of the capability to remove.
    pub name: String,

    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub force: bool,

    /// Remove from registry but keep files on disk.
    #[arg(long)]
    pub keep_files: bool,
}

// ─── vault update ────────────────────────────────────────────────

/// Update installed capabilities to their latest versions.
///
/// Respects version constraints set during install.
/// Use --force to bypass version constraints.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault update filesystem           # Update single MCP\n  \
        vault update --all                # Update all capabilities\n  \
        vault update --all --dry-run      # Preview what would change")]
pub struct UpdateArgs {
    /// Name of the capability to update. Omit for --all.
    pub name: Option<String>,

    /// Update all installed capabilities.
    #[arg(short, long)]
    pub all: bool,

    /// Show what would be updated without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Bypass version constraints.
    #[arg(short, long)]
    pub force: bool,
}

// ─── vault list ──────────────────────────────────────────────────

/// List installed capabilities.
///
/// By default, lists all capability types in a formatted table.
/// Use filters to show only specific types.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault list                    # List all\n  \
        vault list --mcps             # MCPs only\n  \
        vault list --skills --json    # Skills as JSON\n  \
        vault list --table            # Force table output")]
pub struct ListArgs {
    /// Show only MCP servers.
    #[arg(short, long)]
    pub mcps: bool,

    /// Show only skills.
    #[arg(short, long)]
    pub skills: bool,

    /// Show only workflows.
    #[arg(short, long)]
    pub workflows: bool,

    /// Show all capability types (default behavior).
    #[arg(short, long)]
    pub all: bool,

    /// Output as JSON.
    #[arg(long)]
    pub json: bool,

    /// Output as formatted table.
    #[arg(long)]
    pub table: bool,

    /// Show full details (version, source, path, env vars).
    #[arg(long)]
    pub detail: bool,
}

// ─── vault search ────────────────────────────────────────────────

/// Search for capabilities in the local registry or remote sources.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault search filesystem\n  \
        vault search github --source npm\n  \
        vault search memory --limit 5")]
pub struct SearchArgs {
    /// Search query string.
    pub query: String,

    /// Search source.
    #[arg(short, long, value_enum)]
    pub source: Option<SearchSource>,

    /// Maximum number of results to display.
    #[arg(short, long, default_value = "20")]
    pub limit: usize,

    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum SearchSource {
    /// Search only the local vault registry.
    Registry,
    /// Search the npm registry.
    Npm,
    /// Search PyPI.
    Pypi,
    /// Search GitHub.
    Github,
}

// ─── vault sync ──────────────────────────────────────────────────

/// Synchronize vault capabilities to agent configurations.
///
/// Reads the vault registry and writes MCP entries into the target
/// agent's configuration file. Creates a backup before writing.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault sync claude             # Sync to Claude Code\n  \
        vault sync --all              # Sync to all agents\n  \
        vault sync claude --dry-run   # Preview changes\n  \
        vault sync --all --force      # Overwrite all (no merge)")]
pub struct SyncArgs {
    /// Agent to sync to. Omit with --all to sync all.
    pub agent: Option<String>,

    /// Sync to all registered agents.
    #[arg(short, long)]
    pub all: bool,

    /// Show what would change without writing.
    #[arg(long)]
    pub dry_run: bool,

    /// Force overwrite (replace agent's MCP config entirely).
    /// Without this flag, vault merges with existing entries.
    #[arg(short, long)]
    pub force: bool,

    /// Create a backup before syncing (default: true, see config).
    #[arg(long)]
    pub backup: Option<bool>,

    /// Remove MCP entries from agent config that are not in the vault.
    #[arg(long)]
    pub prune: bool,
}

// ─── vault status ────────────────────────────────────────────────

/// Show vault status and health summary.
#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

// ─── vault config ────────────────────────────────────────────────

/// View or modify AgentVault configuration.
///
/// Without arguments, prints the current configuration.
/// With a key, prints that key's value.
/// With a key and value, sets the key.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault config                          # Print all config\n  \
        vault config default_agent            # Get a value\n  \
        vault config default_agent claude     # Set a value\n  \
        vault config --list                   # Print as key=value pairs\n  \
        vault config --reset                  # Reset to defaults")]
pub struct ConfigArgs {
    /// Configuration key to get or set.
    pub key: Option<String>,

    /// Value to set (requires key).
    pub value: Option<String>,

    /// Print all configuration as key=value pairs.
    #[arg(short, long)]
    pub list: bool,

    /// Reset configuration to defaults.
    #[arg(long)]
    pub reset: bool,
}

// ─── vault doctor ────────────────────────────────────────────────

/// Diagnose vault and environment issues.
///
/// Checks vault directory, database integrity, tool availability,
/// orphaned files, and agent connector status.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault doctor         # Run all checks\n  \
        vault doctor --fix   # Attempt to auto-fix issues")]
pub struct DoctorArgs {
    /// Attempt to automatically fix detected issues.
    #[arg(short, long)]
    pub fix: bool,

    /// Check MCP servers are responsive (requires running them).
    #[arg(long)]
    pub check_mcps: bool,
}

// ─── vault connector ─────────────────────────────────────────────

/// Manage agent connectors (register, list, remove).
#[derive(Parser, Debug)]
pub struct ConnectorArgs {
    #[command(subcommand)]
    pub command: ConnectorCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConnectorCommands {
    /// Register a new agent connector.
    Add(ConnectorAddArgs),

    /// List registered agent connectors.
    List(ConnectorListArgs),

    /// Remove a registered agent connector.
    Remove(ConnectorRemoveArgs),
}

#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault connector add claude\n  \
        vault connector add gemini --config-path ~/.gemini/settings.json\n  \
        vault connector add custom --config-path /path/to/config.json")]
pub struct ConnectorAddArgs {
    /// Agent type to register.
    /// Built-in: claude, gemini, opencode, codex, cursor
    /// Or any custom name.
    pub agent_type: String,

    /// Override the default config file path.
    #[arg(short, long)]
    pub config_path: Option<String>,

    /// Enable auto-sync for this connector.
    #[arg(long)]
    pub auto_sync: bool,
}

#[derive(Parser, Debug)]
pub struct ConnectorListArgs {
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct ConnectorRemoveArgs {
    /// Agent type to remove.
    pub agent_type: String,

    /// Skip confirmation prompt.
    #[arg(short, long)]
    pub force: bool,
}

// ─── vault export ────────────────────────────────────────────────

/// Export current vault state to a manifest file.
///
/// Generates a vault.toml (or JSON) file that can be imported
/// on another machine to reproduce the same capability set.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault export                       # Export to ./vault.toml\n  \
        vault export --output my-vault.toml\n  \
        vault export --format json --output vault.json")]
pub struct ExportArgs {
    /// Output format.
    #[arg(short, long, value_enum, default_value = "toml")]
    pub format: ExportFormat,

    /// Output file path. Default: ./vault.toml (or ./vault.json).
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ExportFormat {
    Toml,
    Json,
}

// ─── vault import ────────────────────────────────────────────────

/// Import capabilities from a manifest file.
///
/// Reads a vault.toml (or JSON) file and installs any capabilities
/// that are not already present in the vault. Optionally prunes
/// capabilities not in the manifest.
#[derive(Parser, Debug, Clone)]
#[command(after_help = "Examples:\n  \
        vault import vault.toml\n  \
        vault import vault.toml --dry-run\n  \
        vault import vault.toml --replace\n  \
        vault import vault.json --merge")]
pub struct ImportArgs {
    /// Path to the manifest file to import.
    pub file: String,

    /// Show what would change without making modifications.
    #[arg(long)]
    pub dry_run: bool,

    /// Merge with existing vault state (default).
    /// New capabilities are added, existing ones are updated.
    #[arg(long, group = "strategy")]
    pub merge: bool,

    /// Replace entire vault state with the manifest.
    /// Capabilities not in the manifest are removed.
    #[arg(long, group = "strategy")]
    pub replace: bool,
}

// ─── vault completions ──────────────────────────────────────────

/// Generate shell completion scripts.
#[derive(Parser, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

// ─── vault serve ──────────────────────────────────────────────────

/// Start AgentVault as a Model Context Protocol (MCP) server.
///
/// Exposes AgentVault's installation, removal, update, sync, and listing capabilities
/// as standardized MCP tools. Other AI agents can connect to this process to dynamically
/// modify their tools/skills, check status, or synchronize settings.
#[derive(Parser, Debug)]
#[command(
    after_help = "Examples:\n  vault serve             # Start stdio-based MCP server\n  vault serve --gateway   # Start as gateway aggregating all installed MCPs"
)]
pub struct ServeArgs {
    /// Run in gateway mode: spawn all installed MCP servers and
    /// aggregate their tools behind a single unified endpoint.
    #[arg(long)]
    pub gateway: bool,
}

// ─── vault ui ─────────────────────────────────────────────────────

/// Start the interactive TUI dashboard.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  vault ui             # Start interactive TUI dashboard")]
pub struct UiArgs {
    /// Initial color theme (slate, nord, dracula, monokai).
    #[arg(short, long)]
    pub theme: Option<String>,
}

// ─── vault watch ──────────────────────────────────────────────────

/// Watch agent configurations and re-synchronize on change.
#[derive(Parser, Debug)]
#[command(after_help = "Examples:\n  \
        vault watch             # Run in foreground\n  \
        vault watch --daemon    # Run in background as daemon")]
pub struct WatchArgs {
    /// Run as a background daemon.
    #[arg(short, long)]
    pub daemon: bool,
}
