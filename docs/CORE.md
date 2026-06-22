# AgentVault — Core Architecture Document

> **Version:** 0.1.0-draft
> **Last Updated:** 2026-06-22
> **Status:** Implementation-Ready Specification

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Rust Project Structure](#2-rust-project-structure)
3. [Core Traits and Interfaces](#3-core-traits-and-interfaces)
4. [Error Architecture](#4-error-architecture)
5. [Configuration Architecture](#5-configuration-architecture)
6. [Storage Architecture](#6-storage-architecture)
7. [Dependency Resolution](#7-dependency-resolution)
8. [Crate Dependencies](#8-crate-dependencies)
9. [Data Flow Diagrams](#9-data-flow-diagrams)
10. [Testing Strategy](#10-testing-strategy)

---

## 1. Architecture Overview

AgentVault follows a **layered architecture** with strict dependency rules: upper layers may depend on lower layers, but never the reverse. Each layer has a single responsibility and communicates through well-defined Rust traits.

```
┌─────────────────────────────────────────────────────────────────────┐
│                        USER / TERMINAL                              │
└────────────────────────────────┬────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                       vault-cli  (Binary Crate)                     │
│                                                                     │
│   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐ │
│   │ install  │ │  remove  │ │  update  │ │  search  │ │  sync   │ │
│   └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬────┘ │
│        │             │            │             │            │      │
│   ┌────┴─────┐ ┌─────┴────┐ ┌────┴─────┐ ┌────┴─────┐ ┌────┴───┐ │
│   │  list    │ │  status  │ │  config  │ │  doctor  │ │ import │ │
│   └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬───┘ │
│        │             │            │             │            │      │
│   ┌────┴─────┐ ┌─────┴────┐ ┌────┴─────┐                          │
│   │  init    │ │connector │ │  export  │       output.rs           │
│   └──────────┘ └──────────┘ └──────────┘  (formatting helpers)     │
│                                                                     │
│   Responsibilities:                                                 │
│   • Parse CLI arguments (clap derive)                               │
│   • Map user intent to core operations                              │
│   • Format output for terminal (tables, colors, progress bars)      │
│   • Exit codes and user-facing error messages                       │
│                                                                     │
└────────────────────────────────┬────────────────────────────────────┘
                                 │  depends on
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                     vault-core  (Library Crate)                     │
│                                                                     │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │
│   │   config.rs  │  │ manifest.rs  │  │       registry.rs        │ │
│   │              │  │              │  │  (SQLite trait + impl)   │ │
│   └──────────────┘  └──────────────┘  └──────────────────────────┘ │
│                                                                     │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                      mcp/ module                             │  │
│   │  ┌────────────┐ ┌──────────────┐ ┌────────────┐ ┌─────────┐ │  │
│   │  │  models.rs │ │  manager.rs  │ │installer.rs│ │resolver │ │  │
│   │  └────────────┘ └──────────────┘ └────────────┘ └─────────┘ │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│   ┌──────────────────┐  ┌──────────────────┐  ┌─────────────────┐  │
│   │   skill/ module  │  │ workflow/ module  │  │capability/ mod  │  │
│   │  models.rs       │  │  models.rs        │  │  resolver.rs    │  │
│   │  manager.rs      │  │  manager.rs       │  │                 │  │
│   └──────────────────┘  └──────────────────┘  └─────────────────┘  │
│                                                                     │
│   ┌──────────────────────────────────────────────────────────────┐  │
│   │                       error.rs                               │  │
│   │                  (VaultError enum — thiserror)                │  │
│   └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
│   Responsibilities:                                                 │
│   • ALL business logic lives here                                   │
│   • Owns the domain model (MCP, Skill, Workflow, Capability)        │
│   • Registry CRUD against SQLite                                    │
│   • Filesystem store operations                                     │
│   • Version resolution and conflict detection                       │
│   • Configuration parsing and validation                            │
│                                                                     │
└────────────────────────────────┬────────────────────────────────────┘
                                 │  depends on
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                  vault-connectors  (Library Crate)                  │
│                                                                     │
│   ┌──────────────┐  ┌──────────────────────────────────────────┐   │
│   │  traits.rs   │  │  Connector Implementations               │   │
│   │              │  │  ┌───────────┐ ┌───────────┐ ┌─────────┐ │   │
│   │ AgentConnect │  │  │ claude.rs │ │ gemini.rs │ │opencode │ │   │
│   │   or trait   │  │  └───────────┘ └───────────┘ └─────────┘ │   │
│   └──────────────┘  │  ┌───────────┐                           │   │
│                     │  │  codex.rs │                            │   │
│   ┌──────────────┐  │  └───────────┘                           │   │
│   │   types.rs   │  └──────────────────────────────────────────┘   │
│   │ SyncResult   │                                                  │
│   │ SyncDiff     │  ┌──────────────┐                               │
│   │ SyncAction   │  │   sync.rs    │                               │
│   └──────────────┘  │  (engine)    │                               │
│                     └──────────────┘                                │
│                                                                     │
│   Responsibilities:                                                 │
│   • Read/write each agent's native config format                    │
│   • Detect installed capabilities per agent                         │
│   • Compute sync diffs between vault state and agent state          │
│   • Apply changes atomically with rollback                          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        STORAGE LAYER                                │
│                                                                     │
│   ┌─────────────────┐  ┌───────────────┐  ┌─────────────────────┐  │
│   │   SQLite DB     │  │  Filesystem   │  │   Config Files      │  │
│   │  registry.db    │  │  ~/.agentvault│  │   config.toml       │  │
│   │                 │  │  /store/      │  │   vault.toml        │  │
│   │  • capabilities│  │  • binaries   │  │   manifest.toml     │  │
│   │  • versions    │  │  • scripts    │  │                     │  │
│   │  • metadata    │  │  • configs    │  │                     │  │
│   │  • sync_log    │  │  • backups    │  │                     │  │
│   └─────────────────┘  └───────────────┘  └─────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Layer Dependency Rules

| Layer              | May depend on          | Must NOT depend on     |
|--------------------|------------------------|------------------------|
| `vault-cli`        | `vault-core`, `vault-connectors` | — |
| `vault-core`       | External crates only   | `vault-cli`, `vault-connectors` |
| `vault-connectors` | `vault-core` (types, traits) | `vault-cli` |

> **Key Invariant:** `vault-core` is a pure library with zero knowledge of CLI concerns or agent-specific config formats. This makes it independently testable and reusable (e.g., a future GUI or web interface can depend on `vault-core` without pulling in CLI logic).

---

## 2. Rust Project Structure

### Workspace Layout

```
agentvault/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── vault-cli/              # Binary crate — CLI entry point
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/        # One file per command
│   │       │   ├── mod.rs
│   │       │   ├── install.rs
│   │       │   ├── remove.rs
│   │       │   ├── update.rs
│   │       │   ├── list.rs
│   │       │   ├── search.rs
│   │       │   ├── sync.rs
│   │       │   ├── status.rs
│   │       │   ├── config.rs
│   │       │   ├── init.rs
│   │       │   ├── doctor.rs
│   │       │   ├── connector.rs
│   │       │   ├── import.rs
│   │       │   └── export.rs
│   │       └── output.rs       # Terminal formatting helpers
│   ├── vault-core/             # Library crate — all core logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs        # VaultError with thiserror
│   │       ├── config.rs       # config.toml parsing
│   │       ├── manifest.rs     # vault.toml parsing
│   │       ├── registry.rs     # SQLite registry (trait + impl)
│   │       ├── store.rs        # Filesystem operations
│   │       ├── mcp/
│   │       │   ├── mod.rs
│   │       │   ├── models.rs       # McpEntry, McpSource, etc.
│   │       │   ├── manager.rs      # McpManager trait + impl
│   │       │   ├── installer.rs    # Source-specific install logic
│   │       │   └── resolver.rs     # Version resolution
│   │       ├── skill/
│   │       │   ├── mod.rs
│   │       │   ├── models.rs
│   │       │   └── manager.rs
│   │       ├── workflow/
│   │       │   ├── mod.rs
│   │       │   ├── models.rs
│   │       │   └── manager.rs
│   │       └── capability/
│   │           ├── mod.rs
│   │           └── resolver.rs
│   └── vault-connectors/       # Library crate — agent connectors
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── traits.rs       # AgentConnector trait
│           ├── types.rs        # SyncResult, SyncDiff, etc.
│           ├── claude.rs       # Claude Code connector
│           ├── gemini.rs       # Gemini CLI connector
│           ├── opencode.rs     # OpenCode connector
│           ├── codex.rs        # Codex CLI connector
│           └── sync.rs         # Sync engine orchestration
└── tests/                      # Integration tests
    ├── cli_tests.rs
    ├── sync_tests.rs
    └── registry_tests.rs
```

### Why This Structure?

**1. Separation of Concerns**

Each crate owns a single slice of the system:

- **`vault-cli`** owns the human interface: argument parsing, terminal output, progress bars, exit codes. It converts user intent into calls on `vault-core` and `vault-connectors`, then formats results for the terminal. If AgentVault ever gets a TUI, GUI, or HTTP API, this is the only crate that gets replaced.
- **`vault-core`** owns the domain model and business logic. It has no opinion about how users interact with it (CLI, GUI, library) or which agents exist. It defines the canonical types (`McpEntry`, `SkillEntry`, `WorkflowEntry`), the registry abstraction, the version resolver, and the filesystem store.
- **`vault-connectors`** owns agent-specific knowledge: where Claude stores its MCP config, how Gemini CLI structures its settings, what format OpenCode expects. Each connector is isolated in its own file — adding a new agent means adding one file and updating `mod.rs`.

**2. Testability**

- `vault-core` is tested in isolation with no CLI or connector dependencies. Mock the `Registry` trait with an in-memory implementation, mock the filesystem with a `tempdir`, and every code path is reachable from `#[cfg(test)]` modules.
- `vault-connectors` is tested by fabricating agent config directories in temp dirs and asserting that read/write/diff operations produce correct results. No real agent installations needed.
- `vault-cli` is tested end-to-end with `assert_cmd`, exercising the real binary against temp home directories.

**3. Independent Versioning**

Cargo workspaces allow each crate to have its own `version` in its `Cargo.toml`. This means:
- A bug fix in `vault-connectors` (e.g., Claude changed its config path) ships as a patch to `vault-connectors` without bumping `vault-core`.
- A new CLI command ships as a minor bump to `vault-cli` only.
- A breaking change to a core trait bumps `vault-core`'s major version, and dependents update on their own schedule.

**4. Compile-Time Boundaries**

Rust's crate system enforces visibility at compile time. A `pub(crate)` item in `vault-core` is invisible to `vault-cli` — no runtime discipline required. This prevents accidental coupling that plagues monolithic codebases.

### Workspace Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/vault-cli",
    "crates/vault-core",
    "crates/vault-connectors",
]

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "MIT"
repository = "https://github.com/user/agentvault"

[workspace.dependencies]
# Shared dependency versions — each crate picks what it needs
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
anyhow = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
chrono = { version = "0.4", features = ["serde"] }
semver = { version = "1", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
```

---

## 3. Core Traits and Interfaces

All traits are defined in `vault-core` and are the **only** contract between layers. Implementations are swappable — production code uses real SQLite and real filesystem; tests use in-memory mocks.

### 3.1 CapabilityManager Trait

The top-level orchestration trait. Each capability type (MCP, Skill, Workflow) has its own manager, but they all satisfy this interface.

```rust
// vault-core/src/capability/mod.rs

use crate::error::VaultError;

/// Identifies what kind of capability we are operating on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CapabilityKind {
    Mcp,
    Skill,
    Workflow,
}

/// A request to install a single capability.
#[derive(Debug, Clone)]
pub struct InstallRequest {
    /// Human-readable name or registry identifier.
    pub name: String,
    /// Where to fetch from: registry, git, local path, URL.
    pub source: InstallSource,
    /// Optional version constraint (e.g., "^1.2.0"). None = latest.
    pub version: Option<semver::VersionReq>,
    /// Which agents should receive this capability on next sync.
    pub target_agents: Vec<String>,
    /// If true, overwrite an existing installation without prompting.
    pub force: bool,
}

/// The origin of a capability.
#[derive(Debug, Clone)]
pub enum InstallSource {
    /// Official or community registry (future).
    Registry { registry_url: String },
    /// Git repository with optional ref (branch/tag/commit).
    Git { url: String, ref_: Option<String> },
    /// Local filesystem path (for development).
    LocalPath(std::path::PathBuf),
    /// Direct download URL (tarball or zip).
    Url(String),
    /// npm package name (for MCP servers published on npm).
    Npm { package: String, version: Option<String> },
}

/// Result of a successful install.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub name: String,
    pub version: semver::Version,
    pub kind: CapabilityKind,
    pub install_path: std::path::PathBuf,
    pub source: InstallSource,
    /// SHA-256 of the installed artifact, if applicable.
    pub integrity_hash: Option<String>,
}

/// Filter for listing capabilities.
#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    pub kind: Option<CapabilityKind>,
    pub agent: Option<String>,
    pub name_contains: Option<String>,
    pub installed_only: bool,
}

/// Summary of one installed capability.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilityInfo {
    pub name: String,
    pub version: semver::Version,
    pub kind: CapabilityKind,
    pub source: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub agents: Vec<String>,
    pub status: CapabilityStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum CapabilityStatus {
    Active,
    Disabled,
    UpdateAvailable { latest: semver::Version },
    Broken { reason: String },
}

/// Result of an update operation.
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub name: String,
    pub previous_version: semver::Version,
    pub new_version: semver::Version,
    pub changelog_url: Option<String>,
}

/// Where to search for capabilities.
#[derive(Debug, Clone)]
pub enum SearchSource {
    /// Local registry only.
    Local,
    /// Remote registry / npm / GitHub.
    Remote,
    /// Both local and remote.
    All,
}

/// A single search result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub version: semver::Version,
    pub source: String,
    pub kind: CapabilityKind,
    /// true if already installed locally.
    pub installed: bool,
    /// Download count or popularity score, if available.
    pub popularity: Option<u64>,
}

/// The primary interface for managing capabilities of any kind.
///
/// Each capability type (MCP, Skill, Workflow) provides an implementation.
/// The CLI layer calls through this trait without knowing the underlying type.
///
/// All `async` methods use `async-trait` to enable dynamic dispatch.
#[async_trait::async_trait]
pub trait CapabilityManager: Send + Sync {
    /// Install a capability from the specified source.
    ///
    /// # Behavior
    /// 1. Validate the install request (name format, source reachability).
    /// 2. Check for conflicts with existing installations.
    /// 3. Download/copy the artifact to the local store.
    /// 4. Compute the integrity hash (SHA-256).
    /// 5. Register the capability in the SQLite registry.
    /// 6. Write a per-capability `manifest.toml`.
    ///
    /// # Errors
    /// - `VaultError::AlreadyExists` if installed and `force` is false.
    /// - `VaultError::Network` if a remote source is unreachable.
    /// - `VaultError::InvalidManifest` if the artifact has no valid manifest.
    async fn install(&self, request: InstallRequest) -> Result<InstallResult, VaultError>;

    /// Remove a capability by name.
    ///
    /// # Behavior
    /// 1. Look up the capability in the registry.
    /// 2. If `force` is false, check for dependents and prompt if any exist.
    /// 3. Remove the filesystem artifacts.
    /// 4. Delete the registry entry.
    /// 5. Remove from any agent sync targets.
    ///
    /// # Errors
    /// - `VaultError::NotFound` if the capability is not installed.
    /// - `VaultError::VersionConflict` if dependents exist and `force` is false.
    async fn remove(&self, name: &str, force: bool) -> Result<(), VaultError>;

    /// Update one or all capabilities.
    ///
    /// If `name` is `Some`, update only that capability.
    /// If `all` is true, update everything.
    /// Returns a list of what was actually updated (skips already-current).
    ///
    /// # Errors
    /// - `VaultError::NotFound` if a named capability doesn't exist.
    /// - `VaultError::Network` if update check fails.
    async fn update(
        &self,
        name: Option<&str>,
        all: bool,
    ) -> Result<Vec<UpdateResult>, VaultError>;

    /// List capabilities matching the given filter.
    ///
    /// This is a synchronous operation — all data comes from the local registry.
    fn list(&self, filter: ListFilter) -> Result<Vec<CapabilityInfo>, VaultError>;

    /// Search for capabilities locally, remotely, or both.
    ///
    /// Remote search queries known registries and npm (for MCP servers).
    /// Results are merged and deduplicated by name.
    async fn search(
        &self,
        query: &str,
        source: SearchSource,
    ) -> Result<Vec<SearchResult>, VaultError>;

    /// Return the kind of capability this manager handles.
    fn kind(&self) -> CapabilityKind;
}
```

### 3.2 AgentConnector Trait

Defined in `vault-connectors/src/traits.rs`. Each supported agent (Claude, Gemini, OpenCode, Codex) implements this trait.

```rust
// vault-connectors/src/traits.rs

use crate::types::{
    AgentCapability, AgentInfo, AgentStatus, SyncAction, SyncDiff, SyncOptions, SyncResult,
};
use vault_core::error::VaultError;
use std::path::{Path, PathBuf};

/// Represents a single agent installation on the local machine.
///
/// Each connector knows how to:
/// 1. Detect whether its agent is installed.
/// 2. Read the agent's current capability configuration.
/// 3. Compute a diff between vault state and agent state.
/// 4. Apply changes to the agent's config files.
/// 5. Validate that the agent's config is in a healthy state.
///
/// # Thread Safety
/// Connectors are `Send + Sync` so the sync engine can operate on
/// multiple agents concurrently via `tokio::spawn`.
#[async_trait::async_trait]
pub trait AgentConnector: Send + Sync {
    /// Human-readable name of the agent (e.g., "Claude Code", "Gemini CLI").
    fn name(&self) -> &str;

    /// Unique identifier used in config files and the registry.
    /// Lowercase, no spaces (e.g., "claude", "gemini", "opencode", "codex").
    fn id(&self) -> &str;

    /// Detect whether this agent is installed on the system.
    ///
    /// Checks for:
    /// - The agent's binary in $PATH.
    /// - The agent's config directory existence.
    ///
    /// Returns `AgentStatus::NotInstalled` if neither is found,
    /// `AgentStatus::Installed` with version info if found,
    /// `AgentStatus::Misconfigured` if installed but config is broken.
    async fn detect(&self) -> Result<AgentStatus, VaultError>;

    /// Return metadata about the agent installation.
    ///
    /// Includes: version, config directory path, binary path,
    /// supported capability kinds.
    async fn info(&self) -> Result<AgentInfo, VaultError>;

    /// Return the filesystem path to the agent's config directory.
    ///
    /// Examples:
    /// - Claude: `~/.claude/` (or platform equivalent)
    /// - Gemini: `~/.gemini/config/`
    /// - OpenCode: `~/.opencode/`
    /// - Codex: `~/.codex/`
    fn config_dir(&self) -> Result<PathBuf, VaultError>;

    /// Read the agent's current capability state from its config files.
    ///
    /// Parses the agent's native config format and returns a normalized
    /// list of capabilities (MCPs, skills, etc.) that the agent currently
    /// has configured.
    ///
    /// This does NOT modify any files — it is a read-only operation.
    async fn read_capabilities(&self) -> Result<Vec<AgentCapability>, VaultError>;

    /// Compute the diff between the vault's desired state and the agent's
    /// current state.
    ///
    /// Returns a `SyncDiff` containing:
    /// - Capabilities to add (in vault but not in agent).
    /// - Capabilities to remove (in agent but not in vault, if managed).
    /// - Capabilities to update (version mismatch).
    /// - Capabilities to skip (already in sync).
    ///
    /// # Arguments
    /// - `vault_capabilities`: The desired state from the vault registry.
    async fn diff(
        &self,
        vault_capabilities: &[AgentCapability],
    ) -> Result<SyncDiff, VaultError>;

    /// Apply a set of sync actions to the agent's config files.
    ///
    /// # Behavior
    /// 1. Create a backup of the current config (if `options.backup` is true).
    /// 2. Apply each action in order (add, remove, update).
    /// 3. Validate the resulting config (parse it back and check for errors).
    /// 4. If validation fails and `options.rollback_on_error` is true,
    ///    restore the backup.
    ///
    /// # Arguments
    /// - `actions`: The ordered list of changes to apply.
    /// - `options`: Controls backup, dry-run, and rollback behavior.
    ///
    /// # Errors
    /// - `VaultError::Connector` if the agent's config format is unexpected.
    /// - `VaultError::Io` if file operations fail.
    /// - `VaultError::PermissionDenied` if config files are not writable.
    async fn apply(
        &self,
        actions: &[SyncAction],
        options: &SyncOptions,
    ) -> Result<SyncResult, VaultError>;

    /// Validate the agent's current config for structural correctness.
    ///
    /// Does NOT check whether capabilities are functional — only that the
    /// config file parses correctly and has no obvious errors (duplicate
    /// keys, invalid paths, etc.).
    async fn validate(&self) -> Result<Vec<ValidationIssue>, VaultError>;

    /// Create a backup of the agent's config files.
    ///
    /// Returns the path to the backup directory/file.
    /// Backups are stored in `~/.agentvault/backups/<agent_id>/<timestamp>/`.
    async fn backup(&self) -> Result<PathBuf, VaultError>;

    /// Restore a previous backup.
    ///
    /// # Arguments
    /// - `backup_path`: Path returned by a previous `backup()` call.
    async fn restore(&self, backup_path: &Path) -> Result<(), VaultError>;
}

/// Validation finding from `AgentConnector::validate()`.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Config works but could be improved.
    Warning,
    /// Config has errors that will cause problems.
    Error,
}
```

### 3.3 Registry Trait

The persistence abstraction. Production uses SQLite; tests use an in-memory mock.

```rust
// vault-core/src/registry.rs

use crate::capability::{CapabilityInfo, CapabilityKind, CapabilityStatus, ListFilter};
use crate::error::VaultError;
use chrono::{DateTime, Utc};
use semver::Version;
use std::path::PathBuf;

/// A row in the registry — the canonical record of an installed capability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegistryEntry {
    /// Unique identifier (UUID v4).
    pub id: uuid::Uuid,
    /// Human-readable name (e.g., "filesystem-mcp").
    pub name: String,
    /// Installed version.
    pub version: Version,
    /// MCP, Skill, or Workflow.
    pub kind: CapabilityKind,
    /// How it was installed (serialized InstallSource).
    pub source: String,
    /// Where the artifact lives on disk.
    pub install_path: PathBuf,
    /// SHA-256 hash of the installed artifact.
    pub integrity_hash: Option<String>,
    /// Which agents this capability is targeted to.
    pub agents: Vec<String>,
    /// Current status.
    pub status: CapabilityStatus,
    /// When first installed.
    pub installed_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
    /// Arbitrary metadata (JSON blob).
    pub metadata: Option<serde_json::Value>,
}

/// Filter for querying the registry.
#[derive(Debug, Clone, Default)]
pub struct RegistryQuery {
    pub name: Option<String>,
    pub kind: Option<CapabilityKind>,
    pub agent: Option<String>,
    pub status: Option<CapabilityStatus>,
    pub installed_after: Option<DateTime<Utc>>,
    pub installed_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// The persistence interface for the capability registry.
///
/// # Design Notes
/// - This trait is synchronous because SQLite operations are fast and
///   `rusqlite` does not support async natively. Wrapping in `spawn_blocking`
///   is the caller's responsibility if needed.
/// - The trait uses `&self` (not `&mut self`) because `rusqlite::Connection`
///   can be wrapped in a `Mutex` for thread safety.
pub trait Registry: Send + Sync {
    /// Insert a new registry entry.
    ///
    /// # Errors
    /// - `VaultError::AlreadyExists` if an entry with the same name and kind exists.
    /// - `VaultError::Database` on SQLite errors.
    fn insert(&self, entry: &RegistryEntry) -> Result<(), VaultError>;

    /// Remove an entry by name and kind.
    ///
    /// # Errors
    /// - `VaultError::NotFound` if no matching entry exists.
    fn remove(&self, name: &str, kind: CapabilityKind) -> Result<(), VaultError>;

    /// Update an existing entry (matched by id).
    ///
    /// Overwrites all fields except `id` and `installed_at`.
    ///
    /// # Errors
    /// - `VaultError::NotFound` if the id doesn't exist.
    fn update(&self, entry: &RegistryEntry) -> Result<(), VaultError>;

    /// Query entries matching the filter.
    ///
    /// Returns an empty vec if no matches found (not an error).
    fn query(&self, query: &RegistryQuery) -> Result<Vec<RegistryEntry>, VaultError>;

    /// Get a single entry by name and kind.
    ///
    /// # Errors
    /// - `VaultError::NotFound` if not present.
    fn get(&self, name: &str, kind: CapabilityKind) -> Result<RegistryEntry, VaultError>;

    /// Get a single entry by UUID.
    ///
    /// # Errors
    /// - `VaultError::NotFound` if not present.
    fn get_by_id(&self, id: uuid::Uuid) -> Result<RegistryEntry, VaultError>;

    /// List all entries, optionally filtered.
    fn list(&self, filter: ListFilter) -> Result<Vec<RegistryEntry>, VaultError>;

    /// Return the count of entries matching the filter.
    fn count(&self, filter: ListFilter) -> Result<usize, VaultError>;

    /// Run pending schema migrations.
    ///
    /// Called once at startup. Migrations are idempotent.
    /// Uses a `schema_version` table to track the current version.
    ///
    /// # Errors
    /// - `VaultError::Database` if a migration SQL statement fails.
    fn migrate(&self) -> Result<(), VaultError>;

    /// Record a sync event in the sync log.
    fn log_sync(
        &self,
        agent_id: &str,
        actions: &[crate::SyncLogEntry],
    ) -> Result<(), VaultError>;

    /// Retrieve sync history for an agent.
    fn sync_history(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<SyncLogRecord>, VaultError>;
}

/// A single entry in the sync log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncLogEntry {
    pub capability_name: String,
    pub action: String,  // "add", "remove", "update"
    pub success: bool,
    pub error_message: Option<String>,
}

/// A full sync log record with timestamp and agent info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncLogRecord {
    pub id: uuid::Uuid,
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub entries: Vec<SyncLogEntry>,
}
```

### 3.4 Resolver Trait

Handles version resolution and dependency/conflict checking.

```rust
// vault-core/src/capability/resolver.rs

use crate::error::VaultError;
use crate::registry::RegistryEntry;
use semver::{Version, VersionReq};
use std::collections::HashMap;

/// A dependency declaration from a capability's manifest.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    /// Name of the required capability.
    pub name: String,
    /// Version constraint (e.g., ">=1.0.0, <2.0.0").
    pub version_req: VersionReq,
    /// If true, the dependency is optional (feature-gated).
    pub optional: bool,
}

/// A detected conflict between capabilities.
#[derive(Debug, Clone)]
pub struct Conflict {
    /// The capability name that conflicts.
    pub capability_name: String,
    /// Agents or capabilities that require conflicting versions.
    pub conflicting_parties: Vec<ConflictParty>,
    /// Human-readable explanation.
    pub message: String,
    /// Suggested resolution, if any.
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConflictParty {
    /// Who requires this version (agent name or capability name).
    pub requester: String,
    /// What version they require.
    pub required_version: VersionReq,
    /// What version is currently installed.
    pub installed_version: Option<Version>,
}

/// Result of a dependency resolution.
#[derive(Debug, Clone)]
pub struct ResolutionPlan {
    /// Capabilities that need to be installed, in dependency order.
    pub to_install: Vec<ResolutionStep>,
    /// Capabilities that need to be updated.
    pub to_update: Vec<ResolutionStep>,
    /// Detected conflicts that block resolution.
    pub conflicts: Vec<Conflict>,
    /// True if the plan can be executed without conflicts.
    pub is_satisfiable: bool,
}

#[derive(Debug, Clone)]
pub struct ResolutionStep {
    pub name: String,
    pub version: Version,
    pub reason: String, // Why this is needed (e.g., "required by workflow X")
}

/// The dependency resolver interface.
///
/// Handles:
/// - Topological ordering of workflow steps.
/// - Semver constraint solving for MCP version requirements.
/// - Cross-agent conflict detection (same MCP, different versions).
#[async_trait::async_trait]
pub trait Resolver: Send + Sync {
    /// Given a set of capabilities to install/update, resolve all transitive
    /// dependencies and produce an ordered execution plan.
    ///
    /// # Algorithm
    /// 1. Build a dependency graph from manifests.
    /// 2. Detect cycles (error if found — capabilities must be a DAG).
    /// 3. Topological sort to determine install order.
    /// 4. For each node, find the best version satisfying all constraints.
    /// 5. Report any unsatisfiable constraints as conflicts.
    ///
    /// # Arguments
    /// - `requested`: The capabilities the user explicitly asked to install.
    /// - `installed`: The current registry state (to avoid re-installing).
    async fn resolve_dependencies(
        &self,
        requested: &[Dependency],
        installed: &[RegistryEntry],
    ) -> Result<ResolutionPlan, VaultError>;

    /// Check whether installing a new capability would conflict with
    /// existing installations.
    ///
    /// This is a fast check (no network) — it only looks at local state.
    ///
    /// # Returns
    /// - Empty vec if no conflicts.
    /// - One `Conflict` per detected issue.
    fn check_conflicts(
        &self,
        new_capability: &str,
        new_version: &Version,
        installed: &[RegistryEntry],
    ) -> Result<Vec<Conflict>, VaultError>;

    /// For workflow steps, produce a topologically sorted execution order.
    ///
    /// # Errors
    /// - `VaultError::VersionConflict` if a cycle is detected.
    fn topological_sort(
        &self,
        steps: &[WorkflowStep],
    ) -> Result<Vec<WorkflowStep>, VaultError>;
}

/// A single step in a workflow (used for topological sorting).
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub name: String,
    pub depends_on: Vec<String>,
    pub capability_name: String,
    pub config: serde_json::Value,
}
```

---

## 4. Error Architecture

### VaultError Enum

All errors in `vault-core` and `vault-connectors` are expressed as variants of `VaultError`. The CLI layer converts these into user-facing messages and exit codes.

```rust
// vault-core/src/error.rs

use std::path::PathBuf;

/// The unified error type for all AgentVault operations.
///
/// # Design Decisions
///
/// - **`thiserror`** for the enum definition: gives us `Display`, `Error`,
///   and `From` impls with zero boilerplate.
/// - **Structured variants** (not just string messages): the CLI can
///   pattern-match on variants to produce contextual help, suggest fixes,
///   or set specific exit codes.
/// - **No `anyhow::Error` in the core**: `anyhow` is used only at the CLI
///   boundary for ad-hoc context. Core code always returns `VaultError`.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    // ── Filesystem & I/O ──────────────────────────────────────────────

    /// A filesystem operation failed.
    ///
    /// Wraps `std::io::Error` with the path that caused the failure.
    #[error("I/O error at '{}': {source}", path.display())]
    Io {
        source: std::io::Error,
        path: PathBuf,
        /// What operation was being attempted (read, write, delete, etc.)
        operation: String,
    },

    // ── Database ──────────────────────────────────────────────────────

    /// A SQLite operation failed.
    #[error("Database error: {source}")]
    Database {
        #[from]
        source: rusqlite::Error,
    },

    /// A database migration failed.
    #[error("Migration failed (version {version}): {message}")]
    Migration {
        version: u32,
        message: String,
    },

    // ── Configuration ─────────────────────────────────────────────────

    /// The global config file (config.toml) has invalid content.
    #[error("Configuration error in '{}': {message}", path.display())]
    Config {
        path: PathBuf,
        message: String,
        /// The specific field that is invalid, if known.
        field: Option<String>,
    },

    // ── Network ───────────────────────────────────────────────────────

    /// An HTTP request failed (download, registry query, etc.).
    #[error("Network error: {message}")]
    Network {
        message: String,
        url: Option<String>,
        status_code: Option<u16>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // ── Agent Connectors ──────────────────────────────────────────────

    /// An agent connector encountered an error.
    ///
    /// This is the "catch-all" for agent-specific problems:
    /// config file in an unexpected format, agent binary not found, etc.
    #[error("Connector error ({agent}): {message}")]
    Connector {
        agent: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    // ── MCP Installation ──────────────────────────────────────────────

    /// An MCP server failed to install.
    #[error("MCP install failed for '{name}': {message}")]
    McpInstall {
        name: String,
        message: String,
        /// The install source that was attempted.
        source_type: String,
    },

    // ── Lookup Errors ─────────────────────────────────────────────────

    /// A requested capability was not found in the registry.
    #[error("Not found: {kind} '{name}'")]
    NotFound {
        name: String,
        kind: String,
        /// Suggestions for similar names (fuzzy match).
        suggestions: Vec<String>,
    },

    /// A capability with this name already exists.
    #[error("Already exists: {kind} '{name}' (version {existing_version})")]
    AlreadyExists {
        name: String,
        kind: String,
        existing_version: String,
    },

    // ── Version & Dependency ──────────────────────────────────────────

    /// Two or more requirements for the same capability are incompatible.
    #[error("Version conflict for '{name}': {message}")]
    VersionConflict {
        name: String,
        message: String,
        /// Who requires what.
        parties: Vec<String>,
    },

    /// A dependency cycle was detected.
    #[error("Dependency cycle detected: {cycle}")]
    DependencyCycle {
        cycle: String,
    },

    // ── Permissions ───────────────────────────────────────────────────

    /// A file or directory is not accessible with the required permissions.
    #[error("Permission denied: '{}'", path.display())]
    PermissionDenied {
        path: PathBuf,
        /// What was attempted (read, write, execute).
        required: String,
    },

    // ── Serialization ─────────────────────────────────────────────────

    /// A TOML, JSON, or other serialization/deserialization failed.
    #[error("Serialization error ({format}): {message}")]
    Serialization {
        format: String, // "toml", "json", etc.
        message: String,
    },

    // ── Manifest ──────────────────────────────────────────────────────

    /// A capability's manifest.toml is invalid or missing required fields.
    #[error("Invalid manifest for '{name}': {message}")]
    InvalidManifest {
        name: String,
        message: String,
        /// The path to the manifest file, if known.
        path: Option<PathBuf>,
    },

    // ── Agent Support ─────────────────────────────────────────────────

    /// The user requested an operation on an agent that is not supported.
    #[error("Unsupported agent: '{agent}' (supported: {supported})")]
    UnsupportedAgent {
        agent: String,
        supported: String, // Comma-separated list
    },

    // ── External Commands ─────────────────────────────────────────────

    /// An external command (npm, npx, git, etc.) failed.
    #[error("Command failed: `{command}` exited with {exit_code}")]
    CommandFailed {
        command: String,
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
}
```

### Error Propagation Path

```
┌─────────────────────────────────────────────────────────────────────┐
│                       Error Propagation                             │
│                                                                     │
│   Storage Layer                                                     │
│   ┌──────────────────────────────────────────┐                      │
│   │  rusqlite::Error  ──→  VaultError::Database                     │
│   │  std::io::Error   ──→  VaultError::Io                           │
│   │  toml::de::Error  ──→  VaultError::Serialization                │
│   └──────────────────────────────────────────┘                      │
│                           │                                         │
│                           ▼                                         │
│   Core Layer (vault-core)                                           │
│   ┌──────────────────────────────────────────┐                      │
│   │  Functions return Result<T, VaultError>                         │
│   │  Use ? operator for automatic conversion                       │
│   │  Add context via VaultError variants                            │
│   │                                                                │
│   │  Example:                                                      │
│   │    registry.get(name, kind)              // VaultError::NotFound│
│   │    store.read_manifest(path)             // VaultError::Io     │
│   │    resolver.check_conflicts(...)         // VersionConflict    │
│   └──────────────────────────────────────────┘                      │
│                           │                                         │
│                           ▼                                         │
│   Connector Layer (vault-connectors)                                │
│   ┌──────────────────────────────────────────┐                      │
│   │  Agent-specific errors wrapped in                               │
│   │  VaultError::Connector { agent, message }                       │
│   │                                                                │
│   │  JSON parse failures → VaultError::Serialization                │
│   │  Missing config → VaultError::Config                            │
│   └──────────────────────────────────────────┘                      │
│                           │                                         │
│                           ▼                                         │
│   CLI Layer (vault-cli)                                             │
│   ┌──────────────────────────────────────────┐                      │
│   │  match on VaultError variant:                                   │
│   │                                                                │
│   │  NotFound        → stderr red message + suggestions            │
│   │                     exit code 1                                │
│   │  VersionConflict → stderr message + conflict table             │
│   │                     exit code 2                                │
│   │  Network         → stderr message + "check connection"        │
│   │                     exit code 3                                │
│   │  PermissionDenied → stderr + "try with sudo"                  │
│   │                     exit code 4                                │
│   │  *               → stderr generic message                     │
│   │                     exit code 1                                │
│   └──────────────────────────────────────────┘                      │
│                           │                                         │
│                           ▼                                         │
│   ┌──────────────────────────────────────────┐                      │
│   │  User sees: colored, actionable message                        │
│   │  Machine sees: exit code + optional JSON (--json flag)         │
│   └──────────────────────────────────────────┘                      │
└─────────────────────────────────────────────────────────────────────┘
```

### CLI Exit Code Convention

| Exit Code | Meaning                       | VaultError Variant(s)             |
|-----------|-------------------------------|-----------------------------------|
| 0         | Success                       | —                                 |
| 1         | General error / not found     | `NotFound`, `InvalidManifest`, `Serialization`, `McpInstall` |
| 2         | Conflict / dependency error   | `VersionConflict`, `DependencyCycle`, `AlreadyExists` |
| 3         | Network error                 | `Network`                         |
| 4         | Permission error              | `PermissionDenied`                |
| 5         | Configuration error           | `Config`                          |
| 6         | Unsupported agent             | `UnsupportedAgent`                |
| 10        | External command failed       | `CommandFailed`                   |
| 20        | Database error                | `Database`, `Migration`           |
| 127       | Bug / unexpected              | Anything unmatched (should never happen) |

---

## 5. Configuration Architecture

AgentVault uses a **three-level configuration hierarchy**, from broadest scope to narrowest:

### 5.1 Global Configuration: `~/.agentvault/config.toml`

This file controls AgentVault's own behavior. It is created by `vault init` and can be edited manually or via `vault config set`.

```toml
# ~/.agentvault/config.toml
# AgentVault global configuration

[general]
# Default source for installing capabilities.
# Options: "npm", "git", "registry", "local"
default_source = "npm"

# Whether to automatically create backups before sync operations.
auto_backup = true

# Maximum number of backups to retain per agent.
max_backups = 10

# Log level: "trace", "debug", "info", "warn", "error"
log_level = "info"

# Path to the log file. Empty string disables file logging.
log_file = "~/.agentvault/logs/vault.log"

# Whether to show interactive prompts (set to false for CI/scripts).
interactive = true

# Output format: "pretty" (default, colored), "json", "plain"
output_format = "pretty"

[registry]
# Path to the SQLite database.
database_path = "~/.agentvault/registry.db"

# Enable WAL mode for better concurrent read performance.
wal_mode = true

[store]
# Root directory for installed capability artifacts.
store_path = "~/.agentvault/store"

# Root directory for backups.
backup_path = "~/.agentvault/backups"

[network]
# HTTP request timeout in seconds.
timeout_secs = 30

# HTTP proxy (empty = no proxy).
proxy = ""

# Whether to verify TLS certificates (disable only for testing).
tls_verify = true

# User-Agent header for HTTP requests.
user_agent = "agentvault/0.1.0"

[agents]
# Which agents to auto-detect on `vault init` and `vault doctor`.
# Options: "claude", "gemini", "opencode", "codex"
auto_detect = ["claude", "gemini", "opencode", "codex"]

# Default agents to sync to when no --agent flag is provided.
default_sync_targets = ["claude", "gemini"]

[sync]
# Strategy when a capability exists in the agent but not in vault:
# "ignore" — leave it alone (default)
# "warn"   — print a warning
# "import" — import it into the vault
unmanaged_strategy = "ignore"

# Whether to run `vault doctor` checks after each sync.
post_sync_check = true

# Dry-run by default (require --apply to actually change files).
dry_run_default = false
```

### 5.2 Project Manifest: `vault.toml`

This file is the **declarative capability list** — the "desired state" of what should be installed and synced. It lives in the project root or `~/.agentvault/vault.toml` for global scope.

```toml
# vault.toml — Declarative capability manifest
# Defines what capabilities should be installed and which agents get them.

[metadata]
name = "my-dev-setup"
description = "My full AI development environment"
version = "1.0.0"
author = "developer@example.com"

# ── MCP Servers ───────────────────────────────────────────────────────

[[mcp]]
name = "filesystem"
source = "npm"
package = "@anthropic/mcp-filesystem"
version = "^1.2.0"
agents = ["claude", "gemini"]
enabled = true

[mcp.config]
# MCP-specific configuration passed to the server.
allowed_directories = ["/home/user/projects", "/tmp"]

[[mcp]]
name = "postgres"
source = "npm"
package = "@anthropic/mcp-postgres"
version = "^0.5.0"
agents = ["claude"]
enabled = true

[mcp.config]
connection_string = "postgresql://localhost:5432/mydb"

[[mcp]]
name = "custom-tool"
source = "git"
url = "https://github.com/user/custom-mcp-tool.git"
ref = "v2.1.0"
agents = ["claude", "gemini", "opencode"]
enabled = true

[[mcp]]
name = "local-dev-mcp"
source = "local"
path = "/home/user/projects/my-mcp-server"
agents = ["claude"]
enabled = true

# ── Skills ────────────────────────────────────────────────────────────

[[skill]]
name = "rust-conventions"
source = "local"
path = "/home/user/.gemini/config/skills/rust-skills"
agents = ["gemini"]
enabled = true

[[skill]]
name = "code-review"
source = "git"
url = "https://github.com/user/agent-skills.git"
path = "skills/code-review"  # subdirectory within the repo
agents = ["gemini", "claude"]
enabled = true

# ── Workflows ─────────────────────────────────────────────────────────

[[workflow]]
name = "full-stack-setup"
description = "Install all capabilities for a full-stack project"
agents = ["claude", "gemini"]
enabled = true

[[workflow.step]]
name = "install-filesystem"
capability = "filesystem"
order = 1

[[workflow.step]]
name = "install-postgres"
capability = "postgres"
order = 2
depends_on = ["install-filesystem"]

[[workflow.step]]
name = "install-skills"
capability = "code-review"
order = 3
```

### 5.3 Per-Capability Manifest: `manifest.toml`

Each installed capability gets its own manifest at `~/.agentvault/store/<kind>/<name>/manifest.toml`. This is auto-generated during install and updated during updates.

```toml
# ~/.agentvault/store/mcp/filesystem/manifest.toml
# Auto-generated by AgentVault — do not edit manually.

[capability]
name = "filesystem"
kind = "mcp"
version = "1.2.3"
description = "Provides filesystem access to AI agents"
source_type = "npm"
source_ref = "@anthropic/mcp-filesystem@1.2.3"
install_path = "/home/user/.agentvault/store/mcp/filesystem"

[integrity]
sha256 = "a1b2c3d4e5f6..."
verified_at = "2026-06-22T14:00:00Z"

[agents]
targets = ["claude", "gemini"]

[timestamps]
installed_at = "2026-06-22T12:00:00Z"
updated_at = "2026-06-22T14:00:00Z"

[mcp]
# MCP-specific fields
transport = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-filesystem", "--allowed-dirs", "/home/user/projects"]

[mcp.env]
# Environment variables to set when running the MCP server.
# Values can reference other env vars with ${VAR_NAME}.
MCP_LOG_LEVEL = "info"

[dependencies]
# Other capabilities this one requires.
# Empty for most MCPs, populated for complex workflows.
```

### Configuration Precedence

When the same setting appears at multiple levels:

```
Per-Capability manifest.toml  (highest priority — most specific)
      ↓ overrides
Project vault.toml             (project-level)
      ↓ overrides
Global config.toml             (lowest priority — most general)
```

---

## 6. Storage Architecture

AgentVault uses three complementary storage backends, each chosen for a specific reason.

### 6.1 SQLite Registry (`~/.agentvault/registry.db`)

**Why SQLite?**

| Requirement                      | SQLite Advantage                              |
|----------------------------------|-----------------------------------------------|
| Fast queries across all capabilities | Indexed queries on name, kind, agent, version |
| ACID transactions                | Atomic installs: either fully registered or not at all |
| Zero configuration               | No daemon, no server, no port — it's a file   |
| Single-file portable             | Backup = copy one file. Migration = embedded. |
| Concurrent reads                 | WAL mode allows readers and one writer simultaneously |
| Small footprint                  | Hundreds of capabilities fit in < 1 MB         |

**Schema:**

```sql
-- Version tracking for migrations
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Core capability registry
CREATE TABLE IF NOT EXISTS capabilities (
    id TEXT PRIMARY KEY,                  -- UUID v4
    name TEXT NOT NULL,
    version TEXT NOT NULL,                -- Semver string
    kind TEXT NOT NULL CHECK(kind IN ('mcp', 'skill', 'workflow')),
    source TEXT NOT NULL,                 -- JSON: { "type": "npm", "package": "..." }
    install_path TEXT NOT NULL,
    integrity_hash TEXT,                  -- SHA-256
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'disabled', 'broken')),
    metadata TEXT,                        -- JSON blob for extensibility
    installed_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(name, kind)
);

-- Many-to-many: which agents receive which capability
CREATE TABLE IF NOT EXISTS capability_agents (
    capability_id TEXT NOT NULL REFERENCES capabilities(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    synced_at TEXT,                        -- NULL = never synced
    sync_status TEXT DEFAULT 'pending'
        CHECK(sync_status IN ('pending', 'synced', 'failed')),
    PRIMARY KEY (capability_id, agent_id)
);

-- Sync operation log
CREATE TABLE IF NOT EXISTS sync_log (
    id TEXT PRIMARY KEY,                  -- UUID v4
    agent_id TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    entries TEXT NOT NULL                  -- JSON array of SyncLogEntry
);

-- Indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_capabilities_kind ON capabilities(kind);
CREATE INDEX IF NOT EXISTS idx_capabilities_name ON capabilities(name);
CREATE INDEX IF NOT EXISTS idx_capabilities_status ON capabilities(status);
CREATE INDEX IF NOT EXISTS idx_capability_agents_agent ON capability_agents(agent_id);
CREATE INDEX IF NOT EXISTS idx_sync_log_agent ON sync_log(agent_id);
CREATE INDEX IF NOT EXISTS idx_sync_log_timestamp ON sync_log(timestamp);
```

**Connection Management:**

```rust
// vault-core/src/registry.rs (implementation sketch)

use rusqlite::Connection;
use std::sync::Mutex;

/// Production registry backed by SQLite.
pub struct SqliteRegistry {
    /// Connection wrapped in Mutex for thread safety.
    /// `rusqlite::Connection` is not Sync, but Mutex<Connection> is.
    conn: Mutex<Connection>,
}

impl SqliteRegistry {
    /// Open or create the registry database.
    ///
    /// # Behavior
    /// 1. Open the SQLite file (create if not exists).
    /// 2. Enable WAL mode for concurrent read performance.
    /// 3. Set busy timeout to 5 seconds (prevents SQLITE_BUSY).
    /// 4. Run pending migrations.
    pub fn open(path: &std::path::Path) -> Result<Self, VaultError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let registry = Self {
            conn: Mutex::new(conn),
        };
        registry.migrate()?;
        Ok(registry)
    }
}
```

**Migration Strategy:**

Migrations are embedded as Rust constants and run sequentially at startup:

```rust
const MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("../migrations/001_initial.sql")),
    (2, include_str!("../migrations/002_add_sync_log.sql")),
    // Future migrations appended here.
];

impl SqliteRegistry {
    fn migrate(&self) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();

        // Create the version table if it doesn't exist.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );"
        )?;

        let current_version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )?;

        for (version, sql) in MIGRATIONS {
            if *version > current_version {
                conn.execute_batch(sql)
                    .map_err(|e| VaultError::Migration {
                        version: *version,
                        message: e.to_string(),
                    })?;
                conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    [version],
                )?;
                tracing::info!("Applied migration v{}", version);
            }
        }

        Ok(())
    }
}
```

### 6.2 Filesystem Store (`~/.agentvault/store/`)

**Why filesystem?**

MCP servers are real executables, npm packages, or scripts. They must live on disk for agents to invoke them. SQLite stores metadata; the filesystem stores the actual artifacts.

**Directory Layout:**

```
~/.agentvault/
├── config.toml
├── vault.toml
├── registry.db
├── store/
│   ├── mcp/
│   │   ├── filesystem/
│   │   │   ├── manifest.toml
│   │   │   ├── node_modules/          # npm-installed MCP
│   │   │   └── package.json
│   │   ├── postgres/
│   │   │   ├── manifest.toml
│   │   │   └── ...
│   │   └── custom-tool/
│   │       ├── manifest.toml
│   │       ├── src/                    # git-cloned MCP
│   │       └── ...
│   ├── skill/
│   │   ├── rust-conventions/
│   │   │   ├── manifest.toml
│   │   │   └── SKILL.md
│   │   └── code-review/
│   │       ├── manifest.toml
│   │       └── SKILL.md
│   └── workflow/
│       └── full-stack-setup/
│           └── manifest.toml
├── backups/
│   ├── claude/
│   │   └── 2026-06-22T12-00-00/
│   │       └── claude_desktop_config.json
│   └── gemini/
│       └── 2026-06-22T12-00-00/
│           └── settings.json
├── logs/
│   └── vault.log
└── cache/
    └── npm/                            # Cached npm tarballs
```

### 6.3 Configuration Files (TOML)

**Why TOML?**

- Human-readable and human-editable (unlike SQLite blobs).
- First-class support in the Rust ecosystem (`toml` crate, `serde` derive).
- Hierarchical structure maps naturally to Rust structs.
- Comments are preserved (unlike JSON).
- Used by Cargo itself, so Rust developers are already familiar with it.

---

## 7. Dependency Resolution

### 7.1 Topological Sort for Workflow Steps

Workflows define steps with `depends_on` relationships, forming a Directed Acyclic Graph (DAG). Steps must execute in an order that respects all dependency edges.

**Algorithm: Kahn's algorithm (BFS-based topological sort)**

```
Input:  steps = [{ name, depends_on }]
Output: ordered list of steps, or error if cycle detected

1. Build adjacency list and in-degree map:
   for each step S:
       in_degree[S] = |S.depends_on|
       for each dep D in S.depends_on:
           adjacency[D].push(S)

2. Initialize queue with all steps where in_degree == 0
   (these have no unmet dependencies)

3. While queue is not empty:
       S = queue.pop_front()
       output.push(S)
       for each step T in adjacency[S]:
           in_degree[T] -= 1
           if in_degree[T] == 0:
               queue.push(T)

4. If |output| != |steps|:
       ERROR: cycle detected
       (the un-output steps form the cycle)
       Return VaultError::DependencyCycle
```

**Example:**

```
Steps:
  A (depends_on: [])
  B (depends_on: [A])
  C (depends_on: [A])
  D (depends_on: [B, C])

Adjacency:
  A → [B, C]
  B → [D]
  C → [D]

In-degree:
  A: 0, B: 1, C: 1, D: 2

Sort:
  Queue: [A]
  Pop A → output [A], decrement B (0), C (0) → Queue: [B, C]
  Pop B → output [A, B], decrement D (1) → Queue: [C]
  Pop C → output [A, B, C], decrement D (0) → Queue: [D]
  Pop D → output [A, B, C, D]

Result: [A, B, C, D] ✓
```

### 7.2 Version Constraint Solving for MCPs

When multiple agents or workflows require the same MCP at different version constraints, the resolver must find a single version that satisfies all constraints.

**Algorithm:**

```
Input:
  constraints = [
      { requester: "vault.toml",     name: "filesystem", req: "^1.2.0" },
      { requester: "workflow-X",     name: "filesystem", req: ">=1.0, <2.0" },
      { requester: "agent-claude",   name: "filesystem", req: "~1.2" },
  ]

Steps:
1. Group constraints by capability name.

2. For each group, compute the intersection of all VersionReq ranges:
   - "^1.2.0"        → >=1.2.0, <2.0.0
   - ">=1.0, <2.0"   → >=1.0.0, <2.0.0
   - "~1.2"          → >=1.2.0, <1.3.0

   Intersection: >=1.2.0, <1.3.0

3. Query available versions (from npm / registry / git tags).

4. Find the highest version within the intersection range.
   Available: [1.0.0, 1.1.0, 1.2.0, 1.2.3, 1.2.5, 1.3.0, 2.0.0]
   Matching:  [1.2.0, 1.2.3, 1.2.5]
   Selected:  1.2.5 (highest)

5. If no version satisfies all constraints → VaultError::VersionConflict
   with details about which constraints are incompatible.
```

**Implementation using the `semver` crate:**

```rust
use semver::{Version, VersionReq};

fn find_best_version(
    available: &[Version],
    constraints: &[VersionReq],
) -> Option<Version> {
    available
        .iter()
        .filter(|v| constraints.iter().all(|req| req.matches(v)))
        .max()
        .cloned()
}
```

### 7.3 Cross-Agent Conflict Detection

A conflict arises when two agents need the same MCP but at incompatible versions. Since AgentVault installs each MCP once in its store, it must detect this early.

```
Scenario:
  Claude wants   "postgres" at "^0.5.0"
  Gemini wants   "postgres" at "^0.4.0"

  These ranges do NOT overlap:
    ^0.5.0 → >=0.5.0, <0.6.0
    ^0.4.0 → >=0.4.0, <0.5.0

  Resolution options (presented to user):
  1. Install both versions side-by-side (store/mcp/postgres-0.5/ and postgres-0.4/)
  2. Upgrade the lower constraint (if possible)
  3. Let the user choose which version to use globally

  AgentVault's default: report the conflict and let the user decide.
  Future enhancement: automatic side-by-side installation.
```

---

## 8. Crate Dependencies

### `vault-core/Cargo.toml`

```toml
[package]
name = "vault-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
# Serialization
serde.workspace = true
serde_json.workspace = true
toml.workspace = true

# Error handling
thiserror.workspace = true

# Database
rusqlite = { version = "0.31", features = ["bundled"] }
# `bundled` compiles SQLite from source, avoiding system-library headaches
# on macOS, Windows, and Linux. Adds ~30s to first build, but users never
# need to install libsqlite3-dev.

# Async runtime
tokio.workspace = true
async-trait = "0.1"
# async-trait is needed because Rust's native async trait support does not
# yet support dynamic dispatch (dyn CapabilityManager). The 0.1 series is
# stable and widely used.

# HTTP client (for remote install/search)
reqwest = { version = "0.12", features = ["rustls-tls", "json"] }
# rustls-tls avoids OpenSSL system dependency.
# json feature enables .json() on Response.

# Time
chrono.workspace = true

# Versioning
semver.workspace = true

# ID generation
uuid.workspace = true

# Hashing (integrity checks)
sha2 = "0.10"
# SHA-256 for verifying downloaded artifacts match expected hashes.

# Logging
tracing.workspace = true
tracing-subscriber.workspace = true

# Platform-specific directories
dirs = "6"
# Resolves ~/.agentvault on Linux, ~/Library/Application Support/agentvault
# on macOS, %APPDATA%\agentvault on Windows.
```

### `vault-cli/Cargo.toml`

```toml
[package]
name = "vault-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "vault"
path = "src/main.rs"

[dependencies]
vault-core = { path = "../vault-core" }
vault-connectors = { path = "../vault-connectors" }

# CLI framework
clap = { version = "4", features = ["derive", "env", "wrap_help"] }
# derive: #[derive(Parser)] for zero-boilerplate arg parsing.
# env: read defaults from environment variables.
# wrap_help: auto-wrap long help text to terminal width.

# Error handling at the CLI boundary
anyhow.workspace = true
# anyhow provides .context("doing X") for ad-hoc error wrapping.
# Used ONLY in vault-cli, never in vault-core.

# Terminal output
owo-colors = "4"
# Fast, zero-dependency terminal color library.
# Chosen over `colored` for smaller binary size and better API.

indicatif = "0.17"
# Progress bars and spinners for long-running operations (install, sync).

tabled = "0.17"
# ASCII table rendering for `vault list` and `vault status` output.

dialoguer = "0.11"
# Interactive prompts (confirm, select, multi-select) for destructive
# operations like `vault remove` without --force.

# Shared workspace deps
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
chrono.workspace = true
```

### `vault-connectors/Cargo.toml`

```toml
[package]
name = "vault-connectors"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
vault-core = { path = "../vault-core" }

# Serialization (reading/writing agent config files)
serde.workspace = true
serde_json.workspace = true
toml.workspace = true

# Async
tokio.workspace = true
async-trait = "0.1"

# Error handling
thiserror.workspace = true

# Logging
tracing.workspace = true

# Time
chrono.workspace = true

# Platform paths
dirs = "6"
```

### Dependency Justification Summary

| Crate               | Version  | Why                                                    |
|----------------------|----------|--------------------------------------------------------|
| `clap`              | 4.x      | Industry-standard CLI parser; `derive` eliminates boilerplate |
| `serde` + `serde_json` | 1.x   | De-facto serialization; needed for JSON agent configs  |
| `toml`              | 0.8      | TOML parsing/writing; config files and manifests       |
| `rusqlite`          | 0.31     | SQLite bindings; `bundled` avoids system deps          |
| `tokio`             | 1.x      | Async runtime; needed for HTTP, concurrent sync        |
| `reqwest`           | 0.12     | HTTP client for downloads and registry queries         |
| `indicatif`         | 0.17     | Progress bars for long operations                      |
| `owo-colors`        | 4.x      | Zero-dep terminal colors                               |
| `thiserror`         | 2.x      | Derive `Error` + `Display` for `VaultError`            |
| `anyhow`            | 1.x      | CLI-only error context (`.context()`)                  |
| `dirs`              | 6.x      | Cross-platform `~/.agentvault` resolution              |
| `chrono`            | 0.4      | Timestamps in registry and logs                        |
| `semver`            | 1.x      | Parse and compare semver versions and ranges            |
| `sha2`              | 0.10     | SHA-256 integrity hashes for installed artifacts       |
| `tracing`           | 0.1      | Structured logging throughout the stack                |
| `tracing-subscriber`| 0.3      | Log output formatting (console + file)                 |
| `dialoguer`         | 0.11     | Interactive terminal prompts                           |
| `tabled`            | 0.17     | Render ASCII tables for `list` and `status`            |
| `async-trait`       | 0.1      | Async methods in traits (dyn dispatch)                 |
| `uuid`              | 1.x      | Generate unique IDs for registry entries and sync logs |

---

## 9. Data Flow Diagrams

### 9.1 Install Flow

```
User runs: vault install @anthropic/mcp-filesystem --agents claude,gemini

┌────────┐
│ vault  │
│  CLI   │
└───┬────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ 1. Parse CLI arguments (clap)               │
│    name    = "@anthropic/mcp-filesystem"     │
│    source  = npm (auto-detected from @scope) │
│    agents  = ["claude", "gemini"]            │
│    version = None (latest)                   │
│    force   = false                           │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 2. Build InstallRequest                     │
│    Detect source type from input:           │
│    • Starts with @ or looks like npm pkg    │
│      → InstallSource::Npm                   │
│    • Starts with https://github.com         │
│      → InstallSource::Git                   │
│    • Starts with / or ./                    │
│      → InstallSource::LocalPath             │
│    • Otherwise → error                      │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 3. Check for conflicts                      │
│    registry.get("filesystem", Mcp)?         │
│    • Found + !force → VaultError::AlreadyExists │
│    • Found + force  → will overwrite        │
│    • Not found      → proceed               │
│                                             │
│    resolver.check_conflicts(                │
│        "filesystem", version, installed     │
│    )?                                       │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 4. Download / Install artifact              │
│                                             │
│    match source {                           │
│      Npm { package, version } => {          │
│        // Run: npm install <pkg>@<ver>      │
│        // in store/mcp/<name>/              │
│        // Show progress bar (indicatif)     │
│      }                                      │
│      Git { url, ref_ } => {                │
│        // git clone --depth 1 --branch <ref>│
│        // into store/mcp/<name>/            │
│      }                                      │
│      LocalPath(path) => {                   │
│        // Symlink or copy into store        │
│      }                                      │
│      Url(url) => {                          │
│        // HTTP GET → store/mcp/<name>/      │
│      }                                      │
│    }                                        │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 5. Compute integrity hash                   │
│    Walk all files in store/mcp/<name>/      │
│    Compute SHA-256 of content tree          │
│    Store hash in registry entry             │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 6. Register in SQLite                       │
│    INSERT INTO capabilities (               │
│      id, name, version, kind, source,       │
│      install_path, integrity_hash,          │
│      status, installed_at, updated_at       │
│    )                                        │
│                                             │
│    INSERT INTO capability_agents (          │
│      capability_id, agent_id, sync_status   │
│    ) for each agent in target_agents        │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 7. Write per-capability manifest.toml       │
│    at store/mcp/<name>/manifest.toml        │
│    (see §5.3 for format)                    │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 8. Output result to terminal                │
│                                             │
│    ✓ Installed filesystem v1.2.5 (npm)      │
│      Path: ~/.agentvault/store/mcp/filesys  │
│      Agents: claude, gemini                 │
│      Run `vault sync` to push to agents.    │
└─────────────────────────────────────────────┘
```

### 9.2 Sync Flow

```
User runs: vault sync --agents claude

┌────────┐
│ vault  │
│  CLI   │
└───┬────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ 1. Load vault state from registry           │
│    registry.list(ListFilter {               │
│        agent: Some("claude"),               │
│        installed_only: true,                │
│    })                                       │
│    → vault_caps: [filesystem, postgres, ...]│
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 2. Load agent's current state               │
│    let connector = ClaudeConnector::new()?;  │
│    connector.detect()?                       │
│    → AgentStatus::Installed { version }     │
│                                             │
│    connector.read_capabilities()?            │
│    → agent_caps: [filesystem(old), git, ...]│
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 3. Compute diff                             │
│    connector.diff(&vault_caps)?              │
│                                             │
│    SyncDiff {                               │
│      to_add:    [postgres]                  │
│      to_update: [filesystem (1.2.0→1.2.5)] │
│      to_remove: []                          │
│      in_sync:   []                          │
│      unmanaged: [git]  (in agent, not vault)│
│    }                                        │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 4. Display plan and confirm                 │
│                                             │
│    Sync Plan for Claude Code:               │
│    ┌──────────┬──────────┬────────────────┐ │
│    │ Action   │ Name     │ Details        │ │
│    ├──────────┼──────────┼────────────────┤ │
│    │ ADD      │ postgres │ v0.5.2         │ │
│    │ UPDATE   │ filesys  │ 1.2.0 → 1.2.5 │ │
│    │ SKIP     │ git      │ unmanaged      │ │
│    └──────────┴──────────┴────────────────┘ │
│                                             │
│    Proceed? [Y/n]                           │
└───────────────────────┬─────────────────────┘
                        │ (user confirms)
                        ▼
┌─────────────────────────────────────────────┐
│ 5. Create backup                            │
│    connector.backup()?                       │
│    → ~/.agentvault/backups/claude/2026-.../ │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 6. Apply changes                            │
│    connector.apply(&actions, &options)?      │
│                                             │
│    For Claude, this means:                  │
│    • Read claude_desktop_config.json        │
│    • Add "postgres" entry to mcpServers     │
│    • Update "filesystem" version/args       │
│    • Write back claude_desktop_config.json  │
│    • Also update .claude/settings.json      │
│      for CLI-mode MCP settings              │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 7. Verify                                   │
│    connector.validate()?                     │
│    • Parse the written config back          │
│    • Check for JSON syntax errors           │
│    • Verify all referenced paths exist      │
│                                             │
│    If validation fails:                     │
│      connector.restore(backup_path)?        │
│      Report error                           │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 8. Update registry sync status              │
│    UPDATE capability_agents                  │
│      SET synced_at = now(),                 │
│          sync_status = 'synced'             │
│      WHERE agent_id = 'claude'             │
│                                             │
│    registry.log_sync("claude", entries)     │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 9. Output result                            │
│                                             │
│    ✓ Sync complete for Claude Code          │
│      Added:   1 (postgres)                  │
│      Updated: 1 (filesystem)                │
│      Skipped: 1 (git — unmanaged)           │
│      Backup:  ~/.agentvault/backups/clau... │
└─────────────────────────────────────────────┘
```

### 9.3 Search Flow

```
User runs: vault search "filesystem" --source all

┌────────┐
│ vault  │
│  CLI   │
└───┬────┘
    │
    ▼
┌─────────────────────────────────────────────┐
│ 1. Parse query and source                    │
│    query  = "filesystem"                     │
│    source = SearchSource::All                │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
         ┌──────────────┴──────────────┐
         │                             │
         ▼                             ▼
┌──────────────────┐     ┌──────────────────────┐
│ 2a. Local Search │     │ 2b. Remote Search     │
│                  │     │     (concurrent)      │
│ registry.query(  │     │                       │
│   name LIKE      │     │ npm search:           │
│   "%filesystem%" │     │   GET npmjs.com/...   │
│ )                │     │   → npm_results       │
│ → local_results  │     │                       │
│                  │     │ GitHub search:         │
│                  │     │   GET api.github.com  │
│                  │     │   → gh_results        │
└────────┬─────────┘     └──────────┬───────────┘
         │                          │
         └──────────┬───────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│ 3. Merge and deduplicate                     │
│                                              │
│    combined = local_results                  │
│               + npm_results                  │
│               + gh_results                   │
│                                              │
│    Dedup by name (prefer local if installed) │
│    Sort by relevance (exact match first,     │
│           then substring, then fuzzy)        │
│    Mark .installed = true for local matches  │
└───────────────────────┬─────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────┐
│ 4. Display results (tabled)                  │
│                                              │
│ Search results for "filesystem":             │
│ ┌───┬──────────────────┬───────┬──────────┐  │
│ │   │ Name             │ Ver   │ Source   │  │
│ ├───┼──────────────────┼───────┼──────────┤  │
│ │ ✓ │ filesystem       │ 1.2.5 │ npm      │  │
│ │   │ filesystem-extra │ 0.3.0 │ npm      │  │
│ │   │ fs-mcp           │ 2.0.0 │ github   │  │
│ └───┴──────────────────┴───────┴──────────┘  │
│                                              │
│ ✓ = installed locally                        │
│                                              │
│ Install with: vault install <name>           │
└─────────────────────────────────────────────┘
```

---

## 10. Testing Strategy

### 10.1 Unit Tests

Every module in `vault-core` has a `#[cfg(test)] mod tests` section. Dependencies are mocked via traits.

**Example: Testing the registry**

```rust
// vault-core/src/registry.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn test_registry() -> SqliteRegistry {
        let tmp = NamedTempFile::new().unwrap();
        SqliteRegistry::open(tmp.path()).unwrap()
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let reg = test_registry();
        let entry = RegistryEntry {
            id: uuid::Uuid::new_v4(),
            name: "test-mcp".into(),
            version: semver::Version::new(1, 0, 0),
            kind: CapabilityKind::Mcp,
            source: r#"{"type":"npm","package":"test-mcp"}"#.into(),
            install_path: "/tmp/test".into(),
            integrity_hash: Some("abc123".into()),
            agents: vec!["claude".into()],
            status: CapabilityStatus::Active,
            installed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: None,
        };

        reg.insert(&entry).unwrap();
        let got = reg.get("test-mcp", CapabilityKind::Mcp).unwrap();
        assert_eq!(got.name, "test-mcp");
        assert_eq!(got.version, semver::Version::new(1, 0, 0));
    }

    #[test]
    fn insert_duplicate_returns_already_exists() {
        let reg = test_registry();
        let entry = /* ... */;
        reg.insert(&entry).unwrap();

        let result = reg.insert(&entry);
        assert!(matches!(result, Err(VaultError::AlreadyExists { .. })));
    }

    #[test]
    fn remove_nonexistent_returns_not_found() {
        let reg = test_registry();
        let result = reg.remove("nonexistent", CapabilityKind::Mcp);
        assert!(matches!(result, Err(VaultError::NotFound { .. })));
    }

    #[test]
    fn query_filters_by_kind() {
        let reg = test_registry();
        // Insert 2 MCPs and 1 Skill...
        let results = reg.query(&RegistryQuery {
            kind: Some(CapabilityKind::Mcp),
            ..Default::default()
        }).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn migrations_are_idempotent() {
        let reg = test_registry();
        reg.migrate().unwrap(); // Already called in open()
        reg.migrate().unwrap(); // Should not fail
    }
}
```

**Example: Mock registry for testing managers**

```rust
// vault-core/src/test_helpers.rs (cfg(test) only)

pub struct MockRegistry {
    entries: std::sync::Mutex<Vec<RegistryEntry>>,
}

impl MockRegistry {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Registry for MockRegistry {
    fn insert(&self, entry: &RegistryEntry) -> Result<(), VaultError> {
        let mut entries = self.entries.lock().unwrap();
        if entries.iter().any(|e| e.name == entry.name && e.kind == entry.kind) {
            return Err(VaultError::AlreadyExists {
                name: entry.name.clone(),
                kind: format!("{:?}", entry.kind),
                existing_version: entries
                    .iter()
                    .find(|e| e.name == entry.name)
                    .unwrap()
                    .version
                    .to_string(),
            });
        }
        entries.push(entry.clone());
        Ok(())
    }

    // ... other trait methods with in-memory implementations
}
```

### 10.2 Integration Tests

Located in `tests/` at the workspace root. These use real SQLite databases and real filesystem operations in temporary directories.

```rust
// tests/cli_tests.rs

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Helper: create a fresh vault environment in a temp dir.
fn vault_env() -> (TempDir, Command) {
    let dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("vault").unwrap();
    cmd.env("AGENTVAULT_HOME", dir.path());
    (dir, cmd)
}

#[test]
fn init_creates_config_and_db() {
    let (dir, mut cmd) = vault_env();
    cmd.args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized AgentVault"));

    assert!(dir.path().join("config.toml").exists());
    assert!(dir.path().join("registry.db").exists());
    assert!(dir.path().join("store").is_dir());
}

#[test]
fn list_empty_shows_no_capabilities() {
    let (_dir, mut cmd) = vault_env();
    // Init first
    Command::cargo_bin("vault").unwrap()
        .env("AGENTVAULT_HOME", _dir.path())
        .args(["init"])
        .assert()
        .success();

    cmd.args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No capabilities installed"));
}

#[test]
fn install_local_mcp_succeeds() {
    let (dir, _) = vault_env();
    // Create a fake MCP directory with a package.json
    let mcp_dir = dir.path().join("fake-mcp");
    std::fs::create_dir_all(&mcp_dir).unwrap();
    std::fs::write(
        mcp_dir.join("package.json"),
        r#"{"name":"fake-mcp","version":"1.0.0"}"#,
    ).unwrap();

    // Init
    Command::cargo_bin("vault").unwrap()
        .env("AGENTVAULT_HOME", dir.path())
        .args(["init"])
        .assert()
        .success();

    // Install from local path
    Command::cargo_bin("vault").unwrap()
        .env("AGENTVAULT_HOME", dir.path())
        .args(["install", "--source", "local", "--path", mcp_dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed fake-mcp"));
}

#[test]
fn remove_nonexistent_fails_with_not_found() {
    let (dir, mut cmd) = vault_env();
    Command::cargo_bin("vault").unwrap()
        .env("AGENTVAULT_HOME", dir.path())
        .args(["init"])
        .assert()
        .success();

    cmd.args(["remove", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not found"));
}
```

### 10.3 Sync Tests

```rust
// tests/sync_tests.rs

use tempfile::TempDir;

/// Simulate a Claude config directory and test sync.
#[test]
fn sync_adds_mcp_to_claude_config() {
    let vault_home = TempDir::new().unwrap();
    let claude_home = TempDir::new().unwrap();

    // Create a minimal Claude config
    let config_path = claude_home.path().join("claude_desktop_config.json");
    std::fs::write(&config_path, r#"{"mcpServers":{}}"#).unwrap();

    // Init vault, install an MCP, then sync
    // ... (using programmatic API, not CLI, for faster tests)

    // Assert the Claude config now contains the MCP
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&config_path).unwrap()
    ).unwrap();

    assert!(config["mcpServers"]["filesystem"].is_object());
}
```

### 10.4 Snapshot Tests

Using `insta` for deterministic output testing:

```rust
// vault-core/src/config.rs

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn default_config_toml_snapshot() {
        let config = VaultConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert_snapshot!("default_config", toml_str);
    }

    #[test]
    fn default_manifest_toml_snapshot() {
        let manifest = VaultManifest::default();
        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        assert_snapshot!("default_manifest", toml_str);
    }
}
```

### 10.5 Testing Dependencies

```toml
# Workspace root Cargo.toml (dev-dependencies section)

[workspace.dependencies]
# Testing
assert_cmd = "2"          # CLI binary testing
predicates = "3"          # Assertion helpers for assert_cmd
tempfile = "3"            # Temporary directories for integration tests
insta = "1"               # Snapshot testing
mockall = "0.13"          # Auto-generate mock impls from traits
```

### 10.6 Coverage Target

| Scope                | Target | Strategy                                    |
|----------------------|--------|---------------------------------------------|
| `vault-core`         | 85%+   | Unit tests per module + mock dependencies   |
| `vault-connectors`   | 75%+   | Integration tests with fake agent configs   |
| `vault-cli`          | 70%+   | `assert_cmd` tests for each subcommand      |
| Overall              | 80%+   | CI enforced via `cargo llvm-cov`            |

**CI Coverage Command:**

```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Run with coverage
cargo llvm-cov --workspace --lcov --output-path lcov.info

# Fail CI if below threshold
cargo llvm-cov --workspace --fail-under-lines 80
```

### 10.7 Test Organization Conventions

```
1. Each #[cfg(test)] mod tests block lives at the BOTTOM of its file.
2. Test helper functions go in a dedicated test_helpers.rs (cfg(test) only).
3. Integration tests in tests/ use real binaries and real databases.
4. Snapshot files live in src/snapshots/ (managed by insta).
5. Test names follow: <thing_being_tested>_<condition>_<expected_result>
   Examples:
     insert_duplicate_returns_already_exists
     sync_dry_run_does_not_modify_files
     resolve_circular_dependency_returns_cycle_error
```

---

## Appendix A: Filesystem Paths by Platform

| Purpose           | Linux                          | macOS                                    | Windows                          |
|--------------------|--------------------------------|------------------------------------------|----------------------------------|
| Vault home         | `~/.agentvault/`               | `~/Library/Application Support/agentvault/` | `%APPDATA%\agentvault\`         |
| Config file        | `~/.agentvault/config.toml`    | (vault home)/config.toml                 | (vault home)\config.toml        |
| SQLite DB          | `~/.agentvault/registry.db`    | (vault home)/registry.db                 | (vault home)\registry.db        |
| Store              | `~/.agentvault/store/`         | (vault home)/store/                      | (vault home)\store\             |
| Backups            | `~/.agentvault/backups/`       | (vault home)/backups/                    | (vault home)\backups\           |
| Logs               | `~/.agentvault/logs/`          | (vault home)/logs/                       | (vault home)\logs\              |

> All paths are resolved at runtime via the `dirs` crate's `data_dir()` function, with a fallback to `$HOME/.agentvault` on Unix systems.

## Appendix B: Agent Config File Locations

| Agent       | Config File(s)                                                   | Format |
|-------------|------------------------------------------------------------------|--------|
| Claude Code | `~/.claude/claude_desktop_config.json`, `~/.claude/settings.json` | JSON   |
| Gemini CLI  | `~/.gemini/config/settings.json`                                  | JSON   |
| OpenCode    | `~/.opencode/config.json`                                         | JSON   |
| Codex CLI   | `~/.codex/config.json`                                            | JSON   |

> These paths are hardcoded in each connector but are overridable via environment variables (`CLAUDE_HOME`, `GEMINI_CONFIG_DIR`, etc.) for testing.

---

*This document is the implementation-ready specification for AgentVault's core architecture. All code should be written to match the traits, types, and flows described above. Deviations should be documented as Architecture Decision Records (ADRs) in `docs/decisions/`.*
