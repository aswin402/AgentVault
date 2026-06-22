# AgentVault — Implementation Plan

> **A local-first capability management system for AI agents, built in Rust.**
>
> This document is the authoritative, phase-by-phase implementation plan.
> Every phase is self-contained: it declares exactly what ships, which files change,
> which crates are touched, what dependencies are added, how to test it, and when
> it's done. Treat each phase as a mini-sprint with a hard definition of done.

---

## Architecture at a Glance

```
┌──────────────────────────────────────────────────────────┐
│                       vault-cli                          │
│  (binary crate — clap derive, TUI output, user-facing)  │
│                                                          │
│  src/main.rs                                             │
│  src/commands/{init,install,remove,update,list,search,   │
│                sync,status,doctor,config,connector,      │
│                export,import}.rs                         │
│  src/output.rs  ← rich formatting (tables, colors, etc) │
└───────────────────────┬──────────────────────────────────┘
                        │ depends on
          ┌─────────────┴──────────────┐
          ▼                            ▼
┌──────────────────────┐  ┌─────────────────────────────┐
│     vault-core       │  │     vault-connectors        │
│  (library crate)     │  │  (library crate)            │
│                      │  │                             │
│  config, registry,   │  │  AgentConnector trait       │
│  store, mcp manager, │  │  Claude, Gemini, OpenCode,  │
│  skill manager,      │  │  Codex connectors           │
│  workflow manager,   │  │  Sync engine                │
│  manifest, search,   │  │                             │
│  error types, models │  │  depends on vault-core      │
└──────────────────────┘  └─────────────────────────────┘
```

### Filesystem Layout (`~/.agentvault/`)

```
~/.agentvault/
├── config.toml          # Global configuration
├── vault.db             # SQLite registry (single file)
├── mcps/                # Installed MCP server files/dirs
│   └── <mcp-name>/
├── skills/              # Installed skill directories
│   └── <skill-name>/
├── workflows/           # Installed workflow definitions
│   └── <workflow-name>/
├── backups/             # Agent config backups (timestamped)
│   ├── claude/
│   ├── gemini/
│   ├── opencode/
│   └── codex/
└── logs/                # Operation and sync logs
```

### SQLite Schema (Preview)

| Table           | Purpose                                       |
|-----------------|-----------------------------------------------|
| `mcps`          | Installed MCP server records                  |
| `skills`        | Installed skill records                       |
| `workflows`     | Installed workflow records                    |
| `capabilities`  | Unified capability view (kind, ref_id, tags)  |
| `agent_configs` | Registered agent connectors                   |
| `sync_history`  | Audit log of every sync operation             |

---

## Dependency Map

All crates and their minimum versions used across the workspace:

| Crate               | Version   | Used In              | Purpose                          |
|----------------------|-----------|----------------------|----------------------------------|
| `clap`               | `4.x`    | vault-cli            | CLI argument parsing (derive)    |
| `clap_complete`      | `4.x`    | vault-cli            | Shell completion generation      |
| `clap_mangen`        | `0.2`    | vault-cli (build)    | Man page generation              |
| `serde`              | `1.x`    | vault-core           | Serialization framework          |
| `serde_json`         | `1.x`    | vault-core, connectors | JSON read/write               |
| `toml`               | `0.8`    | vault-core           | TOML config/manifest parsing     |
| `rusqlite`           | `0.31`   | vault-core           | SQLite (bundled feature)         |
| `tokio`              | `1.x`    | vault-core, cli      | Async runtime (full features)    |
| `reqwest`            | `0.12`   | vault-core           | HTTP client (rustls-tls)         |
| `indicatif`          | `0.17`   | vault-cli            | Progress bars / spinners         |
| `owo-colors`         | `4.x`    | vault-cli            | Terminal coloring                |
| `thiserror`          | `2.x`    | vault-core           | Ergonomic error derive           |
| `anyhow`             | `1.x`    | vault-cli            | Application-level error context  |
| `dirs`               | `5.x`    | vault-core           | Platform-specific directories    |
| `chrono`             | `0.4`    | vault-core           | Date/time handling               |
| `semver`             | `1.x`    | vault-core           | Semantic versioning              |
| `sha2`               | `0.10`   | vault-core           | SHA-256 checksums                |
| `tracing`            | `0.1`    | all                  | Structured logging               |
| `tracing-subscriber` | `0.3`    | vault-cli            | Log output formatting            |
| `dialoguer`          | `0.11`   | vault-cli            | Interactive prompts              |
| `tabled`             | `0.16`   | vault-cli            | Terminal table formatting        |
| `async-trait`        | `0.1`    | vault-core, connectors | Async trait support            |
| `uuid`               | `1.x`    | vault-core           | Unique identifiers               |
| `tempfile`           | `3.x`    | vault-core (dev)     | Temp dirs in tests               |

---

## Phase 0: Project Bootstrap

> **Goal:** A compilable Cargo workspace with a fully-defined CLI skeleton,
> error types, structured logging, and CI. Every command exists but prints
> "not yet implemented."

**Estimated Time:** 1 day (Day 1)

### Scope

| What                              | Detail                                                                 |
|-----------------------------------|------------------------------------------------------------------------|
| Workspace initialization          | Cargo workspace with three member crates                               |
| CLI skeleton                      | Every subcommand defined via `clap` derive; prints stub message        |
| Error foundation                  | `VaultError` enum with all known variants                              |
| Logging                           | `tracing-subscriber` with `VAULT_LOG` env filter                       |
| CI                                | GitHub Actions: fmt, clippy, test on Ubuntu / macOS / Windows          |
| `.gitignore`                      | Standard Rust ignores                                                  |

### Files to Create / Modify

| File (relative to project root)                         | Action | Crate/Module          |
|----------------------------------------------------------|--------|-----------------------|
| `Cargo.toml`                                             | Create | workspace root        |
| `crates/vault-cli/Cargo.toml`                            | Create | vault-cli             |
| `crates/vault-cli/src/main.rs`                           | Create | vault-cli             |
| `crates/vault-cli/src/commands/mod.rs`                   | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/init.rs`                  | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/install.rs`               | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/remove.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/update.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/list.rs`                  | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/search.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/sync.rs`                  | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/status.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/doctor.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/config.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/connector.rs`             | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/export.rs`                | Create | vault-cli::commands   |
| `crates/vault-cli/src/commands/import.rs`                | Create | vault-cli::commands   |
| `crates/vault-core/Cargo.toml`                           | Create | vault-core            |
| `crates/vault-core/src/lib.rs`                           | Create | vault-core            |
| `crates/vault-core/src/error.rs`                         | Create | vault-core::error     |
| `crates/vault-connectors/Cargo.toml`                     | Create | vault-connectors      |
| `crates/vault-connectors/src/lib.rs`                     | Create | vault-connectors      |
| `.gitignore`                                             | Create | root                  |
| `.github/workflows/ci.yml`                               | Create | CI                    |

### Dependencies Added

| Crate            | `vault-cli` | `vault-core` | `vault-connectors` |
|------------------|:-----------:|:------------:|:-------------------:|
| clap (derive)    | ✓           |              |                     |
| anyhow           | ✓           |              |                     |
| indicatif        | ✓           |              |                     |
| owo-colors       | ✓           |              |                     |
| dialoguer        | ✓           |              |                     |
| tabled           | ✓           |              |                     |
| tracing          | ✓           | ✓            | ✓                   |
| tracing-subscriber | ✓         |              |                     |
| serde            |             | ✓            |                     |
| serde_json       |             | ✓            | ✓                   |
| toml             |             | ✓            |                     |
| rusqlite         |             | ✓            |                     |
| tokio            | ✓           | ✓            |                     |
| reqwest          |             | ✓            |                     |
| thiserror        |             | ✓            |                     |
| dirs             |             | ✓            |                     |
| chrono           |             | ✓            |                     |
| semver           |             | ✓            |                     |
| sha2             |             | ✓            |                     |
| uuid             |             | ✓            |                     |
| async-trait      |             | ✓            | ✓                   |
| vault-core       | ✓           |              | ✓                   |
| vault-connectors | ✓           |              |                     |

### `VaultError` Variants

```rust
// crates/vault-core/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Connector error ({agent}): {message}")]
    Connector { agent: String, message: String },

    #[error("MCP installation failed ({source}): {message}")]
    McpInstall { source: String, message: String },

    #[error("Not found: {kind} '{name}'")]
    NotFound { kind: String, name: String },

    #[error("Already exists: {kind} '{name}'")]
    AlreadyExists { kind: String, name: String },

    #[error("Version conflict for '{name}': wanted {wanted}, found {found}")]
    VersionConflict { name: String, wanted: String, found: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Serialization error: {0}")]
    Serialization(String),
}
```

### CLI Skeleton (clap derive)

```
vault
├── init                  # Initialize a new vault
├── install <source>      # Install a capability
├── remove <name>         # Remove a capability
├── update [name]         # Update one or all
├── list                  # List installed capabilities
│   ├── --mcps
│   ├── --skills
│   ├── --workflows
│   ├── --json
│   └── --table
├── search <query>        # Search for capabilities
│   ├── --local
│   ├── --npm
│   └── --limit <n>
├── sync <agent>          # Sync to an agent config
│   ├── --all
│   └── --dry-run
├── status                # Vault health summary
├── doctor                # Diagnose environment
├── config                # View/set config values
│   ├── set <key> <value>
│   └── get [key]
├── connector             # Manage agent connectors
│   ├── add <agent>
│   ├── list
│   └── remove <agent>
├── export                # Export state to vault.toml
│   └── --output <path>
└── import <path>         # Import from vault.toml
    ├── --dry-run
    ├── --merge
    ├── --replace
    └── --prune
```

### Testing Requirements

| Test                                    | Type       | Assertion                                           |
|-----------------------------------------|------------|-----------------------------------------------------|
| `cargo build --workspace`               | Build      | Compiles with zero errors                           |
| `cargo clippy --workspace`              | Lint       | No warnings                                         |
| `cargo fmt --all -- --check`            | Format     | No formatting diffs                                 |
| `cargo test --workspace`                | Unit       | All tests pass (trivial at this phase)               |
| Run `vault --help`                      | Smoke      | Prints help text listing all commands                |
| Run `vault install test`                | Smoke      | Prints "not yet implemented"                        |
| CI on Ubuntu, macOS, Windows            | Integration| Green on all three OS targets                       |

### Definition of Done

- [ ] `cargo build --workspace` succeeds on stable Rust (zero errors, zero warnings)
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `vault --help` prints complete help with all subcommands listed
- [ ] Every subcommand (`vault init`, `vault install foo`, etc.) runs and prints its stub message
- [ ] `VaultError` enum compiles and has `From` impls for `std::io::Error`, `rusqlite::Error`, `reqwest::Error`
- [ ] `tracing-subscriber` initializes and respects `VAULT_LOG=debug` env var
- [ ] CI workflow runs on push/PR for Ubuntu, macOS, Windows with stable Rust
- [ ] `.gitignore` covers `target/`, `*.db`, `.env`, `*.swp`

---

## Phase 1: Storage Foundation

> **Goal:** The vault has persistent storage: a config file, a SQLite database,
> and a directory structure. `vault init`, `vault status`, and `vault doctor`
> are fully functional.

**Estimated Time:** 2 days (Days 2–3)

### Scope

| What                    | Detail                                                                      |
|-------------------------|-----------------------------------------------------------------------------|
| Config                  | `VaultConfig` struct, `config.toml` parse/write, defaults                   |
| SQLite schema           | All 6 tables, migration on first open                                       |
| Registry trait          | `Registry` trait + `SqliteRegistry` with full CRUD                          |
| Filesystem store        | `~/.agentvault/` directory creation and validation                          |
| `vault init`            | Creates dirs, DB, default config; idempotent                                |
| `vault status`          | Shows vault dir, counts, last sync, DB size                                 |
| `vault doctor`          | Health checks: dir, DB, config, npm, pip, uv, git availability             |

### Files to Create / Modify

| File                                                    | Action | Crate/Module              |
|---------------------------------------------------------|--------|---------------------------|
| `crates/vault-core/src/config.rs`                       | Create | vault-core::config        |
| `crates/vault-core/src/registry.rs`                     | Create | vault-core::registry      |
| `crates/vault-core/src/store.rs`                        | Create | vault-core::store         |
| `crates/vault-core/src/lib.rs`                          | Modify | vault-core (re-exports)   |
| `crates/vault-cli/src/commands/init.rs`                 | Modify | vault-cli::commands       |
| `crates/vault-cli/src/commands/status.rs`               | Modify | vault-cli::commands       |
| `crates/vault-cli/src/commands/doctor.rs`               | Modify | vault-cli::commands       |

### Detailed Implementation

#### `VaultConfig` (`vault-core/src/config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Root directory for vault data (default: ~/.agentvault)
    pub vault_dir: PathBuf,
    /// Default agent to sync to (e.g., "claude")
    pub default_agent: Option<String>,
    /// Auto-sync to all agents after install/remove/update
    pub sync_on_install: bool,
    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,
}
```

- Loaded from `~/.agentvault/config.toml`
- `VaultConfig::default()` produces sensible defaults
- `VaultConfig::load(path)` reads and deserializes
- `VaultConfig::save(path)` serializes and writes atomically

#### SQLite Schema (`vault-core/src/registry.rs`)

```sql
CREATE TABLE IF NOT EXISTS mcps (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    version     TEXT NOT NULL,
    source      TEXT NOT NULL,      -- JSON: McpSource
    transport   TEXT NOT NULL,      -- JSON: McpTransport
    config_json TEXT DEFAULT '{}',
    env_vars    TEXT DEFAULT '{}',
    installed_at TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    checksum    TEXT
);

CREATE TABLE IF NOT EXISTS skills (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    version     TEXT NOT NULL,
    source      TEXT NOT NULL,
    path        TEXT NOT NULL,
    tags        TEXT DEFAULT '[]',
    description TEXT DEFAULT '',
    installed_at TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflows (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    version     TEXT NOT NULL,
    source      TEXT NOT NULL,
    definition_json TEXT NOT NULL,
    installed_at TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS capabilities (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,      -- 'mcp' | 'skill' | 'workflow'
    ref_id      TEXT NOT NULL,      -- FK to mcps.id / skills.id / workflows.id
    tags        TEXT DEFAULT '[]',
    description TEXT DEFAULT ''
);

CREATE TABLE IF NOT EXISTS agent_configs (
    id          TEXT PRIMARY KEY,
    agent_type  TEXT NOT NULL UNIQUE,
    config_path TEXT NOT NULL,
    last_synced TEXT,
    enabled     INTEGER DEFAULT 1
);

CREATE TABLE IF NOT EXISTS sync_history (
    id          TEXT PRIMARY KEY,
    agent_type  TEXT NOT NULL,
    action      TEXT NOT NULL,
    diff_json   TEXT DEFAULT '{}',
    synced_at   TEXT NOT NULL,
    success     INTEGER NOT NULL
);
```

#### Registry Trait

```rust
#[async_trait]
pub trait Registry: Send + Sync {
    // MCP operations
    fn insert_mcp(&self, entry: &McpEntry) -> Result<(), VaultError>;
    fn get_mcp(&self, name: &str) -> Result<McpEntry, VaultError>;
    fn list_mcps(&self) -> Result<Vec<McpEntry>, VaultError>;
    fn update_mcp(&self, entry: &McpEntry) -> Result<(), VaultError>;
    fn delete_mcp(&self, name: &str) -> Result<(), VaultError>;

    // Skill operations (same pattern)
    fn insert_skill(&self, entry: &SkillEntry) -> Result<(), VaultError>;
    fn get_skill(&self, name: &str) -> Result<SkillEntry, VaultError>;
    fn list_skills(&self) -> Result<Vec<SkillEntry>, VaultError>;
    fn update_skill(&self, entry: &SkillEntry) -> Result<(), VaultError>;
    fn delete_skill(&self, name: &str) -> Result<(), VaultError>;

    // Workflow operations (same pattern)
    fn insert_workflow(&self, entry: &WorkflowEntry) -> Result<(), VaultError>;
    fn get_workflow(&self, name: &str) -> Result<WorkflowEntry, VaultError>;
    fn list_workflows(&self) -> Result<Vec<WorkflowEntry>, VaultError>;
    fn update_workflow(&self, entry: &WorkflowEntry) -> Result<(), VaultError>;
    fn delete_workflow(&self, name: &str) -> Result<(), VaultError>;

    // Agent config operations
    fn insert_agent_config(&self, config: &AgentConfig) -> Result<(), VaultError>;
    fn get_agent_config(&self, agent_type: &str) -> Result<AgentConfig, VaultError>;
    fn list_agent_configs(&self) -> Result<Vec<AgentConfig>, VaultError>;
    fn delete_agent_config(&self, agent_type: &str) -> Result<(), VaultError>;

    // Sync history
    fn log_sync(&self, entry: &SyncHistoryEntry) -> Result<(), VaultError>;
    fn get_sync_history(&self, agent_type: &str, limit: usize) -> Result<Vec<SyncHistoryEntry>, VaultError>;

    // Search
    fn search(&self, query: &str) -> Result<Vec<CapabilityRecord>, VaultError>;
}
```

#### `vault doctor` Checks

| Check                                | Pass Criteria                          | Fail Message                                   |
|--------------------------------------|----------------------------------------|-------------------------------------------------|
| Vault directory exists               | `~/.agentvault/` exists and writable   | `"Vault directory not found. Run 'vault init'"` |
| SQLite database accessible           | Can open and query `vault.db`          | `"Database corrupted or inaccessible"`          |
| SQLite integrity check               | `PRAGMA integrity_check` returns "ok"  | `"Database integrity check failed"`             |
| Config file valid                    | `config.toml` parses to `VaultConfig`  | `"Config file is malformed"`                    |
| Orphaned files                       | No dirs in `mcps/` not in DB           | `"Found N orphaned MCP directories"`            |
| Missing files                        | No DB entries missing from disk        | `"Found N MCPs in DB but missing from disk"`    |
| `npm` available                      | `npm --version` succeeds               | `"npm not found — npm installs won't work"`     |
| `npx` available                      | `npx --version` succeeds               | `"npx not found"`                               |
| `uv` available                       | `uv --version` succeeds                | `"uv not found — PyPI installs will use pip"`   |
| `pip` available                      | `pip --version` succeeds               | `"pip not found — PyPI installs won't work"`    |
| `git` available                      | `git --version` succeeds               | `"git not found — GitHub installs won't work"`  |

### Testing Requirements

| Test                                           | Type        | Assertion                                                   |
|------------------------------------------------|-------------|-------------------------------------------------------------|
| Config parse: valid TOML                       | Unit        | Deserializes to `VaultConfig` with correct field values     |
| Config parse: missing optional fields          | Unit        | Uses defaults, no error                                     |
| Config parse: invalid TOML                     | Unit        | Returns `VaultError::Config`                                |
| Config write + read round-trip                 | Unit        | Written config reads back identically                       |
| SQLite CRUD: insert → get MCP                  | Unit        | Retrieved entry matches inserted entry                      |
| SQLite CRUD: list MCPs (empty)                 | Unit        | Returns empty Vec                                           |
| SQLite CRUD: insert → list MCPs                | Unit        | Returns Vec with one entry                                  |
| SQLite CRUD: update MCP                        | Unit        | Updated fields persist                                      |
| SQLite CRUD: delete MCP                        | Unit        | Entry no longer returned by get/list                        |
| SQLite CRUD: duplicate insert                  | Unit        | Returns `VaultError::AlreadyExists`                         |
| SQLite CRUD: get non-existent                  | Unit        | Returns `VaultError::NotFound`                              |
| Same CRUD tests for skills, workflows          | Unit        | Parity with MCP tests                                       |
| Filesystem store: create dirs                  | Unit        | All expected subdirectories exist                           |
| `vault init`: first run                        | Integration | Creates dirs, DB, config.toml; prints success               |
| `vault init`: second run (idempotent)          | Integration | No errors, no data loss, prints "already initialized"       |
| `vault status`: on initialized vault           | Integration | Shows correct counts (0 MCPs, 0 skills, etc.)              |
| `vault doctor`: healthy vault                  | Integration | All checks pass                                             |
| `vault doctor`: missing vault dir              | Integration | Reports failure for vault directory check                   |

### Definition of Done

- [ ] `VaultConfig` loads from, saves to, and round-trips through `config.toml` without data loss
- [ ] All 6 SQLite tables are created on first DB open via migration
- [ ] `SqliteRegistry` passes all CRUD unit tests for MCPs, skills, workflows, agent configs, sync history
- [ ] `vault init` creates `~/.agentvault/` with `config.toml`, `vault.db`, and all subdirectories
- [ ] `vault init` is idempotent — running twice does not corrupt or duplicate data
- [ ] `vault status` displays vault directory path, capability counts, and database size
- [ ] `vault doctor` runs all health checks and reports pass/warn/fail for each
- [ ] All unit and integration tests pass (`cargo test --workspace`)

---

## Phase 2: MCP Management Core

> **Goal:** Users can install, remove, update, and list MCP servers from npm,
> PyPI, GitHub, and local paths. Environment variables and version pinning
> are supported.

**Estimated Time:** 4 days (Days 4–7)

### Scope

| What                    | Detail                                                                           |
|-------------------------|----------------------------------------------------------------------------------|
| MCP data models         | `McpEntry`, `McpSource`, `McpTransport`, `McpConfig`, `McpStatus`                |
| MCP manager             | `McpManager` trait + `DefaultMcpManager` implementation                          |
| npm installer           | Detect npm/npx → `npm install` → parse package.json → register                  |
| PyPI installer          | Detect uv/pip → `uv tool install` or `pip install` → detect entry point          |
| GitHub installer        | `git clone` → detect build system → build → register                             |
| Local installer         | Validate path → symlink → register                                               |
| `vault install`         | Source auto-detection, progress spinner, optional auto-sync                       |
| `vault remove`          | File cleanup + registry deletion                                                 |
| `vault update`          | Single and `--all`, version comparison                                           |
| `vault list`            | `--mcps`, `--skills`, `--workflows`, `--json`, `--table`                         |
| Env var management      | `vault config set/get` per MCP, secret masking                                   |
| Version pinning         | Semver constraints, `--force` override                                           |
| Output formatting       | `output.rs` with rich table rendering, colors, icons                             |

### Files to Create / Modify

| File                                                    | Action | Crate/Module                |
|---------------------------------------------------------|--------|-----------------------------|
| `crates/vault-core/src/mcp/mod.rs`                      | Create | vault-core::mcp             |
| `crates/vault-core/src/mcp/models.rs`                   | Create | vault-core::mcp::models     |
| `crates/vault-core/src/mcp/manager.rs`                  | Create | vault-core::mcp::manager    |
| `crates/vault-core/src/mcp/installer.rs`                | Create | vault-core::mcp::installer  |
| `crates/vault-core/src/mcp/resolver.rs`                 | Create | vault-core::mcp::resolver   |
| `crates/vault-core/src/lib.rs`                          | Modify | vault-core (add `mod mcp`)  |
| `crates/vault-cli/src/commands/install.rs`              | Modify | vault-cli::commands         |
| `crates/vault-cli/src/commands/remove.rs`               | Modify | vault-cli::commands         |
| `crates/vault-cli/src/commands/update.rs`               | Modify | vault-cli::commands         |
| `crates/vault-cli/src/commands/list.rs`                 | Modify | vault-cli::commands         |
| `crates/vault-cli/src/commands/config.rs`               | Modify | vault-cli::commands         |
| `crates/vault-cli/src/output.rs`                        | Create | vault-cli::output           |

### Key Data Models

```rust
// crates/vault-core/src/mcp/models.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEntry {
    pub id: String,              // UUID
    pub name: String,            // e.g., "@anthropic/mcp-server-github"
    pub version: String,         // semver string
    pub source: McpSource,
    pub transport: McpTransport,
    pub config: McpConfig,
    pub env_vars: HashMap<String, String>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub checksum: Option<String>, // SHA-256 of installed package
    pub pinned_version: Option<String>, // semver constraint
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpSource {
    Npm { package: String, version: Option<String> },
    PyPi { package: String, version: Option<String> },
    GitHub { repo: String, git_ref: Option<String> },
    Local { path: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio { command: String, args: Vec<String> },
    Sse { url: String },
    StreamableHttp { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub env_vars: HashMap<String, String>,
    pub extra_args: Vec<String>,
    pub transport_overrides: Option<McpTransport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum McpStatus {
    Installed,
    UpdateAvailable { latest: String },
    Broken { reason: String },
}
```

#### Source Auto-Detection Logic (`vault-core/src/mcp/resolver.rs`)

```
Input string               Detected source
─────────────────────────  ──────────────────────────────────
"npm:@scope/pkg"           McpSource::Npm { package: "@scope/pkg" }
"pip:package-name"         McpSource::PyPi { package: "package-name" }
"github:user/repo"         McpSource::GitHub { repo: "user/repo" }
"github:user/repo@v1.2"   McpSource::GitHub { repo: "user/repo", ref: "v1.2" }
"/absolute/local/path"     McpSource::Local { path }
"./relative/local/path"    McpSource::Local { path (canonicalized) }
"@scope/pkg"               McpSource::Npm { package: "@scope/pkg" }  (npm heuristic)
"bare-name"                McpSource::Npm { package: "bare-name" }   (npm fallback)
```

#### Installer Pipeline (per source type)

**npm install flow:**
1. Verify `npm` or `npx` is on `$PATH`
2. Run `npm install --prefix ~/.agentvault/mcps/<name> <package>@<version>`
3. Parse `<prefix>/node_modules/<package>/package.json` for `bin` field
4. Build `McpTransport::Stdio { command: "npx", args: ["-y", "<package>"] }` or resolve bin path
5. Compute SHA-256 checksum of `package.json`
6. Create `McpEntry` and insert via `Registry`
7. Return success with install summary

**PyPI install flow:**
1. Check for `uv` first (preferred), fall back to `pip`
2. Create venv: `~/.agentvault/mcps/<name>/venv/`
3. Run `uv pip install --prefix ... <package>` or `pip install --target ... <package>`
4. Detect console_scripts entry point in `*.dist-info/entry_points.txt`
5. Build `McpTransport::Stdio { command: "<venv>/bin/<entrypoint>", args: [] }`
6. Create `McpEntry` and insert via `Registry`

**GitHub install flow:**
1. Verify `git` is on `$PATH`
2. Run `git clone <repo> ~/.agentvault/mcps/<name>` (optionally checkout `git_ref`)
3. Detect build system:
   - `package.json` → run `npm install && npm run build`
   - `Cargo.toml` → run `cargo build --release`
   - `pyproject.toml` → run `uv pip install -e .` in venv
4. Detect entry point from build output
5. Create `McpEntry` and insert via `Registry`

**Local install flow:**
1. Validate path exists
2. Create symlink: `~/.agentvault/mcps/<name>` → `<local-path>`
3. Prompt for or auto-detect transport (look for `package.json`, `Cargo.toml`, `pyproject.toml`)
4. Create `McpEntry` with `McpSource::Local` and insert via `Registry`

### Testing Requirements

| Test                                                  | Type        | Assertion                                                              |
|-------------------------------------------------------|-------------|------------------------------------------------------------------------|
| Source auto-detection: npm prefix                     | Unit        | `"npm:@scope/pkg"` → `McpSource::Npm`                                 |
| Source auto-detection: pip prefix                     | Unit        | `"pip:mcp-server"` → `McpSource::PyPi`                                |
| Source auto-detection: github prefix                  | Unit        | `"github:user/repo@v1"` → `McpSource::GitHub` with ref                |
| Source auto-detection: local path                     | Unit        | `"/tmp/my-mcp"` → `McpSource::Local`                                  |
| Source auto-detection: bare name fallback             | Unit        | `"some-package"` → `McpSource::Npm`                                   |
| McpEntry serialization round-trip                     | Unit        | JSON serialize → deserialize produces identical struct                 |
| Install from local path                               | Integration | Creates symlink, registry entry exists, `vault list` shows it         |
| Remove installed MCP                                  | Integration | Removes files and registry entry, `vault list` doesn't show it        |
| Update installed MCP                                  | Integration | Version changes in registry, files updated on disk                    |
| `vault list --json`                                   | Integration | Output is valid JSON array                                            |
| `vault list --table`                                  | Integration | Output contains formatted table headers                               |
| `vault list --mcps` (empty)                           | Integration | Prints "No MCPs installed" or empty table                             |
| Version constraint enforcement                        | Unit        | `^1.0` blocks install of `2.0.0`                                      |
| `--force` overrides version constraint                | Unit        | Install proceeds despite constraint violation                         |
| Env var set and get                                   | Integration | `vault config set mcp KEY VAL` → `vault config get mcp` shows `KEY`  |
| Secret masking in output                              | Unit        | API key values display as `****`                                       |

### Definition of Done

- [ ] All four install sources (npm, PyPI, GitHub, local) work end-to-end
- [ ] Source auto-detection correctly resolves all documented input patterns
- [ ] `vault install <source>` shows a progress spinner and prints install summary on success
- [ ] `vault remove <name>` deletes both files and registry entry
- [ ] `vault update <name>` re-installs latest version and updates registry
- [ ] `vault update --all` iterates all MCPs and updates each
- [ ] `vault list` with `--mcps`, `--json`, `--table` flags all produce correct output
- [ ] Env vars per MCP: set, get, and mask secrets in output
- [ ] Version pinning: semver constraints enforced, `--force` bypasses
- [ ] All unit and integration tests pass (`cargo test --workspace`)
- [ ] No panics — all error paths return `VaultError` variants

---

## Phase 3: Agent Connectors

> **Goal:** AgentVault can sync installed MCPs to Claude Code, Gemini CLI,
> OpenCode, and Codex CLI configs. Sync is safe (backups), diffable (dry-run),
> and auditable (sync history).

**Estimated Time:** 4 days (Days 8–11)

### Scope

| What                    | Detail                                                                    |
|-------------------------|---------------------------------------------------------------------------|
| `AgentConnector` trait  | Generic interface for reading/writing/syncing agent configs               |
| Claude Code connector   | Parse + write `claude_desktop_config.json`                                |
| Gemini CLI connector    | Parse + write `settings.json`                                             |
| OpenCode connector      | Research format, implement read/write                                     |
| Codex CLI connector     | Research format, implement read/write                                     |
| `vault sync`            | Single agent, `--all`, `--dry-run`                                        |
| `vault connector`       | `add`, `list`, `remove` subcommands                                       |
| Sync history            | Log every sync to SQLite, display in `vault status`                       |

### Files to Create / Modify

| File                                                    | Action | Crate/Module                    |
|---------------------------------------------------------|--------|---------------------------------|
| `crates/vault-connectors/src/traits.rs`                 | Create | vault-connectors::traits        |
| `crates/vault-connectors/src/types.rs`                  | Create | vault-connectors::types         |
| `crates/vault-connectors/src/claude.rs`                 | Create | vault-connectors::claude        |
| `crates/vault-connectors/src/gemini.rs`                 | Create | vault-connectors::gemini        |
| `crates/vault-connectors/src/opencode.rs`               | Create | vault-connectors::opencode      |
| `crates/vault-connectors/src/codex.rs`                  | Create | vault-connectors::codex         |
| `crates/vault-connectors/src/sync.rs`                   | Create | vault-connectors::sync          |
| `crates/vault-connectors/src/lib.rs`                    | Modify | vault-connectors (re-exports)   |
| `crates/vault-cli/src/commands/sync.rs`                 | Modify | vault-cli::commands             |
| `crates/vault-cli/src/commands/connector.rs`            | Modify | vault-cli::commands             |
| `crates/vault-cli/src/commands/status.rs`               | Modify | vault-cli::commands (add sync info) |

### `AgentConnector` Trait

```rust
// crates/vault-connectors/src/traits.rs

#[async_trait]
pub trait AgentConnector: Send + Sync {
    /// Identifier for this agent type (e.g., "claude", "gemini")
    fn agent_type(&self) -> &str;

    /// Path to the agent's configuration file
    fn config_path(&self) -> &Path;

    /// Read the current agent configuration from disk
    fn read_config(&self) -> Result<AgentMcpConfig, VaultError>;

    /// Write an updated configuration to disk (atomically)
    fn write_config(&self, config: &AgentMcpConfig) -> Result<(), VaultError>;

    /// Compute the diff between current agent config and desired vault state
    fn diff(&self, mcps: &[McpEntry]) -> Result<ConfigDiff, VaultError>;

    /// Apply vault state to the agent's config file
    fn sync(&self, mcps: &[McpEntry]) -> Result<SyncResult, VaultError>;

    /// Create a timestamped backup of the current config
    fn backup(&self, backup_dir: &Path) -> Result<PathBuf, VaultError>;

    /// Verify the written config is valid and parseable
    fn verify(&self) -> Result<bool, VaultError>;
}
```

### Connector Implementation Details

#### Claude Code (`vault-connectors/src/claude.rs`)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| Config path     | `~/.claude/claude_desktop_config.json`            |
| Config format   | JSON                                              |
| MCP section     | `$.mcpServers`                                    |

**MCP entry format (Claude):**
```json
{
  "mcpServers": {
    "<mcp-name>": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-github"],
      "env": {
        "GITHUB_TOKEN": "..."
      }
    }
  }
}
```

**Mapping: `McpEntry` → Claude JSON:**
- `entry.name` → object key under `mcpServers`
- `entry.transport` (Stdio) → `command` + `args`
- `entry.env_vars` → `env` object
- Non-vault entries (not in registry) are preserved during sync

#### Gemini CLI (`vault-connectors/src/gemini.rs`)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| Config path     | `~/.gemini/config/settings.json`                  |
| Config format   | JSON                                              |
| MCP section     | `$.mcpServers`                                    |

**MCP entry format (Gemini CLI):**
```json
{
  "mcpServers": {
    "<mcp-name>": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-server-github"],
      "env": {
        "GITHUB_TOKEN": "..."
      }
    }
  }
}
```

**Mapping:** Same structure as Claude (both use `mcpServers` JSON format). The connector differs in config path, backup location, and any Gemini-specific fields.

#### OpenCode (`vault-connectors/src/opencode.rs`)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| Config path     | `~/.config/opencode/config.json` (research needed)|
| Config format   | JSON (research needed)                            |
| MCP section     | TBD — requires research during implementation     |

> **Action Item:** Research OpenCode's MCP configuration format and location
> before implementing. Check `opencode --help`, source repo docs, and
> example configs. If format is not publicly documented, implement a
> stub connector with `unimplemented!()` and file an issue.

#### Codex CLI (`vault-connectors/src/codex.rs`)

| Property        | Value                                             |
|-----------------|---------------------------------------------------|
| Config path     | `~/.codex/config.json` (research needed)          |
| Config format   | JSON (research needed)                            |
| MCP section     | TBD — requires research during implementation     |

> **Action Item:** Same as OpenCode — research the Codex CLI config format.

### Sync Engine (`vault-connectors/src/sync.rs`)

```rust
pub struct SyncEngine {
    registry: Arc<dyn Registry>,
    backup_dir: PathBuf,
}

impl SyncEngine {
    /// Sync vault state to a single agent
    pub fn sync_agent(&self, connector: &dyn AgentConnector) -> Result<SyncResult, VaultError>;

    /// Sync vault state to all registered agents
    pub fn sync_all(&self, connectors: &[Box<dyn AgentConnector>]) -> Result<Vec<SyncResult>, VaultError>;

    /// Preview changes without writing (dry-run)
    pub fn dry_run(&self, connector: &dyn AgentConnector) -> Result<ConfigDiff, VaultError>;
}
```

**Sync algorithm:**
1. `connector.backup(backup_dir)` — always backup first
2. `connector.read_config()` — read current agent state
3. Compute diff: MCPs in vault but not in agent (to add), MCPs in both but different (to update), MCPs managed by vault but removed from vault (to remove)
4. Merge: add/update vault-managed entries, preserve non-vault entries
5. `connector.write_config(&merged)` — atomic write
6. `connector.verify()` — validate the written config
7. Log to `sync_history` table

### `ConfigDiff` Structure

```rust
pub struct ConfigDiff {
    pub added: Vec<McpEntry>,      // In vault, not in agent
    pub updated: Vec<(McpEntry, McpEntry)>,  // (old, new)
    pub removed: Vec<String>,       // Was vault-managed, now removed from vault
    pub unchanged: Vec<String>,     // Same in both
    pub unmanaged: Vec<String>,     // In agent, not managed by vault (preserved)
}
```

### Testing Requirements

| Test                                                    | Type        | Assertion                                                             |
|---------------------------------------------------------|-------------|-----------------------------------------------------------------------|
| Claude connector: read valid config                     | Unit        | Parses all `mcpServers` entries correctly                             |
| Claude connector: write config                          | Unit        | Written JSON is valid and contains expected entries                   |
| Claude connector: round-trip read → write → read        | Integration | Config is identical after round-trip                                  |
| Claude connector: preserves non-vault entries           | Integration | Entries not in vault registry are untouched after sync                |
| Gemini connector: read valid config                     | Unit        | Same as Claude but for Gemini path/format                             |
| Gemini connector: write config                          | Unit        | Same as Claude                                                        |
| Backup creation                                         | Unit        | Backup file exists at expected path with correct content              |
| Backup file naming                                      | Unit        | Includes timestamp in filename                                        |
| Atomic write                                            | Unit        | Partial write failure leaves original file intact                     |
| Dry-run produces diff without file changes              | Integration | Config file modification time unchanged after dry-run                 |
| Sync with empty vault → removes vault-managed entries   | Integration | Only vault-managed entries removed; unmanaged entries preserved       |
| Sync history logged to SQLite                           | Integration | `sync_history` table has entry after sync                             |
| `vault connector add claude`                            | Integration | Adds connector to `agent_configs` table                               |
| `vault connector list`                                  | Integration | Shows all registered connectors                                       |
| `vault connector remove claude`                         | Integration | Removes from `agent_configs`, does NOT delete agent config file       |

### Definition of Done

- [ ] `AgentConnector` trait is defined and documented
- [ ] Claude Code connector reads and writes `claude_desktop_config.json` correctly
- [ ] Gemini CLI connector reads and writes `settings.json` correctly
- [ ] OpenCode connector implemented (or documented as stub with reason)
- [ ] Codex CLI connector implemented (or documented as stub with reason)
- [ ] `vault sync claude` writes vault MCPs to Claude config, preserving non-vault entries
- [ ] `vault sync --all` syncs to all registered connectors
- [ ] `vault sync --dry-run` shows diff without modifying any file
- [ ] Every sync creates a timestamped backup in `~/.agentvault/backups/<agent>/`
- [ ] Config writes are atomic (temp file → rename)
- [ ] Sync history is logged to SQLite after every sync operation
- [ ] `vault connector add/list/remove` all work correctly
- [ ] `vault status` shows last sync time per registered agent
- [ ] All unit and integration tests pass

---

## Phase 4: Search & Discovery

> **Goal:** Users can search for MCP servers across local registry and npm.
> Results are displayed in rich, colored terminal output.

**Estimated Time:** 2 days (Days 12–13)

### Scope

| What                    | Detail                                                             |
|-------------------------|--------------------------------------------------------------------|
| Local search            | Fuzzy name/tag/description matching against SQLite registry        |
| npm search              | Query `registry.npmjs.org` search API                              |
| `vault search`          | Unified command with `--local`, `--npm`, `--limit` flags           |
| Rich output             | Colored tables, status icons, padded columns                       |

### Files to Create / Modify

| File                                                    | Action | Crate/Module              |
|---------------------------------------------------------|--------|---------------------------|
| `crates/vault-core/src/search.rs`                       | Create | vault-core::search        |
| `crates/vault-core/src/lib.rs`                          | Modify | vault-core (add `mod search`) |
| `crates/vault-cli/src/commands/search.rs`               | Modify | vault-cli::commands       |
| `crates/vault-cli/src/output.rs`                        | Modify | vault-cli::output         |

### Search Implementation

#### Local Search (`vault-core/src/search.rs`)

```rust
pub struct SearchEngine {
    registry: Arc<dyn Registry>,
}

pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub source: SearchSource,
    pub installed: bool,        // Already in local vault?
    pub relevance_score: f64,   // 0.0 to 1.0
}

pub enum SearchSource {
    Local,
    Npm,
}

impl SearchEngine {
    /// Search local registry by name, tags, and description
    pub fn search_local(&self, query: &str) -> Result<Vec<SearchResult>, VaultError>;

    /// Search npm registry via HTTP API
    pub async fn search_npm(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, VaultError>;

    /// Combined search: local first, then npm, deduplicated
    pub async fn search_all(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, VaultError>;
}
```

**Local search algorithm:**
1. Query all MCPs, skills, workflows from registry
2. For each, compute relevance score:
   - Exact name match → 1.0
   - Name contains query (case-insensitive) → 0.8
   - Tag exact match → 0.7
   - Description contains query → 0.5
3. Filter results with score > 0.3
4. Sort by score descending

**npm search API:**
- Endpoint: `https://registry.npmjs.org/-/v1/search?text={query}&size={limit}`
- Parse response: extract `objects[].package.{name, version, description, keywords}`
- Filter by MCP-related keywords heuristic: `"mcp"`, `"model-context-protocol"`, `"mcp-server"`
- Mark `installed: true` if name exists in local registry

#### Rich Terminal Output (`vault-cli/src/output.rs`)

| Output Element       | Implementation                                   |
|----------------------|--------------------------------------------------|
| Table formatting     | `tabled` crate with custom style                 |
| Colors               | `owo-colors` — green for installed, yellow for update, dim for metadata |
| Status icons         | `✓` installed, `↓` available for download, `⚠` broken |
| Progress spinners    | `indicatif` for network requests                 |
| Padded columns       | Right-align version, left-align name             |

### Testing Requirements

| Test                                         | Type        | Assertion                                                    |
|----------------------------------------------|-------------|--------------------------------------------------------------|
| Local search: exact match                    | Unit        | Returns entry with score ≈ 1.0                               |
| Local search: partial match                  | Unit        | Returns entry with score > 0.5                               |
| Local search: no match                       | Unit        | Returns empty vec                                            |
| Local search: tag match                      | Unit        | Entry with matching tag appears in results                   |
| npm search: parse valid response             | Unit        | Correctly extracts name, version, description                |
| npm search: empty response                   | Unit        | Returns empty vec                                            |
| npm search: network error                    | Unit        | Returns `VaultError::Network` with helpful message           |
| Combined search: local before npm            | Unit        | Local results appear first in combined results               |
| Combined search: deduplication               | Unit        | Same package in local + npm appears once (marked installed)  |
| Output formatting: table headers             | Unit        | Output contains column headers                               |

### Definition of Done

- [ ] `vault search <query>` searches local registry and returns relevant results
- [ ] `vault search <query> --npm` queries npm registry and displays results
- [ ] `vault search <query> --local` restricts to local-only search
- [ ] `vault search <query> --limit 5` caps results to 5
- [ ] Search results display in a formatted table with name, version, source, description
- [ ] Installed MCPs show `✓` icon, available ones show `↓`
- [ ] Network errors during npm search display a user-friendly error (not a panic)
- [ ] All unit tests pass

---

## Phase 5: Manifest & Declarative Config

> **Goal:** Users can export their vault state to `vault.toml` and import it
> on another machine or share it with a team. Enables reproducible setups.

**Estimated Time:** 2 days (Days 14–15)

### Scope

| What                    | Detail                                                                 |
|-------------------------|------------------------------------------------------------------------|
| Manifest format         | `vault.toml` with `[vault]`, `[[mcp]]`, `[[skill]]`, `[[workflow]]`   |
| Parser                  | `VaultManifest` struct with full validation                            |
| `vault export`          | Registry → `vault.toml`                                               |
| `vault import`          | `vault.toml` → install all, with `--dry-run`, `--merge`, `--replace`, `--prune` |
| Diff                    | Compare manifest against current state                                 |

### Files to Create / Modify

| File                                                    | Action | Crate/Module              |
|---------------------------------------------------------|--------|---------------------------|
| `crates/vault-core/src/manifest.rs`                     | Create | vault-core::manifest      |
| `crates/vault-core/src/lib.rs`                          | Modify | vault-core (add `mod manifest`) |
| `crates/vault-cli/src/commands/export.rs`               | Modify | vault-cli::commands       |
| `crates/vault-cli/src/commands/import.rs`               | Modify | vault-cli::commands       |

### `vault.toml` Format

```toml
[vault]
name = "my-agent-setup"
version = "1.0.0"
description = "My team's shared MCP server configuration"

[[mcp]]
name = "@anthropic/mcp-server-github"
source = "npm:@anthropic/mcp-server-github"
version = "^0.6"
[mcp.env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"    # Reference env var, don't store secrets

[[mcp]]
name = "mcp-server-filesystem"
source = "npm:@anthropic/mcp-server-filesystem"
version = "^0.6"

[[mcp]]
name = "custom-mcp"
source = "github:myorg/custom-mcp@main"

[[skill]]
name = "code-review"
source = "github:myorg/code-review-skill"

[[workflow]]
name = "deploy-pipeline"
source = "local:./workflows/deploy.toml"

[agents]
sync = ["claude", "gemini"]
```

### `VaultManifest` Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultManifest {
    pub vault: ManifestMeta,
    #[serde(default)]
    pub mcp: Vec<ManifestMcp>,
    #[serde(default)]
    pub skill: Vec<ManifestSkill>,
    #[serde(default)]
    pub workflow: Vec<ManifestWorkflow>,
    #[serde(default)]
    pub agents: Option<ManifestAgents>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMeta {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMcp {
    pub name: String,
    pub source: String,
    pub version: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSkill {
    pub name: String,
    pub source: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestWorkflow {
    pub name: String,
    pub source: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAgents {
    pub sync: Vec<String>,
}
```

### Import Modes

| Flag          | Behavior                                                                    |
|---------------|-----------------------------------------------------------------------------|
| `--merge`     | Install missing, update changed, keep existing not in manifest (default)    |
| `--replace`   | Install missing, update changed, remove existing not in manifest            |
| `--prune`     | Same as `--replace` (alias)                                                 |
| `--dry-run`   | Show what would change without modifying anything                           |

### Manifest Diff

```rust
pub struct ManifestDiff {
    pub to_install: Vec<ManifestMcp>,    // In manifest, not in vault
    pub to_update: Vec<(McpEntry, ManifestMcp)>,  // In both, version changed
    pub to_remove: Vec<McpEntry>,        // In vault, not in manifest (only with --prune)
    pub unchanged: Vec<String>,          // Same in both
}
```

### Testing Requirements

| Test                                              | Type        | Assertion                                                          |
|---------------------------------------------------|-------------|--------------------------------------------------------------------|
| Manifest parse: valid vault.toml                  | Unit        | Deserializes to `VaultManifest` with all fields                    |
| Manifest parse: missing optional fields           | Unit        | Uses defaults, no error                                            |
| Manifest parse: invalid TOML                      | Unit        | Returns `VaultError::Config`                                       |
| Manifest parse: invalid semver constraint         | Unit        | Returns validation error                                           |
| Manifest parse: invalid source string             | Unit        | Returns validation error                                           |
| Export → Import round-trip                        | Integration | Exporting then importing on empty vault produces matching state    |
| Import `--merge`: keeps extra MCPs                | Integration | MCPs in vault but not in manifest are preserved                   |
| Import `--prune`: removes extra MCPs              | Integration | MCPs in vault but not in manifest are removed                     |
| Import `--dry-run`: no modifications              | Integration | Vault state unchanged, diff printed                               |
| Import idempotent: running twice                  | Integration | Second import produces no changes                                  |
| Partial failure: some installs fail               | Integration | Successful installs persist; failures reported with details        |

### Definition of Done

- [ ] `vault.toml` format is defined and documented
- [ ] `VaultManifest` parser validates all fields and reports clear errors for malformed manifests
- [ ] `vault export` serializes current vault state to `vault.toml` correctly
- [ ] `vault export --output ./my-setup.toml` writes to custom path
- [ ] `vault import vault.toml` installs all declared capabilities
- [ ] `vault import --dry-run` previews changes without writing
- [ ] `vault import --merge` preserves existing capabilities not in manifest
- [ ] `vault import --prune` removes capabilities not in manifest
- [ ] Round-trip: export → import on fresh vault produces identical state
- [ ] All tests pass

---

## Phase 6: Skills & Workflows

> **Goal:** AgentVault manages more than MCPs — it also installs skills (SKILL.md-based
> directories) and workflows (multi-step definitions with dependency resolution).

**Estimated Time:** 3 days (Days 16–18)

### Scope

| What                    | Detail                                                                   |
|-------------------------|--------------------------------------------------------------------------|
| Skill models            | `SkillEntry`, `SkillSource`, `SkillManager` trait                        |
| Skill install           | From git repo, local path                                                |
| Workflow models         | `WorkflowEntry`, `WorkflowStep`, `WorkflowManager` trait                 |
| Workflow parsing        | `workflow.toml` definition format                                        |
| Dependency resolution   | Topological sort, missing dependency detection                           |
| CLI extension           | Extend `vault install/remove/list` for skills and workflows              |

### Files to Create / Modify

| File                                                    | Action | Crate/Module                     |
|---------------------------------------------------------|--------|----------------------------------|
| `crates/vault-core/src/skill/mod.rs`                    | Create | vault-core::skill                |
| `crates/vault-core/src/skill/models.rs`                 | Create | vault-core::skill::models        |
| `crates/vault-core/src/skill/manager.rs`                | Create | vault-core::skill::manager       |
| `crates/vault-core/src/workflow/mod.rs`                  | Create | vault-core::workflow             |
| `crates/vault-core/src/workflow/models.rs`               | Create | vault-core::workflow::models     |
| `crates/vault-core/src/workflow/manager.rs`              | Create | vault-core::workflow::manager    |
| `crates/vault-core/src/workflow/resolver.rs`             | Create | vault-core::workflow::resolver   |
| `crates/vault-core/src/lib.rs`                          | Modify | vault-core (add mods)            |
| `crates/vault-cli/src/commands/install.rs`              | Modify | vault-cli::commands              |
| `crates/vault-cli/src/commands/remove.rs`               | Modify | vault-cli::commands              |
| `crates/vault-cli/src/commands/list.rs`                 | Modify | vault-cli::commands              |

### Data Models

```rust
// crates/vault-core/src/skill/models.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: SkillSource,
    pub path: PathBuf,          // Installed location on disk
    pub tags: Vec<String>,
    pub description: String,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillSource {
    Git { repo: String, git_ref: Option<String> },
    Local { path: PathBuf },
}
```

```rust
// crates/vault-core/src/workflow/models.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: WorkflowSource,
    pub steps: Vec<WorkflowStep>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub uses: String,           // Capability reference: "mcp:name" or "skill:name"
    pub args: HashMap<String, String>,
    pub depends_on: Vec<String>, // Names of other steps this depends on
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowSource {
    Git { repo: String, git_ref: Option<String> },
    Local { path: PathBuf },
}
```

### `workflow.toml` Format

```toml
[workflow]
name = "code-review-pipeline"
version = "1.0.0"
description = "Automated code review using multiple MCP servers"

[[step]]
name = "fetch-pr"
uses = "mcp:github-mcp"
[step.args]
action = "get_pull_request"
repo = "${REPO}"
pr_number = "${PR_NUMBER}"

[[step]]
name = "analyze-code"
uses = "mcp:code-analysis-mcp"
depends_on = ["fetch-pr"]
[step.args]
diff = "${fetch-pr.output.diff}"

[[step]]
name = "post-review"
uses = "mcp:github-mcp"
depends_on = ["analyze-code"]
condition = "analyze-code.output.issues > 0"
[step.args]
action = "create_review"
body = "${analyze-code.output.review}"
```

### Dependency Resolution (`vault-core/src/workflow/resolver.rs`)

```rust
pub struct DependencyResolver;

impl DependencyResolver {
    /// Topological sort of workflow steps.
    /// Returns steps in execution order.
    /// Returns VaultError if circular dependency detected.
    pub fn resolve(steps: &[WorkflowStep]) -> Result<Vec<&WorkflowStep>, VaultError>;

    /// Check that all capabilities referenced by steps are installed in the vault.
    /// Returns list of missing capabilities.
    pub fn check_dependencies(
        steps: &[WorkflowStep],
        registry: &dyn Registry,
    ) -> Result<Vec<String>, VaultError>;
}
```

**Algorithm:** Kahn's algorithm for topological sort (BFS-based, O(V+E))

### CLI Extension Points

| Command                          | Change                                                         |
|----------------------------------|----------------------------------------------------------------|
| `vault install --skill <src>`    | Installs a skill from git or local path                        |
| `vault install --workflow <src>` | Installs a workflow from git or local path                     |
| `vault remove <name>`           | Auto-detects capability type (MCP, skill, workflow) by name    |
| `vault list --skills`           | Lists installed skills with name, version, path, tags          |
| `vault list --workflows`        | Lists installed workflows with name, version, step count       |
| `vault list --all`              | Lists MCPs, skills, and workflows grouped by type              |

### Testing Requirements

| Test                                                     | Type        | Assertion                                                       |
|----------------------------------------------------------|-------------|-----------------------------------------------------------------|
| Skill install from local path                            | Integration | SKILL.md parsed, entry in DB, files on disk                     |
| Skill install: validate SKILL.md exists                  | Unit        | Error if SKILL.md not found in skill directory                  |
| Skill install: parse YAML frontmatter                    | Unit        | Correctly extracts name, description, tags from SKILL.md        |
| Skill remove                                             | Integration | Entry removed from DB, files removed from disk                  |
| Workflow install from local path                         | Integration | workflow.toml parsed, entry in DB                               |
| Workflow parsing: valid workflow.toml                     | Unit        | Deserializes all steps with correct fields                      |
| Workflow parsing: invalid TOML                           | Unit        | Returns error                                                   |
| Dependency resolution: valid DAG                         | Unit        | Returns steps in correct topological order                      |
| Dependency resolution: circular dependency               | Unit        | Returns `VaultError` with cycle description                     |
| Dependency resolution: missing step reference            | Unit        | Returns error listing unresolved dependencies                   |
| Check dependencies: all present                          | Unit        | Returns empty missing list                                      |
| Check dependencies: some missing                         | Unit        | Returns names of missing capabilities                           |
| `vault list --skills`                                    | Integration | Shows installed skills                                          |
| `vault list --workflows`                                 | Integration | Shows installed workflows                                       |
| `vault list --all`                                       | Integration | Shows all capability types                                      |

### Definition of Done

- [ ] `SkillEntry` and `SkillManager` fully implemented
- [ ] Skills install from git (clone + parse SKILL.md) and from local path (symlink + parse)
- [ ] `WorkflowEntry` and `WorkflowManager` fully implemented
- [ ] `workflow.toml` parsing handles all fields including `depends_on` and `condition`
- [ ] Topological sort correctly orders steps and detects circular dependencies
- [ ] `check_dependencies` validates all referenced capabilities exist in vault
- [ ] `vault install --skill`, `vault install --workflow` work end-to-end
- [ ] `vault remove` auto-detects and removes skills and workflows
- [ ] `vault list --skills`, `--workflows`, `--all` display correct data
- [ ] All tests pass

---

## Phase 7: Polish & Release

> **Goal:** AgentVault is production-ready: polished UX, shell completions,
> man pages, optimized performance, full test suite, cross-platform binaries,
> README, install script, and a tagged `v0.1.0` release.

**Estimated Time:** 3 days (Days 19–21)

### Scope

| What                    | Detail                                                                   |
|-------------------------|--------------------------------------------------------------------------|
| Error polish            | User-friendly messages with contextual suggestions                       |
| Help text               | `long_about` and `after_help` for every command                          |
| Shell completions       | bash, zsh, fish, powershell via `clap_complete`                          |
| Man pages               | Generated from clap via `clap_mangen`                                    |
| Performance             | Lazy loading, SQLite WAL mode, parallel updates                          |
| Integration tests       | End-to-end + edge cases + error paths                                    |
| Binary release          | Cross-compiled for 5 targets via GitHub Actions                          |
| README                  | Full docs: install, quickstart, command reference, config reference      |
| Install script          | `install.sh` for curl-pipe installs                                      |
| Tag v0.1.0              | Version bump, changelog, git tag, release workflow trigger               |

### Files to Create / Modify

| File                                                    | Action | Crate/Module                       |
|---------------------------------------------------------|--------|------------------------------------|
| `crates/vault-core/src/error.rs`                        | Modify | vault-core::error (polish messages)|
| `crates/vault-cli/src/main.rs`                          | Modify | vault-cli (lazy init, completions) |
| `crates/vault-cli/src/commands/*.rs`                    | Modify | All commands (help text)           |
| `crates/vault-cli/src/commands/completions.rs`          | Create | vault-cli::commands                |
| `crates/vault-cli/Cargo.toml`                           | Modify | Add clap_complete, clap_mangen     |
| `crates/vault-cli/build.rs`                             | Create | Man page generation at build time  |
| `.github/workflows/release.yml`                         | Create | CI — binary release workflow       |
| `README.md`                                             | Create | Root                               |
| `install.sh`                                            | Create | Root                               |
| `CHANGELOG.md` (root)                                   | Modify | Root (v0.1.0 entry)                |
| `tests/integration/*.rs`                                | Create | Integration test suite             |

### Error Message Polish

Every `VaultError` variant should have:
1. A clear, jargon-free message describing what went wrong
2. A suggestion for how to fix it (where applicable)

| Error Variant      | User-Facing Message                                               | Suggestion                                        |
|--------------------|-------------------------------------------------------------------|---------------------------------------------------|
| `Io`               | `"Failed to {action}: {detail}"`                                  | `"Check file permissions and disk space"`         |
| `Database`         | `"Database operation failed: {detail}"`                           | `"Run 'vault doctor' to check database health"`   |
| `Config`           | `"Configuration error: {message}"`                                | `"Run 'vault init' to regenerate defaults"`       |
| `Network`          | `"Network request failed: {detail}"`                              | `"Check your internet connection and try again"`  |
| `Connector`        | `"Agent connector '{agent}' error: {message}"`                    | `"Verify agent config file exists and is valid"`  |
| `McpInstall`       | `"Failed to install from {source}: {message}"`                    | `"Run 'vault doctor' to verify prerequisites"`    |
| `NotFound`         | `"{kind} '{name}' not found in vault"`                            | `"Run 'vault list' to see installed items"`       |
| `AlreadyExists`    | `"{kind} '{name}' is already installed"`                          | `"Use 'vault update' to update it"`               |
| `VersionConflict`  | `"Version conflict for '{name}': wanted {w}, have {f}"`           | `"Use '--force' to override"`                     |

### Shell Completions

```rust
// crates/vault-cli/src/commands/completions.rs

#[derive(Debug, Clone, clap::Args)]
pub struct CompletionsArgs {
    /// The shell to generate completions for
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

pub fn run(args: &CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(
        args.shell,
        &mut cmd,
        "vault",
        &mut std::io::stdout(),
    );
    Ok(())
}
```

### Performance Optimizations

| Optimization                       | Implementation                                                       |
|------------------------------------|----------------------------------------------------------------------|
| Lazy SQLite connection             | Don't open DB until a command actually needs it (`--help` stays fast) |
| SQLite WAL mode                    | `PRAGMA journal_mode=WAL;` on connection open                        |
| Parallel updates                   | `vault update --all` uses `tokio::spawn` per MCP                     |
| Connection pooling                 | Single connection per command invocation (re-use across operations)   |
| Avoid unnecessary clones           | Use `&str` and references where possible in hot paths                |

### Binary Release Targets

| Target                        | OS      | Architecture | Build Tool   |
|-------------------------------|---------|--------------|--------------|
| `x86_64-unknown-linux-gnu`    | Linux   | x86_64       | `cross`      |
| `aarch64-unknown-linux-gnu`   | Linux   | aarch64      | `cross`      |
| `x86_64-apple-darwin`         | macOS   | x86_64       | native       |
| `aarch64-apple-darwin`        | macOS   | aarch64      | native       |
| `x86_64-pc-windows-msvc`     | Windows | x86_64       | native       |

### `install.sh` Script Outline

```bash
#!/bin/bash
set -euo pipefail

REPO="AswinkumarGP/AgentVault"
BINARY="vault"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

# Map to release target names
# ...

# Download binary from GitHub Releases
curl -fsSL "https://github.com/$REPO/releases/latest/download/$BINARY-$TARGET.tar.gz" -o /tmp/vault.tar.gz

# Verify checksum
curl -fsSL "https://github.com/$REPO/releases/latest/download/checksums.sha256" -o /tmp/checksums.sha256
cd /tmp && sha256sum -c --ignore-missing checksums.sha256

# Install to ~/.local/bin
mkdir -p "$HOME/.local/bin"
tar xzf /tmp/vault.tar.gz -C "$HOME/.local/bin"
chmod +x "$HOME/.local/bin/vault"

echo "✓ Installed vault to ~/.local/bin/vault"
echo "  Make sure ~/.local/bin is in your PATH"
```

### Integration Test Suite

| Test Scenario                                              | Description                                                         |
|------------------------------------------------------------|---------------------------------------------------------------------|
| E2E: init → install → sync → export → import              | Full lifecycle on temp vault                                         |
| E2E: install MCP → remove → verify clean                  | Install, remove, verify no orphaned files/entries                   |
| Edge case: empty vault list                                | `vault list` on empty vault prints helpful message                  |
| Edge case: duplicate install                               | Second `vault install` of same MCP returns `AlreadyExists`          |
| Edge case: remove non-existent                             | `vault remove nonexistent` returns `NotFound`                       |
| Error path: network failure (npm search)                   | Graceful error, no panic                                             |
| Error path: permission denied (config write)               | Graceful error, no panic, helpful suggestion                        |
| Error path: corrupt DB                                     | `vault doctor` detects and reports                                  |

### README Structure

```
README.md
├── Logo / Banner
├── What is AgentVault?
├── Why AgentVault?
├── Features
├── Quick Start
│   ├── Install
│   ├── Initialize
│   ├── Install MCPs
│   ├── Sync to Agents
│   └── Export / Import
├── Command Reference
│   ├── vault init
│   ├── vault install
│   ├── vault remove
│   ├── vault update
│   ├── vault list
│   ├── vault search
│   ├── vault sync
│   ├── vault status
│   ├── vault doctor
│   ├── vault config
│   ├── vault connector
│   ├── vault export
│   ├── vault import
│   └── vault completions
├── Configuration Reference
│   ├── config.toml
│   └── vault.toml
├── Supported Agents
│   ├── Claude Code
│   ├── Gemini CLI
│   ├── OpenCode
│   └── Codex CLI
├── Architecture
├── Contributing
├── License
└── Acknowledgments
```

### Testing Requirements (Phase 7 Specific)

| Test                                        | Type        | Assertion                                                    |
|---------------------------------------------|-------------|--------------------------------------------------------------|
| All error variants produce user-friendly output | Unit    | No raw panic messages, all have Display impl                |
| Help text: `vault --help`                   | Smoke       | Contains all subcommand names and descriptions              |
| Help text: every `vault <cmd> --help`       | Smoke       | Contains `long_about` and at least one example              |
| Shell completions: bash                     | Smoke       | `vault completions bash` outputs valid bash script          |
| Shell completions: zsh                      | Smoke       | `vault completions zsh` outputs valid zsh script            |
| Lazy loading: `vault --help`               | Performance | Returns in < 50ms (no DB open)                              |
| E2E integration suite                       | Integration | All scenarios pass                                           |

### Definition of Done

- [ ] Every error message is user-friendly with contextual suggestion
- [ ] Every command has `long_about` with usage examples
- [ ] `vault completions bash|zsh|fish|powershell` generates correct scripts
- [ ] Man pages generated via `clap_mangen` and included in release
- [ ] SQLite uses WAL mode; `vault --help` returns in < 100ms (no DB touch)
- [ ] Full integration test suite passes on all three OS targets
- [ ] GitHub Actions release workflow produces binaries for all 5 targets
- [ ] All binaries include SHA-256 checksums
- [ ] `README.md` contains install instructions, quickstart, full command reference
- [ ] `install.sh` downloads and installs correct binary for current OS/arch
- [ ] All `Cargo.toml` files updated to version `0.1.0`
- [ ] `CHANGELOG.md` updated with v0.1.0 entry
- [ ] Git tag `v0.1.0` created and pushed
- [ ] CI release workflow triggers and completes successfully
- [ ] Binary runs on a clean machine (no Rust toolchain required)

---

## Post-MVP Phases (Outline)

These phases are not scoped at the same level of detail. They serve as a roadmap
for prioritized work after `v0.1.0` ships.

### Phase 8: Capability Abstraction (v0.2.0)

> Unified `Capability` type that wraps MCPs, skills, and workflows. Enables
> treating all capability types uniformly in the registry, search, and sync.

- Unified `Capability` enum wrapping `McpEntry | SkillEntry | WorkflowEntry`
- Generic `CapabilityManager` trait
- Cross-capability dependency resolution (workflow step uses an MCP + a skill)
- Refactor `vault install/remove/list` to use `Capability` type
- **Files:** `crates/vault-core/src/capability.rs`
- **Depends on:** Phase 6 (skills & workflows)

### Phase 9: Remote Registry (v0.2.0)

> A community registry where users can discover and share MCP server configs,
> skills, and workflows.

- Registry server (REST API or static GitHub-based registry)
- `vault publish <name>` — publish a capability to the registry
- `vault search --registry` — search the remote registry
- Rating/download count display
- Checksum verification on download
- **Files:** `crates/vault-core/src/remote_registry.rs`, `crates/vault-cli/src/commands/publish.rs`
- **Depends on:** Phase 4 (search infrastructure)

### Phase 10: More Connectors (v0.2.0)

> Expand agent support to Cursor, RooCode, and Hermes.

- Research config formats for each agent
- Implement `AgentConnector` for each
- Add to `vault connector add` and `vault sync`
- **Files:** `crates/vault-connectors/src/cursor.rs`, `crates/vault-connectors/src/roocode.rs`, `crates/vault-connectors/src/hermes.rs`
- **Depends on:** Phase 3 (connector trait established)

### Phase 11: TUI Dashboard (v0.3.0)

> Interactive terminal UI for managing the vault.

- Built with `ratatui`
- Panels: installed MCPs, sync status per agent, live operation logs
- Keyboard navigation (vim-like)
- Real-time updates during sync/install operations
- **Files:** `crates/vault-tui/` (new crate), `crates/vault-tui/src/*.rs`
- **Depends on:** Phases 0–6 (all core features)

### Phase 12: File Watcher Auto-Sync (v0.3.0)

> Automatically detect changes to agent config files and re-sync.

- Use `notify` crate to watch agent config file paths
- On change detected: compute diff, prompt user (or auto-sync based on config)
- `vault watch` command — starts watcher daemon in foreground
- `vault watch --daemon` — background mode
- **Files:** `crates/vault-core/src/watcher.rs`, `crates/vault-cli/src/commands/watch.rs`
- **Dependencies:** `notify = "6.x"`
- **Depends on:** Phase 3 (connectors)

### Phase 13: Plugin System (v0.4.0)

> Allow third-party connector plugins.

- Define plugin interface (trait objects or WASM)
- Plugin discovery from `~/.agentvault/plugins/`
- `vault plugin install <url>` — download and register a plugin
- `vault plugin list` — show installed plugins
- **Files:** `crates/vault-core/src/plugin.rs`, `crates/vault-cli/src/commands/plugin.rs`
- **Depends on:** Phase 3 (connector trait as plugin interface)

---

## Timeline Summary

| Phase   | Name                        | Duration   | Days    | Key Deliverable                        |
|---------|-----------------------------|------------|---------|----------------------------------------|
| **0**   | Project Bootstrap           | 1 day      | Day 1   | Compilable workspace, CLI skeleton, CI |
| **1**   | Storage Foundation          | 2 days     | 2–3     | Config, SQLite, init/status/doctor     |
| **2**   | MCP Management Core         | 4 days     | 4–7     | Install/remove/update/list MCPs        |
| **3**   | Agent Connectors            | 4 days     | 8–11    | Sync to Claude, Gemini, OpenCode, Codex|
| **4**   | Search & Discovery          | 2 days     | 12–13   | Local + npm search, rich output        |
| **5**   | Manifest & Declarative      | 2 days     | 14–15   | Export/import vault.toml               |
| **6**   | Skills & Workflows          | 3 days     | 16–18   | Skill + workflow management            |
| **7**   | Polish & Release            | 3 days     | 19–21   | v0.1.0 release with binaries           |
|         |                             | **Total:** | **21 days** |                                    |

---

## Critical Path & Risk Register

| Risk                                         | Impact  | Likelihood | Mitigation                                                      |
|----------------------------------------------|---------|------------|------------------------------------------------------------------|
| OpenCode/Codex config format undocumented    | Medium  | High       | Ship stub connectors; implement when format is confirmed         |
| npm install side effects on CI               | Medium  | Medium     | Use mocked filesystem in integration tests; skip npm in CI       |
| Cross-compilation failures for ARM           | Low     | Medium     | Use `cross` Docker images; test in CI before release             |
| SQLite locking on concurrent access          | Medium  | Low        | WAL mode + single-connection-per-command design                  |
| Agent config format changes upstream         | High    | Medium     | Version-check known formats; log warnings for unknown fields     |
| PyPI install isolation (venv issues)         | Medium  | Medium     | Prefer `uv` for clean isolation; document pip fallback caveats   |

---

## Conventions & Standards

| Area               | Convention                                                                         |
|--------------------|------------------------------------------------------------------------------------|
| Error handling     | `vault-core` uses `VaultError` (thiserror); `vault-cli` wraps with `anyhow`       |
| Serialization      | All persistence uses serde: JSON for agent configs, TOML for vault config/manifest |
| Naming             | Structs: `PascalCase`; functions: `snake_case`; files: `snake_case.rs`             |
| Module structure   | One module per domain (mcp, skill, workflow, search, manifest, config, registry)   |
| Testing            | Unit tests in `#[cfg(test)] mod tests` within each file; integration in `tests/`   |
| Logging            | `tracing::info!` for user-visible ops; `tracing::debug!` for internals             |
| Async              | Tokio runtime in CLI; async traits where I/O is involved                           |
| Atomicity          | All file writes via temp-file-then-rename pattern                                  |
| Backups            | Always backup agent configs before writing; timestamped in `backups/<agent>/`      |

---

> **Last updated:** 2026-06-22
>
> **Owner:** @AswinkumarGP
>
> **Status:** Phase 0 — Not Started
