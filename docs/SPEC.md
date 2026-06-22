# AgentVault — Technical Specification

> **Version:** 0.1.0-draft  
> **Status:** Implementation-ready  
> **Last Updated:** 2026-06-22  
> **Authors:** Aswinkumar GP  

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Directory Structure](#2-directory-structure)
3. [Data Models](#3-data-models)
4. [SQLite Schema](#4-sqlite-schema)
5. [CLI Interface](#5-cli-interface)
6. [Agent Connector Specifications](#6-agent-connector-specifications)
7. [Sync Algorithm](#7-sync-algorithm)
8. [vault.toml Manifest Format](#8-vaulttoml-manifest-format)
9. [Error Handling Strategy](#9-error-handling-strategy)
10. [Security Model](#10-security-model)
11. [Plugin Architecture](#11-plugin-architecture)

---

## 1. System Overview

AgentVault is a **local-first capability management system** for AI coding agents, built in Rust. It provides a unified CLI to install, manage, and synchronize MCP servers, skills, and workflows across multiple AI agents (Claude Code, Gemini CLI, OpenCode, Codex CLI, etc.).

### 1.1 Design Principles

| Principle | Description |
|---|---|
| **Local-first** | All data stored on the user's machine. No cloud dependency for core operations. |
| **Non-destructive** | Agent configs are never overwritten without backup. Merge semantics preserve user entries. |
| **Declarative** | `vault.toml` defines the desired state; `vault sync` reconciles reality to match. |
| **Extensible** | New agent connectors are added by implementing a single trait. |
| **Transparent** | Every sync operation is logged with full diffs. `--dry-run` available everywhere. |

### 1.2 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLI Layer (clap)                            │
│  vault install │ vault sync │ vault list │ vault doctor │ ...       │
└──────────┬──────────────────┬───────────────────┬───────────────────┘
           │                  │                   │
           ▼                  ▼                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Command Handlers                              │
│  InstallHandler │ SyncHandler │ ListHandler │ DoctorHandler │ ...   │
└──────────┬──────────────────┬───────────────────┬───────────────────┘
           │                  │                   │
           ▼                  ▼                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Core Engine                                  │
│ ┌──────────────┐ ┌──────────────┐ ┌────────────────┐ ┌───────────┐ │
│ │  MCP Manager │ │Skill Manager │ │Workflow Manager│ │ Capability│ │
│ │              │ │              │ │                │ │ Resolver  │ │
│ │ install()    │ │ install()    │ │ install()      │ │           │ │
│ │ remove()     │ │ remove()     │ │ remove()       │ │ resolve() │ │
│ │ update()     │ │ get()        │ │ validate()     │ │ diff()    │ │
│ │ list()       │ │ list()       │ │ list()         │ │ merge()   │ │
│ └──────┬───────┘ └──────┬───────┘ └───────┬────────┘ └─────┬─────┘ │
│        │                │                 │                │       │
└────────┼────────────────┼─────────────────┼────────────────┼───────┘
         │                │                 │                │
         ▼                ▼                 ▼                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Connector Layer                                │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────────┐      │
│ │   Claude    │ │   Gemini   │ │  OpenCode  │ │   Codex CLI  │      │
│ │ Connector   │ │ Connector  │ │ Connector  │ │  Connector   │      │
│ │            │ │            │ │            │ │              │      │
│ │ read_cfg() │ │ read_cfg() │ │ read_cfg() │ │  read_cfg()  │      │
│ │ write_cfg()│ │ write_cfg()│ │ write_cfg()│ │  write_cfg() │      │
│ │ sync()     │ │ sync()     │ │ sync()     │ │  sync()      │      │
│ │ diff()     │ │ diff()     │ │ diff()     │ │  diff()      │      │
│ └─────┬──────┘ └─────┬──────┘ └─────┬──────┘ └──────┬───────┘      │
│       │              │              │               │              │
└───────┼──────────────┼──────────────┼───────────────┼──────────────┘
        │              │              │               │
        ▼              ▼              ▼               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Storage Layer                                 │
│ ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐     │
│ │  SQLite Registry │ │ Filesystem Store │ │   Config Store   │     │
│ │  (registry.db)   │ │  (mcps/, skills/)│ │ (config.toml,    │     │
│ │                  │ │                  │ │  vault.toml)     │     │
│ │  mcps            │ │  MCP binaries    │ │                  │     │
│ │  skills          │ │  Skill files     │ │  Global config   │     │
│ │  workflows       │ │  Workflow defs   │ │  Manifest        │     │
│ │  capabilities    │ │  Backups         │ │  Env secrets     │     │
│ │  agent_configs   │ │  Logs            │ │                  │     │
│ │  sync_history    │ │  Cache           │ │                  │     │
│ └──────────────────┘ └──────────────────┘ └──────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.3 Crate Layout

AgentVault is a Cargo workspace with three crates:

| Crate | Purpose | Key Dependencies |
|---|---|---|
| `vault-cli` | Binary crate. CLI parsing, output formatting, user interaction. | `clap`, `colored`, `indicatif`, `dialoguer`, `tabled`, `anyhow`, `tracing-subscriber` |
| `vault-core` | Library crate. Data models, business logic, storage, managers. | `serde`, `toml`, `serde_json`, `rusqlite`, `semver`, `chrono`, `dirs`, `sha2`, `thiserror`, `tracing`, `reqwest`, `tokio` |
| `vault-connectors` | Library crate. Agent connector implementations. Depends on `vault-core`. | `vault-core`, `serde_json`, `toml`, `async-trait` |

```
AgentVault/
├── Cargo.toml                    # Workspace root
├── vault-cli/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # Entry point, tracing setup
│       ├── cli.rs                 # Clap derive definitions
│       └── handlers/             # Command handler modules
│           ├── mod.rs
│           ├── install.rs
│           ├── remove.rs
│           ├── update.rs
│           ├── list.rs
│           ├── search.rs
│           ├── sync.rs
│           ├── status.rs
│           ├── config.rs
│           ├── doctor.rs
│           ├── connector.rs
│           ├── export.rs
│           └── import.rs
├── vault-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs              # VaultError enum
│       ├── models/
│       │   ├── mod.rs
│       │   ├── mcp.rs            # McpEntry, McpSource, McpTransport
│       │   ├── skill.rs          # SkillEntry
│       │   ├── workflow.rs       # WorkflowEntry, WorkflowStep
│       │   ├── capability.rs     # CapabilityEntry
│       │   ├── agent.rs          # AgentConnectorConfig, AgentType
│       │   ├── config.rs         # VaultConfig
│       │   └── manifest.rs       # VaultManifest
│       ├── registry/
│       │   ├── mod.rs            # Registry trait
│       │   ├── sqlite.rs         # SqliteRegistry implementation
│       │   └── migrations.rs     # Schema migrations
│       ├── managers/
│       │   ├── mod.rs
│       │   ├── mcp.rs            # McpManager
│       │   ├── skill.rs          # SkillManager
│       │   ├── workflow.rs       # WorkflowManager
│       │   └── capability.rs     # CapabilityResolver
│       └── storage/
│           ├── mod.rs
│           ├── filesystem.rs     # Filesystem operations
│           └── config.rs         # Config file R/W
└── vault-connectors/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── traits.rs             # AgentConnector trait
        ├── registry.rs           # ConnectorRegistry
        ├── claude.rs             # Claude Code connector
        ├── gemini.rs             # Gemini CLI connector
        ├── opencode.rs           # OpenCode connector
        └── codex.rs              # Codex CLI connector
```

---

## 2. Directory Structure

The `~/.agentvault/` directory is the central data store. Created by `vault init` and referenced by all operations.

```
~/.agentvault/
├── config.toml                    # Global AgentVault configuration
├── vault.toml                     # Capability manifest (declarative desired-state)
├── registry.db                    # SQLite registry (single-file database)
│
├── mcps/                          # Installed MCP servers
│   ├── filesystem/                # Example: @anthropic/mcp-filesystem
│   │   ├── manifest.toml          # MCP metadata: name, version, source, env, args
│   │   ├── node_modules/          # npm-sourced MCP dependencies
│   │   └── package.json           # npm package manifest
│   ├── github/                    # Example: @anthropic/mcp-github
│   │   ├── manifest.toml
│   │   ├── node_modules/
│   │   └── package.json
│   └── mcp-memory/                # Example: PyPI-sourced MCP
│       ├── manifest.toml
│       ├── .venv/                  # Isolated Python venv
│       └── pyproject.toml
│
├── skills/                        # Installed skills
│   └── git-workflow/              # Example skill
│       ├── skill.toml             # Skill metadata: name, description, tags
│       └── SKILL.md               # Skill instructions (YAML frontmatter + markdown)
│
├── workflows/                     # Workflow definitions
│   └── full-review/               # Example workflow
│       └── workflow.toml          # Steps, dependencies, config
│
├── connectors/                    # Agent connector state
│   ├── claude.toml                # Claude connector config & sync metadata
│   ├── gemini.toml                # Gemini connector config
│   └── opencode.toml              # OpenCode connector config
│
├── cache/                         # Download cache (tarballs, git clones)
│   ├── npm/                       # Cached npm tarballs
│   ├── pypi/                      # Cached wheels/sdists
│   └── git/                       # Cached git repos (bare clones)
│
├── logs/                          # Operation logs
│   ├── vault.log                  # Rolling log file (tracing output)
│   └── sync/                      # Per-sync detailed logs
│       └── 2026-06-22T19-30-00_claude.log
│
└── backups/                       # Agent config backups before sync
    ├── claude/
    │   ├── 2026-06-22T19-30-00.json
    │   └── 2026-06-21T14-00-00.json
    ├── gemini/
    │   └── 2026-06-22T19-30-00.json
    └── opencode/
        └── 2026-06-22T19-30-00.json
```

### 2.1 MCP `manifest.toml` Format

Each installed MCP has a `manifest.toml` in its directory:

```toml
[mcp]
id = "01J5XQ9Z..."                  # ULID, globally unique
name = "filesystem"
display_name = "Filesystem Access"
version = "0.6.2"

[source]
type = "npm"
package = "@anthropic/mcp-filesystem"

[transport]
type = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"]

[env]
# Empty for this MCP

[metadata]
installed_at = "2026-06-22T14:00:00Z"
updated_at = "2026-06-22T14:00:00Z"
status = "active"
checksum = "sha256:abcdef1234567890..."
```

### 2.2 Skill `skill.toml` Format

```toml
[skill]
id = "01J5XQ9Z..."
name = "git-workflow"
description = "Structures git workflow practices for branching, committing, and versioning."
version = "1.0.0"
tags = ["git", "workflow", "versioning"]

[source]
type = "git"
repo = "https://github.com/user/agent-skills"
ref = "main"
subdirectory = "skills/git-workflow"

[metadata]
installed_at = "2026-06-22T14:30:00Z"
```

---

## 3. Data Models

All data models are defined in `vault-core/src/models/` with full `serde` derives for serialization to both TOML (manifest files) and JSON (SQLite storage, API output).

### 3.1 MCP Models

```rust
// vault-core/src/models/mcp.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A fully-qualified MCP server entry in the vault registry.
///
/// This is the central data structure for an installed MCP. It contains
/// everything needed to:
/// 1. Locate the MCP on disk
/// 2. Launch it (command + args + env)
/// 3. Sync it to any agent connector
/// 4. Track its lifecycle (version, install date, status)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpEntry {
    /// Unique identifier (ULID format: 01J5XQ9Z...).
    /// Generated at install time, immutable thereafter.
    pub id: String,

    /// Short machine-friendly name, used as the directory name under ~/.agentvault/mcps/
    /// and as the key in agent config files (e.g., "filesystem", "github", "memory").
    /// Must be unique within the vault. Validated: [a-z0-9][a-z0-9-]* (lowercase, hyphens ok).
    pub name: String,

    /// Optional human-friendly display name for CLI output.
    /// Example: "Filesystem Access", "GitHub Integration".
    pub display_name: Option<String>,

    /// Installed version string. Follows semver when the source supports it.
    /// For git sources, this may be a commit SHA or tag.
    pub version: String,

    /// Where the MCP was installed from.
    pub source: McpSource,

    /// Absolute path to the MCP's installation directory.
    /// Example: /home/user/.agentvault/mcps/filesystem/
    pub install_path: PathBuf,

    /// The executable command to launch this MCP server.
    /// Examples: "npx", "node", "python", "uvx", "/usr/local/bin/mcp-server"
    pub command: String,

    /// Arguments passed to the command.
    /// Example: ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"]
    pub args: Vec<String>,

    /// Environment variables required by this MCP server.
    /// Keys are var names, values are either literal values or "env:VAR_NAME"
    /// references that delegate to the host environment.
    /// Example: { "GITHUB_TOKEN": "env:GITHUB_TOKEN", "LOG_LEVEL": "info" }
    pub env_vars: HashMap<String, String>,

    /// Transport protocol the MCP server uses.
    pub transport: McpTransport,

    /// Current operational status.
    pub status: McpStatus,

    /// Timestamp when this MCP was first installed.
    pub installed_at: DateTime<Utc>,

    /// Timestamp of the most recent update (version change, config change).
    pub updated_at: DateTime<Utc>,

    /// SHA-256 checksum of the installed package (for integrity verification).
    /// Format: "sha256:<hex>"
    pub checksum: Option<String>,

    /// Which agents this MCP should be synced to.
    /// If empty, synced to all registered agents.
    /// Example: ["claude", "gemini"]
    pub agents: Vec<String>,

    /// Free-form tags for categorization and search.
    pub tags: Vec<String>,

    /// Optional human-readable description.
    pub description: Option<String>,
}

/// Where an MCP server was sourced from.
/// Determines the install strategy (npm install, pip install, git clone, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpSource {
    /// Installed from the npm registry.
    /// Example: @anthropic/mcp-filesystem
    Npm {
        /// Full npm package name (scoped or unscoped).
        package: String,
    },

    /// Installed from the Python Package Index.
    /// Example: mcp-server-memory
    PyPi {
        /// PyPI package name.
        package: String,
    },

    /// Cloned from a GitHub repository (or any git remote).
    /// Example: github.com/anthropics/mcp-servers
    GitHub {
        /// Repository in "owner/repo" format or full URL.
        repo: String,

        /// Optional git ref: branch name, tag, or commit SHA.
        /// Defaults to the repository's default branch if None.
        #[serde(rename = "ref")]
        ref_: Option<String>,
    },

    /// Linked from a local filesystem path.
    /// The MCP is not copied; a symlink is created.
    Local {
        /// Absolute path to the MCP server directory.
        path: PathBuf,
    },

    /// Pulled from a Docker/OCI container image.
    /// The MCP runs inside a container.
    Docker {
        /// Full image reference: "registry/image:tag"
        image: String,
    },
}

/// Transport protocol for communicating with the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    /// Standard I/O transport (stdin/stdout).
    /// The most common transport for locally-installed MCP servers.
    Stdio,

    /// Server-Sent Events over HTTP.
    /// Used for remote or long-running MCP servers.
    Sse {
        /// The SSE endpoint URL.
        /// Example: "http://localhost:3001/sse"
        url: String,
    },

    /// Streamable HTTP transport (newer MCP protocol variant).
    StreamableHttp {
        /// The HTTP endpoint URL.
        /// Example: "http://localhost:3001/mcp"
        url: String,
    },
}

/// Operational status of an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum McpStatus {
    /// MCP is installed and ready to use.
    Active,

    /// MCP is installed but intentionally disabled by the user.
    /// Will be excluded from sync operations.
    Disabled,

    /// MCP is in an error state. The message describes what went wrong.
    Error {
        /// Human-readable error description.
        message: String,
    },
}

impl Default for McpStatus {
    fn default() -> Self {
        McpStatus::Active
    }
}

impl std::fmt::Display for McpSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpSource::Npm { package } => write!(f, "npm:{}", package),
            McpSource::PyPi { package } => write!(f, "pypi:{}", package),
            McpSource::GitHub { repo, ref_ } => {
                write!(f, "github:{}", repo)?;
                if let Some(r) = ref_ {
                    write!(f, "@{}", r)?;
                }
                Ok(())
            }
            McpSource::Local { path } => write!(f, "local:{}", path.display()),
            McpSource::Docker { image } => write!(f, "docker:{}", image),
        }
    }
}

impl std::fmt::Display for McpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpTransport::Stdio => write!(f, "stdio"),
            McpTransport::Sse { url } => write!(f, "sse:{}", url),
            McpTransport::StreamableHttp { url } => write!(f, "http:{}", url),
        }
    }
}

impl std::fmt::Display for McpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpStatus::Active => write!(f, "active"),
            McpStatus::Disabled => write!(f, "disabled"),
            McpStatus::Error { message } => write!(f, "error: {}", message),
        }
    }
}
```

### 3.2 Skill Models

```rust
// vault-core/src/models/skill.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// An installed skill in the vault.
///
/// Skills are directories containing a SKILL.md instruction file
/// and optional supporting resources (scripts, examples, references).
/// They provide domain-specific instructions that AI agents can load
/// to modify their behavior for particular tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillEntry {
    /// Unique identifier (ULID).
    pub id: String,

    /// Short machine-friendly name, used as directory name under ~/.agentvault/skills/.
    /// Must be unique within the vault. Validated: [a-z0-9][a-z0-9-]*
    pub name: String,

    /// Human-readable description of what this skill does.
    pub description: Option<String>,

    /// Absolute path to the skill directory.
    /// Example: /home/user/.agentvault/skills/git-workflow/
    pub path: PathBuf,

    /// Free-form tags for categorization and search.
    /// Example: ["git", "workflow", "versioning"]
    pub tags: Vec<String>,

    /// Where the skill was sourced from.
    pub source: SkillSource,

    /// Timestamp when this skill was first installed.
    pub installed_at: DateTime<Utc>,

    /// Which agents this skill should be synced to.
    /// If empty, synced to all registered agents that support skills.
    pub agents: Vec<String>,
}

/// Where a skill was sourced from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillSource {
    /// Cloned from a git repository.
    Git {
        repo: String,
        #[serde(rename = "ref")]
        ref_: Option<String>,
        /// Optional subdirectory within the repo containing the skill.
        subdirectory: Option<String>,
    },

    /// Linked from a local filesystem path.
    Local {
        path: PathBuf,
    },
}
```

### 3.3 Workflow Models

```rust
// vault-core/src/models/workflow.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A workflow definition in the vault.
///
/// Workflows are multi-step sequences that reference installed capabilities
/// (MCPs and/or skills) to accomplish complex, composed tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowEntry {
    /// Unique identifier (ULID).
    pub id: String,

    /// Short machine-friendly name.
    pub name: String,

    /// Human-readable description.
    pub description: Option<String>,

    /// Ordered list of workflow steps.
    pub steps: Vec<WorkflowStep>,

    /// Names of other capabilities (MCPs, skills) this workflow depends on.
    /// The vault will validate these are installed before executing.
    pub dependencies: Vec<String>,

    /// Timestamp when this workflow was first installed.
    pub installed_at: DateTime<Utc>,
}

/// A single step in a workflow.
///
/// Each step references either an MCP server or a skill (or neither, for
/// inline steps), and carries step-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowStep {
    /// Human-readable name for this step.
    /// Example: "Run linter", "Commit changes"
    pub name: String,

    /// Optional MCP server ID this step invokes.
    pub mcp_id: Option<String>,

    /// Optional skill ID this step invokes.
    pub skill_id: Option<String>,

    /// Step-specific configuration key-value pairs.
    /// Interpretation depends on the referenced capability.
    pub config: HashMap<String, String>,

    /// Names of other steps this step depends on (must complete first).
    /// Used for topological ordering. If empty, step can run immediately.
    pub depends_on: Vec<String>,
}
```

### 3.4 Capability Models

```rust
// vault-core/src/models/capability.rs

use serde::{Deserialize, Serialize};

/// A high-level capability that bundles multiple MCPs, skills, and workflows
/// into a named, reusable unit.
///
/// Capabilities serve as the abstraction layer above individual MCP servers.
/// Example: A "code-review" capability might require the "github" MCP,
/// the "code-review-skill", and the "review-workflow".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityEntry {
    /// Unique identifier (ULID).
    pub id: String,

    /// Short machine-friendly name.
    pub name: String,

    /// Human-readable description.
    pub description: Option<String>,

    /// MCP server names required by this capability.
    pub required_mcps: Vec<String>,

    /// Skill names required by this capability.
    pub required_skills: Vec<String>,

    /// Workflow names required by this capability.
    pub required_workflows: Vec<String>,
}
```

### 3.5 Agent Models

```rust
// vault-core/src/models/agent.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a registered agent connector.
///
/// Tracks which AI agents the vault syncs to, where their config files
/// live, and when they were last synced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConnectorConfig {
    /// Unique identifier (ULID).
    pub id: String,

    /// The type of AI agent.
    pub agent_type: AgentType,

    /// Absolute path to the agent's configuration file.
    /// Example: /home/user/.claude/claude_desktop_config.json
    pub config_path: PathBuf,

    /// Whether this connector is enabled for sync operations.
    pub enabled: bool,

    /// Timestamp of the last successful sync, if any.
    pub last_synced: Option<DateTime<Utc>>,

    /// Whether to automatically sync when capabilities are installed/removed.
    pub auto_sync: bool,
}

/// Supported AI agent types.
///
/// Each variant maps to a specific connector implementation that knows
/// how to read/write that agent's configuration format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Anthropic's Claude Code CLI.
    /// Config: ~/.claude/claude_desktop_config.json
    ClaudeCode,

    /// Google's Gemini CLI.
    /// Config: ~/.gemini/config/settings.json
    GeminiCli,

    /// OpenCode CLI.
    /// Config: ~/.config/opencode/config.json
    OpenCode,

    /// OpenAI's Codex CLI.
    /// Config: ~/.codex/config.json
    CodexCli,

    /// Cursor IDE.
    /// Config: ~/.cursor/mcp.json
    Cursor,

    /// Any other agent not natively supported.
    /// The String is the user-provided agent type name.
    Custom(String),
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::ClaudeCode => write!(f, "claude"),
            AgentType::GeminiCli => write!(f, "gemini"),
            AgentType::OpenCode => write!(f, "opencode"),
            AgentType::CodexCli => write!(f, "codex"),
            AgentType::Cursor => write!(f, "cursor"),
            AgentType::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Ok(AgentType::ClaudeCode),
            "gemini" | "gemini-cli" | "geminicli" => Ok(AgentType::GeminiCli),
            "opencode" | "open-code" => Ok(AgentType::OpenCode),
            "codex" | "codex-cli" | "codexcli" => Ok(AgentType::CodexCli),
            "cursor" => Ok(AgentType::Cursor),
            other => Ok(AgentType::Custom(other.to_string())),
        }
    }
}
```

### 3.6 Configuration Models

```rust
// vault-core/src/models/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global AgentVault configuration, stored in ~/.agentvault/config.toml.
///
/// Controls vault-wide behavior. All fields have sensible defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Root directory for all vault data.
    /// Default: ~/.agentvault/
    #[serde(default = "default_vault_dir")]
    pub vault_dir: PathBuf,

    /// Default agent to sync to when no agent is specified.
    /// Example: "claude"
    pub default_agent: Option<String>,

    /// If true, automatically sync to all registered agents after install/remove/update.
    #[serde(default)]
    pub sync_on_install: bool,

    /// If true, create a backup of agent configs before every sync.
    #[serde(default = "default_true")]
    pub backup_before_sync: bool,

    /// Maximum number of backups to keep per agent.
    /// Oldest backups are pruned when this limit is exceeded.
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,

    /// Log level for file logging. One of: trace, debug, info, warn, error.
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// If true, mask sensitive environment variable values in CLI output.
    #[serde(default = "default_true")]
    pub mask_secrets: bool,

    /// Patterns for identifying secret env var names (case-insensitive substring match).
    /// Default: ["token", "key", "secret", "password", "credential"]
    #[serde(default = "default_secret_patterns")]
    pub secret_patterns: Vec<String>,
}

fn default_vault_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".agentvault")
}

fn default_true() -> bool {
    true
}

fn default_max_backups() -> usize {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_secret_patterns() -> Vec<String> {
    vec![
        "token".to_string(),
        "key".to_string(),
        "secret".to_string(),
        "password".to_string(),
        "credential".to_string(),
    ]
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_dir: default_vault_dir(),
            default_agent: None,
            sync_on_install: false,
            backup_before_sync: true,
            max_backups: 10,
            log_level: "info".to_string(),
            mask_secrets: true,
            secret_patterns: default_secret_patterns(),
        }
    }
}
```

### 3.7 Manifest Models

```rust
// vault-core/src/models/manifest.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The vault.toml manifest — a declarative specification of the desired
/// capability set. Used for `vault import` / `vault export` and for
/// sharing configurations across machines or teams.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultManifest {
    /// Manifest-level metadata.
    pub vault: ManifestMeta,

    /// MCP servers to install.
    #[serde(default)]
    pub mcps: Vec<ManifestMcp>,

    /// Skills to install.
    #[serde(default)]
    pub skills: Vec<ManifestSkill>,

    /// Workflows to install.
    #[serde(default)]
    pub workflows: Vec<ManifestWorkflow>,

    /// Agent connectors to register.
    #[serde(default)]
    pub agents: Vec<ManifestAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMeta {
    /// Manifest name (for display purposes).
    pub name: String,

    /// Manifest version (semver).
    pub version: String,

    /// Optional description.
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMcp {
    /// MCP name (used as the key in agent configs).
    pub name: String,

    /// Source type: "npm", "pypi", "github", "local", "docker".
    pub source: String,

    /// Package or repo identifier.
    /// For npm: "@anthropic/mcp-filesystem"
    /// For pypi: "mcp-server-memory"
    /// For github: "owner/repo"
    /// For local: "/path/to/mcp"
    /// For docker: "registry/image:tag"
    pub package: Option<String>,

    /// Version constraint (semver).
    /// Examples: "latest", "0.6.0", "^1.0", ">=0.5,<1.0"
    #[serde(default = "default_version")]
    pub version: String,

    /// Override the default command.
    pub command: Option<String>,

    /// Override the default args.
    pub args: Option<Vec<String>>,

    /// Environment variables for this MCP.
    /// Values support "env:VAR_NAME" syntax for delegation.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Transport override: "stdio", "sse", "http".
    pub transport: Option<String>,

    /// URL for SSE or HTTP transport.
    pub url: Option<String>,

    /// Which agents to sync this MCP to. Empty = all agents.
    #[serde(default)]
    pub agents: Vec<String>,

    /// Optional display name.
    pub display_name: Option<String>,

    /// Optional description.
    pub description: Option<String>,

    /// Optional tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "latest".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSkill {
    /// Skill name.
    pub name: String,

    /// Source type: "git", "local".
    pub source: String,

    /// Repository URL (for git source).
    pub repo: Option<String>,

    /// Git ref (branch, tag, SHA).
    #[serde(rename = "ref")]
    pub ref_: Option<String>,

    /// Local path (for local source).
    pub path: Option<String>,

    /// Subdirectory within git repo.
    pub subdirectory: Option<String>,

    /// Which agents to sync this skill to.
    #[serde(default)]
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestWorkflow {
    /// Workflow name.
    pub name: String,

    /// Source type: "local", "git".
    pub source: String,

    /// Path or repo.
    pub path: Option<String>,
    pub repo: Option<String>,

    #[serde(rename = "ref")]
    pub ref_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestAgent {
    /// Agent type name: "claude", "gemini", "opencode", "codex", "cursor".
    pub name: String,

    /// Override the default config path.
    pub config_path: Option<String>,

    /// Whether auto-sync is enabled.
    #[serde(default)]
    pub auto_sync: bool,
}
```

### 3.8 Sync Models

```rust
// vault-core/src/models/sync.rs (used by connectors)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Result of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Agent type that was synced.
    pub agent_type: String,

    /// Timestamp of the sync operation.
    pub timestamp: DateTime<Utc>,

    /// The diff that was applied.
    pub diff: SyncDiff,

    /// Whether the sync completed successfully.
    pub success: bool,

    /// Path to the backup file created before sync (if any).
    pub backup_path: Option<String>,

    /// Error message if sync failed.
    pub error: Option<String>,
}

/// Computed diff between vault state and agent config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncDiff {
    /// MCP entries to add to the agent config.
    pub additions: Vec<SyncEntry>,

    /// MCP entries to remove from the agent config.
    pub removals: Vec<SyncEntry>,

    /// MCP entries with changed fields.
    pub updates: Vec<SyncUpdate>,
}

impl SyncDiff {
    /// Returns true if no changes need to be applied.
    pub fn is_empty(&self) -> bool {
        self.additions.is_empty() && self.removals.is_empty() && self.updates.is_empty()
    }

    /// Total number of changes.
    pub fn change_count(&self) -> usize {
        self.additions.len() + self.removals.len() + self.updates.len()
    }
}

/// A single MCP entry in a sync diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    /// MCP name (the key in agent config).
    pub name: String,

    /// Source description (for display).
    pub source: String,

    /// Version (for display).
    pub version: String,
}

/// A changed MCP entry in a sync diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncUpdate {
    /// MCP name.
    pub name: String,

    /// Fields that changed.
    pub changed_fields: Vec<FieldChange>,
}

/// A single field change in an MCP entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Field name (e.g., "version", "args", "env.GITHUB_TOKEN").
    pub field: String,

    /// Previous value (as string).
    pub old_value: String,

    /// New value (as string).
    pub new_value: String,
}
```

---

## 4. SQLite Schema

The registry database (`~/.agentvault/registry.db`) stores all vault state. It is the single source of truth; filesystem artifacts (MCP directories, skill directories) are the installed *files*, but the registry is the *index*.

### 4.1 Full Schema

```sql
-- ============================================================
-- AgentVault SQLite Schema
-- Version: 1
-- ============================================================

-- Enable WAL mode for concurrent reads during sync operations
PRAGMA journal_mode = WAL;

-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- ============================================================
-- MCPs Table
-- Stores all installed MCP server metadata.
-- ============================================================
CREATE TABLE IF NOT EXISTS mcps (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID
    name            TEXT    NOT NULL UNIQUE,          -- machine-friendly name
    display_name    TEXT,                             -- optional human-friendly name
    version         TEXT    NOT NULL,                 -- semver or commit SHA
    source_type     TEXT    NOT NULL,                 -- 'npm', 'pypi', 'github', 'local', 'docker'
    source_value    TEXT    NOT NULL,                 -- package name, repo URL, path, or image
    source_ref      TEXT,                             -- git ref (branch/tag/sha), null for npm/pypi
    install_path    TEXT    NOT NULL,                 -- absolute path to install directory
    command         TEXT    NOT NULL,                 -- executable command
    args_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of command arguments
    env_json        TEXT    NOT NULL DEFAULT '{}',    -- JSON object of env vars
    transport       TEXT    NOT NULL DEFAULT 'stdio', -- 'stdio', 'sse', 'http'
    transport_url   TEXT,                             -- URL for sse/http transports
    status          TEXT    NOT NULL DEFAULT 'active', -- 'active', 'disabled', 'error'
    status_error    TEXT,                             -- error message when status='error'
    checksum        TEXT,                             -- 'sha256:<hex>' integrity hash
    agents_json     TEXT    NOT NULL DEFAULT '[]',    -- JSON array of agent names to sync to
    tags_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of tags
    description     TEXT,                             -- optional description
    installed_at    TEXT    NOT NULL,                 -- ISO 8601 timestamp
    updated_at      TEXT    NOT NULL                  -- ISO 8601 timestamp
);

CREATE INDEX idx_mcps_name ON mcps(name);
CREATE INDEX idx_mcps_source_type ON mcps(source_type);
CREATE INDEX idx_mcps_status ON mcps(status);

-- ============================================================
-- Skills Table
-- Stores installed skill metadata.
-- ============================================================
CREATE TABLE IF NOT EXISTS skills (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID
    name            TEXT    NOT NULL UNIQUE,          -- machine-friendly name
    description     TEXT,                             -- human-readable description
    path            TEXT    NOT NULL,                 -- absolute path to skill directory
    tags_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of tags
    source_type     TEXT    NOT NULL,                 -- 'git', 'local'
    source_value    TEXT    NOT NULL,                 -- repo URL or local path
    source_ref      TEXT,                             -- git ref
    source_subdir   TEXT,                             -- subdirectory within repo
    agents_json     TEXT    NOT NULL DEFAULT '[]',    -- JSON array of agent names
    installed_at    TEXT    NOT NULL                  -- ISO 8601 timestamp
);

CREATE INDEX idx_skills_name ON skills(name);

-- ============================================================
-- Workflows Table
-- Stores workflow definitions.
-- ============================================================
CREATE TABLE IF NOT EXISTS workflows (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID
    name            TEXT    NOT NULL UNIQUE,          -- machine-friendly name
    description     TEXT,                             -- human-readable description
    steps_json      TEXT    NOT NULL DEFAULT '[]',    -- JSON array of WorkflowStep objects
    deps_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of dependency names
    installed_at    TEXT    NOT NULL                  -- ISO 8601 timestamp
);

CREATE INDEX idx_workflows_name ON workflows(name);

-- ============================================================
-- Capabilities Table
-- Higher-level bundles of MCPs + skills + workflows.
-- ============================================================
CREATE TABLE IF NOT EXISTS capabilities (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID
    name            TEXT    NOT NULL UNIQUE,          -- capability name
    description     TEXT,                             -- human-readable description
    mcps_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of required MCP names
    skills_json     TEXT    NOT NULL DEFAULT '[]',    -- JSON array of required skill names
    workflows_json  TEXT    NOT NULL DEFAULT '[]'     -- JSON array of required workflow names
);

CREATE INDEX idx_capabilities_name ON capabilities(name);

-- ============================================================
-- Agent Configs Table
-- Registered agent connectors and their sync state.
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_configs (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID
    agent_type      TEXT    NOT NULL UNIQUE,          -- 'claude', 'gemini', 'opencode', etc.
    config_path     TEXT    NOT NULL,                 -- absolute path to agent's config file
    enabled         INTEGER NOT NULL DEFAULT 1,      -- 1=enabled, 0=disabled
    last_synced     TEXT,                             -- ISO 8601 timestamp or NULL
    auto_sync       INTEGER NOT NULL DEFAULT 0       -- 1=auto-sync on install, 0=manual
);

CREATE INDEX idx_agent_configs_type ON agent_configs(agent_type);

-- ============================================================
-- Sync History Table
-- Audit log of all sync operations.
-- ============================================================
CREATE TABLE IF NOT EXISTS sync_history (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID
    agent_type      TEXT    NOT NULL,                 -- which agent was synced
    timestamp       TEXT    NOT NULL,                 -- ISO 8601 timestamp
    action          TEXT    NOT NULL,                 -- 'sync', 'dry_run', 'backup', 'restore'
    changes_json    TEXT    NOT NULL DEFAULT '{}',    -- JSON SyncDiff object
    backup_path     TEXT,                             -- path to backup file (if created)
    status          TEXT    NOT NULL DEFAULT 'success', -- 'success', 'failure', 'partial'
    error           TEXT,                             -- error message on failure

    FOREIGN KEY (agent_type) REFERENCES agent_configs(agent_type)
        ON DELETE CASCADE
);

CREATE INDEX idx_sync_history_agent ON sync_history(agent_type);
CREATE INDEX idx_sync_history_timestamp ON sync_history(timestamp);

-- ============================================================
-- Migrations Table
-- Tracks applied schema migrations for forward-compatibility.
-- ============================================================
CREATE TABLE IF NOT EXISTS migrations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    version         INTEGER NOT NULL UNIQUE,          -- monotonically increasing schema version
    description     TEXT,                             -- human-readable migration description
    applied_at      TEXT    NOT NULL                  -- ISO 8601 timestamp
);

-- Record the initial schema version
INSERT INTO migrations (version, description, applied_at)
VALUES (1, 'Initial schema', datetime('now'));
```

### 4.2 Migration Strategy

Migrations are applied sequentially on database open:

```rust
// vault-core/src/registry/migrations.rs

/// Each migration is a (version, description, SQL) tuple.
const MIGRATIONS: &[(i64, &str, &str)] = &[
    (1, "Initial schema", include_str!("../../sql/001_initial.sql")),
    // Future migrations added here:
    // (2, "Add mcp categories", include_str!("../../sql/002_mcp_categories.sql")),
];

/// Apply all pending migrations.
pub fn run_migrations(conn: &rusqlite::Connection) -> Result<(), VaultError> {
    // 1. Ensure migrations table exists (bootstrap)
    // 2. Query max applied version
    // 3. Apply each migration with version > max
    // 4. Wrap each migration in a transaction
    // 5. Insert into migrations table on success
}
```

---

## 5. CLI Interface

The CLI is built with `clap` derive macros. Every command, subcommand, argument, flag, and option is defined here.

### 5.1 Top-Level CLI Definition

```rust
// vault-cli/src/cli.rs

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
        vault doctor                            # Check vault health",
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
}
```

### 5.2 Command Argument Definitions

```rust
// ─── vault init ──────────────────────────────────────────────────

/// Initialize a new AgentVault workspace.
///
/// Creates the ~/.agentvault/ directory structure, initializes the SQLite
/// registry, and generates a default config.toml.
///
/// Safe to run multiple times — will not overwrite existing data.
#[derive(Parser, Debug)]
#[command(
    after_help = "Examples:\n  \
        vault init                    # Initialize with defaults\n  \
        vault init --dir ~/my-vault   # Custom vault directory"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault install npm:@anthropic/mcp-filesystem\n  \
        vault install pypi:mcp-server-memory\n  \
        vault install github:anthropics/mcp-servers\n  \
        vault install local:/home/user/my-mcp\n  \
        vault install npm:@anthropic/mcp-github --env GITHUB_TOKEN=env:GITHUB_TOKEN\n  \
        vault install npm:@anthropic/mcp-filesystem --name fs --args '/home/user/projects'"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault remove filesystem\n  \
        vault remove github --force\n  \
        vault remove my-mcp --keep-files"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault update filesystem           # Update single MCP\n  \
        vault update --all                # Update all capabilities\n  \
        vault update --all --dry-run      # Preview what would change"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault list                    # List all\n  \
        vault list --mcps             # MCPs only\n  \
        vault list --skills --json    # Skills as JSON\n  \
        vault list --table            # Force table output"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault search filesystem\n  \
        vault search github --source npm\n  \
        vault search memory --limit 5"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault sync claude             # Sync to Claude Code\n  \
        vault sync --all              # Sync to all agents\n  \
        vault sync claude --dry-run   # Preview changes\n  \
        vault sync --all --force      # Overwrite all (no merge)"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault config                          # Print all config\n  \
        vault config default_agent            # Get a value\n  \
        vault config default_agent claude     # Set a value\n  \
        vault config --list                   # Print as key=value pairs\n  \
        vault config --reset                  # Reset to defaults"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault doctor         # Run all checks\n  \
        vault doctor --fix   # Attempt to auto-fix issues"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault connector add claude\n  \
        vault connector add gemini --config-path ~/.gemini/settings.json\n  \
        vault connector add custom --config-path /path/to/config.json"
)]
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
#[command(
    after_help = "Examples:\n  \
        vault export                       # Export to ./vault.toml\n  \
        vault export --output my-vault.toml\n  \
        vault export --format json --output vault.json"
)]
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
#[derive(Parser, Debug)]
#[command(
    after_help = "Examples:\n  \
        vault import vault.toml\n  \
        vault import vault.toml --dry-run\n  \
        vault import vault.toml --replace\n  \
        vault import vault.json --merge"
)]
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
```

### 5.3 Command Summary Table

| Command | Description | Key Flags |
|---|---|---|
| `vault init` | Create `~/.agentvault/` and initialize registry | `--dir`, `--force` |
| `vault install <source>` | Install MCP/skill/workflow | `--source-type`, `--version`, `--name`, `--env`, `--args`, `--transport`, `--agent`, `--tag` |
| `vault remove <name>` | Remove installed capability | `--force`, `--keep-files` |
| `vault update [name]` | Update capability versions | `--all`, `--dry-run`, `--force` |
| `vault list` | List installed capabilities | `--mcps`, `--skills`, `--workflows`, `--all`, `--json`, `--table`, `--detail` |
| `vault search <query>` | Search local/remote registries | `--source`, `--limit`, `--json` |
| `vault sync [agent]` | Sync vault → agent config | `--all`, `--dry-run`, `--force`, `--backup`, `--prune` |
| `vault status` | Show vault health | `--json` |
| `vault config [key] [value]` | Get/set configuration | `--list`, `--reset` |
| `vault doctor` | Diagnose environment | `--fix`, `--check-mcps` |
| `vault connector add <type>` | Register agent connector | `--config-path`, `--auto-sync` |
| `vault connector list` | List connectors | `--json` |
| `vault connector remove <type>` | Unregister connector | `--force` |
| `vault export` | Export vault state | `--format`, `--output` |
| `vault import <file>` | Import from manifest | `--dry-run`, `--merge`, `--replace` |
| `vault completions <shell>` | Generate shell completions | shell: `bash`, `zsh`, `fish`, `powershell` |

---

## 6. Agent Connector Specifications

Each agent connector must know:
1. Where the agent's config file lives (platform-specific paths)
2. The exact JSON/TOML structure the agent expects
3. How to map `McpEntry` fields to that structure
4. How to preserve non-vault entries during sync

### 6.1 Claude Code

**Config file locations:**

| Platform | Path |
|---|---|
| Linux | `~/.claude/claude_desktop_config.json` |
| macOS | `~/.claude/claude_desktop_config.json` |
| Windows | `%USERPROFILE%\.claude\claude_desktop_config.json` |

**Config structure:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"],
      "env": {}
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-github"],
      "env": {
        "GITHUB_TOKEN": "<token>"
      }
    },
    "memory": {
      "command": "python",
      "args": ["-m", "mcp_server_memory"],
      "env": {}
    }
  }
}
```

**Field mapping from `McpEntry`:**

| McpEntry field | Claude config field |
|---|---|
| `name` | Key in `mcpServers` object |
| `command` | `mcpServers.<name>.command` |
| `args` | `mcpServers.<name>.args` |
| `env_vars` | `mcpServers.<name>.env` |

**Notes:**
- Claude Code only supports `stdio` transport. SSE/HTTP MCPs cannot be synced to Claude.
- The config file may contain other top-level keys (e.g., `permissions`); these must be preserved.
- If `env_vars` contains `"env:VAR_NAME"` references, resolve them to the actual env var value at sync time, or pass the raw reference if the agent supports it.

**Connector behavior:**
- `read_config()`: Parse JSON, extract `mcpServers` object.
- `write_config()`: Merge vault entries into `mcpServers`, preserve other keys, write atomically.
- `diff()`: Compare vault entries against existing `mcpServers` entries by name.

### 6.2 Gemini CLI

**Config file locations:**

| Platform | Path |
|---|---|
| Linux | `~/.gemini/config/settings.json` |
| macOS | `~/.gemini/config/settings.json` |
| Windows | `%USERPROFILE%\.gemini\config\settings.json` |

**Config structure:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"],
      "env": {}
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-github"],
      "env": {
        "GITHUB_TOKEN": "<token>"
      }
    }
  }
}
```

**Field mapping from `McpEntry`:**

| McpEntry field | Gemini config field |
|---|---|
| `name` | Key in `mcpServers` object |
| `command` | `mcpServers.<name>.command` |
| `args` | `mcpServers.<name>.args` |
| `env_vars` | `mcpServers.<name>.env` |

**Notes:**
- Gemini CLI config structure is nearly identical to Claude Code's.
- Gemini may support additional fields like `timeout` or `cwd` — preserve them if present.
- The settings file may contain many other configuration keys — preserve all of them.

### 6.3 OpenCode

**Config file locations:**

| Platform | Path |
|---|---|
| Linux | `~/.config/opencode/config.json` or `$XDG_CONFIG_HOME/opencode/config.json` |
| macOS | `~/.config/opencode/config.json` |
| Windows | `%APPDATA%\opencode\config.json` |

**Config structure:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"],
      "env": {}
    }
  }
}
```

**Field mapping:** Same as Claude Code and Gemini CLI (the `mcpServers` format is becoming a de facto standard).

**Notes:**
- OpenCode also supports per-project `.opencode.json` files — the connector only manages the global config.
- Respects `$XDG_CONFIG_HOME` on Linux.

### 6.4 Codex CLI

**Config file locations:**

| Platform | Path |
|---|---|
| Linux | `~/.codex/config.json` |
| macOS | `~/.codex/config.json` |
| Windows | `%USERPROFILE%\.codex\config.json` |

**Config structure:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"],
      "env": {}
    }
  }
}
```

**Notes:**
- Codex CLI's MCP config format follows the same `mcpServers` convention.
- If the config file doesn't exist, the connector should create it with just the `mcpServers` key.

### 6.5 Cursor (Future)

**Config file locations:**

| Platform | Path |
|---|---|
| Linux | `~/.cursor/mcp.json` |
| macOS | `~/.cursor/mcp.json` |
| Windows | `%USERPROFILE%\.cursor\mcp.json` |

**Config structure:**

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"],
      "env": {}
    }
  }
}
```

### 6.6 Connector Registration

```rust
// vault-connectors/src/registry.rs

use std::collections::HashMap;
use crate::traits::AgentConnector;
use vault_core::models::agent::AgentType;

/// Registry of all available agent connectors.
///
/// Built-in connectors are registered at startup.
/// Custom connectors can be added at runtime.
pub struct ConnectorRegistry {
    connectors: HashMap<AgentType, Box<dyn AgentConnector>>,
}

impl ConnectorRegistry {
    /// Create a new registry with all built-in connectors.
    pub fn new() -> Self {
        let mut registry = Self {
            connectors: HashMap::new(),
        };

        // Register built-in connectors
        registry.register(Box::new(crate::claude::ClaudeConnector::new()));
        registry.register(Box::new(crate::gemini::GeminiConnector::new()));
        registry.register(Box::new(crate::opencode::OpenCodeConnector::new()));
        registry.register(Box::new(crate::codex::CodexConnector::new()));

        registry
    }

    /// Register a connector.
    pub fn register(&mut self, connector: Box<dyn AgentConnector>) {
        self.connectors.insert(connector.agent_type(), connector);
    }

    /// Get a connector by agent type.
    pub fn get(&self, agent_type: &AgentType) -> Option<&dyn AgentConnector> {
        self.connectors.get(agent_type).map(|c| c.as_ref())
    }

    /// List all registered connectors.
    pub fn list(&self) -> Vec<&dyn AgentConnector> {
        self.connectors.values().map(|c| c.as_ref()).collect()
    }
}
```

---

## 7. Sync Algorithm

The sync algorithm is the core value proposition of AgentVault. It must be:
- **Non-destructive**: Never loses user data.
- **Idempotent**: Running twice produces the same result.
- **Auditable**: Every operation is logged with full diffs.

### 7.1 Algorithm Steps

```
FUNCTION sync(agent_type: AgentType, options: SyncOptions) -> Result<SyncResult>:

    // ── Step 1: Resolve connector ────────────────────────────────
    connector = registry.get(agent_type)?
    IF connector is None:
        RETURN Err(VaultError::ConnectorNotFound(agent_type))

    // ── Step 2: Read vault state ─────────────────────────────────
    // Query all MCPs assigned to this agent (or all if agents list is empty)
    vault_mcps = db.list_mcps()?
    target_mcps = vault_mcps.filter(|mcp|
        mcp.status == Active
        AND (mcp.agents.is_empty() OR mcp.agents.contains(agent_type.name()))
    )

    // ── Step 3: Read agent's current config ──────────────────────
    agent_config = connector.read_config()?
    // Returns a map: name -> { command, args, env }

    // ── Step 4: Identify vault-managed entries ───────────────────
    // Vault-managed entries are tracked by a comment/marker or by
    // cross-referencing with the sync_history table.
    // Strategy: entries whose names match vault MCPs are vault-managed.
    vault_managed_names = sync_history.last_sync_names(agent_type)?

    // ── Step 5: Compute diff ─────────────────────────────────────
    diff = SyncDiff::default()

    FOR mcp IN target_mcps:
        IF mcp.name NOT IN agent_config:
            diff.additions.push(mcp)              // New entry
        ELSE:
            existing = agent_config[mcp.name]
            IF mcp.command != existing.command
               OR mcp.args != existing.args
               OR mcp.env_vars != existing.env:
                diff.updates.push(SyncUpdate {     // Changed entry
                    name: mcp.name,
                    changed_fields: compute_field_changes(existing, mcp)
                })

    IF options.prune:
        FOR name IN vault_managed_names:
            IF name NOT IN target_mcps.names():
                diff.removals.push(name)           // Removed from vault

    // ── Step 6: Dry-run check ────────────────────────────────────
    IF options.dry_run:
        display_diff(diff)
        log_sync(agent_type, "dry_run", diff, success=true)
        RETURN Ok(SyncResult { diff, dry_run: true, ... })

    // ── Step 7: Bail if no changes ───────────────────────────────
    IF diff.is_empty():
        print("✓ Already in sync. No changes needed.")
        RETURN Ok(SyncResult { diff, ... })

    // ── Step 8: Create backup ────────────────────────────────────
    IF options.backup OR config.backup_before_sync:
        backup_path = connector.backup()?
        // Copies agent config to:
        //   ~/.agentvault/backups/<agent>/<ISO-8601-timestamp>.json
        prune_old_backups(agent_type, config.max_backups)

    // ── Step 9: Apply additions ──────────────────────────────────
    FOR mcp IN diff.additions:
        agent_config.mcpServers[mcp.name] = {
            command: mcp.command,
            args: resolve_args(mcp.args),
            env: resolve_env_vars(mcp.env_vars)
        }

    // ── Step 10: Apply removals ──────────────────────────────────
    FOR name IN diff.removals:
        agent_config.mcpServers.remove(name)

    // ── Step 11: Apply updates ───────────────────────────────────
    FOR update IN diff.updates:
        mcp = target_mcps.find(update.name)
        agent_config.mcpServers[mcp.name] = {
            command: mcp.command,
            args: resolve_args(mcp.args),
            env: resolve_env_vars(mcp.env_vars)
        }

    // ── Step 12: Write config atomically ─────────────────────────
    // Write to a temp file first, then rename to prevent corruption.
    temp_path = connector.config_path().with_extension("vault-tmp")
    write_json(temp_path, agent_config)?
    rename(temp_path, connector.config_path())?

    // ── Step 13: Verify written config ───────────────────────────
    // Re-read and parse to ensure the file is valid.
    verification = connector.verify()?
    IF NOT verification:
        // Restore backup
        restore_backup(backup_path, connector.config_path())
        RETURN Err(VaultError::SyncVerificationFailed)

    // ── Step 14: Log sync result ─────────────────────────────────
    log_sync(agent_type, "sync", diff, success=true)
    update_agent_last_synced(agent_type, now())

    RETURN Ok(SyncResult {
        agent_type,
        timestamp: now(),
        diff,
        success: true,
        backup_path: Some(backup_path),
        error: None,
    })
```

### 7.2 Environment Variable Resolution

```
FUNCTION resolve_env_vars(env_vars: HashMap<String, String>) -> HashMap<String, String>:
    resolved = HashMap::new()

    FOR (key, value) IN env_vars:
        IF value.starts_with("env:"):
            // Delegate to host environment
            env_name = value.strip_prefix("env:")
            resolved_value = std::env::var(env_name)
                .unwrap_or_else(|_| value.clone())  // Keep reference if not set
            resolved[key] = resolved_value
        ELSE:
            resolved[key] = value

    RETURN resolved
```

### 7.3 Sync for `--all`

```
FUNCTION sync_all(options: SyncOptions) -> Vec<SyncResult>:
    agents = db.list_agent_configs()?.filter(|a| a.enabled)
    results = Vec::new()

    FOR agent IN agents:
        result = sync(agent.agent_type, options.clone())
        results.push(result)

    print_summary_table(results)
    RETURN results
```

---

## 8. vault.toml Manifest Format

The manifest is the declarative specification of a vault's desired state. It is used for:
1. **Sharing**: Teams can commit `vault.toml` to a repo so all members have the same capability set.
2. **Backup**: `vault export` generates a manifest from current state.
3. **Reproducibility**: `vault import vault.toml` on a fresh machine reproduces the full setup.

### 8.1 Full Specification

```toml
# ─── Vault metadata ──────────────────────────────────────────────
[vault]
name = "my-workspace"                   # Manifest name (display only)
version = "1.0.0"                       # Manifest version (for tracking changes)
description = "My AI agent capability set for full-stack development"

# ─── MCP Servers ──────────────────────────────────────────────────
# Each [[mcps]] entry declares an MCP server to install.

[[mcps]]
name = "filesystem"                     # Key in agent configs
source = "npm"                          # Source type: npm, pypi, github, local, docker
package = "@anthropic/mcp-filesystem"   # Package identifier
version = "latest"                      # Version constraint (semver or "latest")
display_name = "Filesystem Access"      # Optional display name
description = "Read/write access to the local filesystem"
command = "npx"                         # Override auto-detected command
args = ["-y", "@anthropic/mcp-filesystem", "/home/user/projects"]
env = { }                              # Environment variables
transport = "stdio"                     # Transport: stdio, sse, http
agents = ["claude", "gemini"]           # Sync to these agents only
tags = ["filesystem", "io"]             # Tags for search/categorization

[[mcps]]
name = "github"
source = "npm"
package = "@anthropic/mcp-github"
version = "0.6.0"                       # Pin to specific version
display_name = "GitHub Integration"
env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }  # env: prefix delegates to host
agents = ["claude", "gemini", "opencode"]
tags = ["github", "vcs", "code-review"]

[[mcps]]
name = "memory"
source = "pypi"
package = "mcp-server-memory"
version = ">=0.2.0"                     # Semver range constraint
agents = ["claude"]

[[mcps]]
name = "postgres"
source = "npm"
package = "@anthropic/mcp-postgres"
version = "latest"
env = { POSTGRES_URL = "env:DATABASE_URL" }
agents = ["claude", "gemini"]

[[mcps]]
name = "custom-analyzer"
source = "github"
package = "myorg/mcp-analyzer"          # GitHub owner/repo
version = "main"                        # Git ref (branch, tag, SHA)
tags = ["analysis", "custom"]

[[mcps]]
name = "local-dev-tools"
source = "local"
package = "/home/user/dev/my-mcp-tools" # Local path
transport = "stdio"
agents = ["claude"]

[[mcps]]
name = "remote-search"
source = "npm"
package = "mcp-search-server"
transport = "sse"                       # SSE transport
url = "http://localhost:3001/sse"        # Required for sse/http transport

# ─── Skills ───────────────────────────────────────────────────────

[[skills]]
name = "git-workflow"
source = "git"
repo = "https://github.com/user/agent-skills"
ref = "main"
subdirectory = "skills/git-workflow"     # Skill within a monorepo
agents = ["claude", "gemini"]

[[skills]]
name = "code-review"
source = "git"
repo = "https://github.com/user/agent-skills"
ref = "v1.0.0"
subdirectory = "skills/code-review"

[[skills]]
name = "local-patterns"
source = "local"
path = "/home/user/dev/my-skills/patterns"

# ─── Workflows ────────────────────────────────────────────────────

[[workflows]]
name = "full-review"
source = "local"
path = "/home/user/dev/workflows/full-review"

# ─── Agent Connectors ────────────────────────────────────────────

[[agents]]
name = "claude"
auto_sync = true                        # Auto-sync on install/remove

[[agents]]
name = "gemini"
auto_sync = true

[[agents]]
name = "opencode"
config_path = "/home/user/.config/opencode/config.json"  # Override default path
auto_sync = false
```

### 8.2 Manifest Validation Rules

| Rule | Error if violated |
|---|---|
| `vault.name` is required and non-empty | `ManifestError: vault.name is required` |
| `vault.version` is valid semver | `ManifestError: vault.version must be valid semver` |
| Each `mcps[].name` is unique | `ManifestError: duplicate MCP name: "<name>"` |
| Each `mcps[].source` is one of: npm, pypi, github, local, docker | `ManifestError: unknown source type: "<source>"` |
| If `source = "npm"` or `"pypi"`, `package` is required | `ManifestError: package is required for npm/pypi source` |
| If `source = "github"`, `package` (repo) is required | `ManifestError: package (repo) is required for github source` |
| If `transport` is "sse" or "http", `url` is required | `ManifestError: url is required for sse/http transport` |
| `version` is valid semver constraint or "latest" | `ManifestError: invalid version constraint: "<version>"` |
| Each `skills[].name` is unique | `ManifestError: duplicate skill name: "<name>"` |
| Each `agents[].name` is unique | `ManifestError: duplicate agent name: "<name>"` |

---

## 9. Error Handling Strategy

All errors flow through the `VaultError` enum, defined with `thiserror` for ergonomic error chaining. Each variant carries enough context for the CLI to display a user-friendly message with a recovery suggestion.

### 9.1 VaultError Definition

```rust
// vault-core/src/error.rs

use std::path::PathBuf;
use thiserror::Error;

/// Central error type for all AgentVault operations.
///
/// Every variant includes:
/// 1. A machine-readable enum discriminant
/// 2. A human-readable message (via thiserror #[error(...)])
/// 3. Sufficient context for the CLI to suggest recovery actions
#[derive(Error, Debug)]
pub enum VaultError {
    // ── Filesystem Errors ────────────────────────────────────────

    /// General I/O error with path context.
    #[error("I/O error at '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// File or directory not found.
    #[error("File not found: '{}'", path.display())]
    FileNotFound { path: PathBuf },

    /// Permission denied on a file or directory.
    #[error("Permission denied: '{}'. Check file permissions.", path.display())]
    PermissionDenied { path: PathBuf },

    // ── Database Errors ──────────────────────────────────────────

    /// SQLite operation failure.
    #[error("Database error: {message}")]
    Database {
        message: String,
        #[source]
        source: Option<rusqlite::Error>,
    },

    /// Database migration failure.
    #[error("Migration failed (version {version}): {message}")]
    Migration { version: i64, message: String },

    // ── Configuration Errors ─────────────────────────────────────

    /// Configuration file parsing error.
    #[error("Config error in '{}': {message}", path.display())]
    Config { path: PathBuf, message: String },

    /// Invalid configuration value.
    #[error("Invalid config value for '{key}': {message}")]
    ConfigValue { key: String, message: String },

    /// Manifest validation error.
    #[error("Manifest error: {message}")]
    Manifest { message: String },

    // ── Network Errors ───────────────────────────────────────────

    /// HTTP request failure.
    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<reqwest::Error>,
    },

    /// Download failure with URL context.
    #[error("Failed to download '{url}': {message}")]
    Download { url: String, message: String },

    // ── Connector Errors ─────────────────────────────────────────

    /// Agent connector read/write failure.
    #[error("Connector error ({agent}): {message}")]
    Connector { agent: String, message: String },

    /// Agent connector not found.
    #[error("No connector registered for agent '{agent}'. Run: vault connector add {agent}")]
    ConnectorNotFound { agent: String },

    /// Sync verification failed (written config is invalid).
    #[error("Sync verification failed for '{agent}'. Backup restored from '{backup_path}'.")]
    SyncVerificationFailed {
        agent: String,
        backup_path: String,
    },

    // ── MCP Errors ───────────────────────────────────────────────

    /// MCP installation failure.
    #[error("Failed to install MCP '{name}': {message}")]
    McpInstall { name: String, message: String },

    /// MCP server not responding (health check failure).
    #[error("MCP server '{name}' is not responding: {message}")]
    McpUnhealthy { name: String, message: String },

    // ── Registry Errors ──────────────────────────────────────────

    /// Requested capability not found in the registry.
    #[error("Capability '{name}' not found. Run: vault search {name}")]
    NotFound { name: String },

    /// Attempting to install a capability that already exists.
    #[error("Capability '{name}' is already installed (version {version}). Use: vault update {name}")]
    AlreadyExists { name: String, version: String },

    /// Version constraint violation.
    #[error("Version conflict for '{name}': installed {installed}, required {required}. Use --force to override.")]
    VersionConflict {
        name: String,
        installed: String,
        required: String,
    },

    // ── Serialization Errors ─────────────────────────────────────

    /// JSON serialization/deserialization error.
    #[error("JSON error: {message}")]
    Json {
        message: String,
        #[source]
        source: Option<serde_json::Error>,
    },

    /// TOML serialization/deserialization error.
    #[error("TOML error: {message}")]
    Toml {
        message: String,
        #[source]
        source: Option<toml::de::Error>,
    },

    // ── Tool Errors ──────────────────────────────────────────────

    /// Required external tool not found on PATH.
    #[error("Required tool '{tool}' not found. Install it and ensure it's on your PATH.")]
    ToolNotFound { tool: String },

    /// External tool execution failure.
    #[error("Tool '{tool}' failed (exit code {exit_code}): {stderr}")]
    ToolFailed {
        tool: String,
        exit_code: i32,
        stderr: String,
    },

    // ── Vault State Errors ───────────────────────────────────────

    /// Vault is not initialized.
    #[error("Vault not initialized. Run: vault init")]
    NotInitialized,

    /// Vault is already initialized.
    #[error("Vault already initialized at '{}'. Use --force to re-initialize.", path.display())]
    AlreadyInitialized { path: PathBuf },

    // ── Generic ──────────────────────────────────────────────────

    /// Catch-all for errors that don't fit other variants.
    #[error("{0}")]
    Other(String),
}
```

### 9.2 Error Display Strategy

Each error variant maps to a user-facing message with:
1. **What happened** — the error message (from `#[error(...)]`)
2. **What to do** — a recovery suggestion embedded in the message or appended by the CLI handler

```rust
// In the CLI handler (vault-cli/src/main.rs):

fn handle_error(err: VaultError) {
    match &err {
        VaultError::NotInitialized => {
            eprintln!("❌ {}", err);
            eprintln!("   💡 Run `vault init` to create a new vault.");
        }
        VaultError::NotFound { name } => {
            eprintln!("❌ {}", err);
            eprintln!("   💡 Try `vault search {}` to find similar capabilities.", name);
        }
        VaultError::ToolNotFound { tool } => {
            eprintln!("❌ {}", err);
            eprintln!("   💡 Install {} and try again.", tool);
            if tool == "npm" || tool == "npx" {
                eprintln!("      → https://nodejs.org/");
            } else if tool == "uv" || tool == "pip" {
                eprintln!("      → https://docs.astral.sh/uv/");
            }
        }
        VaultError::ConnectorNotFound { agent } => {
            eprintln!("❌ {}", err);
        }
        _ => {
            eprintln!("❌ {}", err);
        }
    }

    // Always show the full error chain in verbose mode
    if std::env::var("VAULT_LOG").is_ok() || is_verbose() {
        if let Some(source) = std::error::Error::source(&err) {
            eprintln!("\n   Caused by:");
            let mut source = Some(source);
            while let Some(s) = source {
                eprintln!("     → {}", s);
                source = std::error::Error::source(s);
            }
        }
    }
}
```

---

## 10. Security Model

### 10.1 Environment Variable Handling

Environment variables often contain secrets (API keys, tokens, credentials). AgentVault handles them with layered protection:

| Layer | Mechanism |
|---|---|
| **At rest (in registry.db)** | Env var values are stored as-is in the SQLite database. The database file has `0600` permissions (owner read/write only). Future: AES-256 encryption of values matching secret patterns. |
| **At rest (in manifest)** | The `vault.toml` file uses `env:VAR_NAME` references by default, never literal secrets. Literal values are allowed but flagged with a warning. |
| **In CLI output** | Values whose keys match `secret_patterns` (config) are masked as `****` in `vault list`, `vault status`, and `vault sync --dry-run`. |
| **During sync** | `env:VAR_NAME` references are resolved at sync time by reading from the host environment. If the host env var is not set, the reference is written literally so it can be resolved later. |

**`env:` prefix protocol:**

```
# In vault.toml or vault install --env:
GITHUB_TOKEN = "env:GITHUB_TOKEN"

# Resolution at sync time:
#   1. Read $GITHUB_TOKEN from host environment
#   2. If set, write the literal value into the agent config
#   3. If not set, write "env:GITHUB_TOKEN" literally (agent may resolve it)
```

### 10.2 Trust Levels

MCPs are assigned trust levels based on their source:

| Trust Level | Source | Badge | Behavior |
|---|---|---|---|
| **Verified** | Official Anthropic/Google packages | `✓` | Auto-approved for install |
| **Community** | Published npm/PyPI packages | `○` | Install with confirmation |
| **Local** | Local filesystem paths | `~` | Auto-approved (user's own code) |
| **Unknown** | Arbitrary GitHub repos, Docker images | `?` | Install with explicit `--yes` or confirmation prompt |

Trust level is stored in the MCP's metadata and displayed in `vault list`.

### 10.3 File Permissions

| Path | Permission | Rationale |
|---|---|---|
| `~/.agentvault/` | `0755` | Directory traversal |
| `~/.agentvault/config.toml` | `0600` | May contain sensitive defaults |
| `~/.agentvault/registry.db` | `0600` | Contains env var values |
| `~/.agentvault/vault.toml` | `0644` | Intended for sharing/version control |
| `~/.agentvault/mcps/` | `0755` | MCP binaries need execute |
| `~/.agentvault/backups/` | `0700` | Backups may contain secrets from agent configs |
| `~/.agentvault/logs/` | `0755` | Logs should be readable for debugging |
| `~/.agentvault/logs/*.log` | `0644` | Log files |

Permissions are set during `vault init` and verified during `vault doctor`.

### 10.4 Backup Policy

- **When**: Before every sync operation (unless `--backup false` or `backup_before_sync = false`).
- **Where**: `~/.agentvault/backups/<agent>/<ISO-8601-timestamp>.<ext>`
- **Pruning**: Keep the most recent `max_backups` (default: 10) per agent. Oldest are deleted.
- **Integrity**: Backups are byte-exact copies of the agent config file at the moment before writing.
- **Restore**: Manual restore by copying the backup file back. Future: `vault sync --restore <timestamp>`.

### 10.5 Atomic Writes

All config file writes follow the atomic write pattern to prevent corruption from crashes or power loss:

```
1. Write content to a temporary file: <config_path>.vault-tmp
2. Flush and sync the temp file to disk (fsync)
3. Rename temp file to the target path (atomic on all major filesystems)
4. If rename fails, the original file is untouched
```

---

## 11. Plugin Architecture

The connector system is designed for extensibility. New agent connectors can be added by:
1. Implementing the `AgentConnector` trait
2. Registering the connector in the `ConnectorRegistry`

### 11.1 AgentConnector Trait

```rust
// vault-connectors/src/traits.rs

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use vault_core::{
    error::VaultError,
    models::{
        agent::AgentType,
        mcp::McpEntry,
        sync::{SyncDiff, SyncResult},
    },
};

/// Parsed agent configuration.
/// This is a connector-agnostic representation of an agent's config file.
/// Each connector parses its specific format into this common structure.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// The raw JSON/TOML value of the entire config file.
    /// Preserved for round-trip fidelity — fields we don't understand
    /// are passed through untouched.
    pub raw: serde_json::Value,

    /// The parsed MCP server entries from the config.
    /// Key: MCP name, Value: MCP server config (command, args, env).
    pub mcp_servers: std::collections::HashMap<String, AgentMcpConfig>,
}

/// A single MCP server entry as represented in an agent's config file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentMcpConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

/// The core trait that every agent connector must implement.
///
/// Each method is designed to be independently testable and composable.
/// The `sync()` method orchestrates the full sync flow, but `diff()`,
/// `read_config()`, and `write_config()` can be called independently
/// for dry-runs, testing, and debugging.
///
/// # Contract
///
/// - `read_config()` must never fail if the config file exists and is valid.
///   If the file doesn't exist, it should return a default empty config.
/// - `write_config()` must use atomic writes (write-to-temp + rename).
/// - `write_config()` must preserve all keys/sections in the config that
///   are outside `mcpServers` (non-destructive merge).
/// - `backup()` must create an exact byte-copy of the current config file.
/// - `verify()` must re-read the written config and validate it parses correctly.
/// - All methods must be safe to call concurrently (Send + Sync).
#[async_trait]
pub trait AgentConnector: Send + Sync {
    /// Returns the agent type this connector handles.
    fn agent_type(&self) -> AgentType;

    /// Returns the path to the agent's configuration file.
    ///
    /// This is the file that will be read and written during sync.
    /// The path is platform-dependent (see Section 6 for paths per agent).
    fn config_path(&self) -> &Path;

    /// Read and parse the agent's current configuration.
    ///
    /// If the config file doesn't exist, returns a default (empty) config.
    /// If the config file exists but is malformed, returns VaultError::Config.
    async fn read_config(&self) -> Result<AgentConfig, VaultError>;

    /// Write the given configuration to the agent's config file.
    ///
    /// Uses atomic writes: write to temp file, fsync, rename.
    /// Preserves all non-mcpServers keys in the config.
    async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError>;

    /// Perform a full sync: compute diff, backup, apply, verify.
    ///
    /// This is the high-level orchestration method. It:
    /// 1. Reads current config
    /// 2. Computes diff against provided entries
    /// 3. Creates a backup
    /// 4. Applies changes
    /// 5. Writes config
    /// 6. Verifies the written config
    /// 7. Returns the result with full diff details
    async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError>;

    /// Compute the diff between the provided entries and the current config.
    ///
    /// Does NOT modify any files. Used for `--dry-run` and preview.
    async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError>;

    /// Create a backup of the current config file.
    ///
    /// Returns the path to the backup file.
    /// Backup location: ~/.agentvault/backups/<agent>/<timestamp>.<ext>
    fn backup(&self) -> Result<PathBuf, VaultError>;

    /// Verify the current config file is valid.
    ///
    /// Re-reads the config file and checks it parses correctly.
    /// Returns true if valid, false if corrupt.
    fn verify(&self) -> Result<bool, VaultError>;
}
```

### 11.2 Example Connector Implementation (Claude Code)

```rust
// vault-connectors/src/claude.rs

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vault_core::{
    error::VaultError,
    models::{agent::AgentType, mcp::McpEntry, sync::*},
};
use crate::traits::{AgentConfig, AgentConnector, AgentMcpConfig};

pub struct ClaudeConnector {
    config_path: PathBuf,
    backup_dir: PathBuf,
}

impl ClaudeConnector {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        Self {
            config_path: home.join(".claude").join("claude_desktop_config.json"),
            backup_dir: home.join(".agentvault").join("backups").join("claude"),
        }
    }

    /// Convert an McpEntry to the Claude config format.
    fn mcp_to_agent_config(entry: &McpEntry) -> AgentMcpConfig {
        AgentMcpConfig {
            command: entry.command.clone(),
            args: entry.args.clone(),
            env: entry.env_vars.clone(),
        }
    }
}

#[async_trait]
impl AgentConnector for ClaudeConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }

    fn config_path(&self) -> &Path {
        &self.config_path
    }

    async fn read_config(&self) -> Result<AgentConfig, VaultError> {
        if !self.config_path.exists() {
            return Ok(AgentConfig {
                raw: serde_json::json!({}),
                mcp_servers: HashMap::new(),
            });
        }

        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| VaultError::Io {
                path: self.config_path.clone(),
                source: e,
            })?;

        let raw: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| VaultError::Config {
                path: self.config_path.clone(),
                message: format!("Invalid JSON: {}", e),
            })?;

        let mcp_servers = raw
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(name, value)| {
                        serde_json::from_value::<AgentMcpConfig>(value.clone())
                            .ok()
                            .map(|config| (name.clone(), config))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(AgentConfig { raw, mcp_servers })
    }

    async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError> {
        // Rebuild the raw JSON with updated mcpServers
        let mut raw = config.raw.clone();
        let mcp_obj: serde_json::Value = serde_json::to_value(&config.mcp_servers)
            .map_err(|e| VaultError::Json {
                message: format!("Failed to serialize mcpServers: {}", e),
                source: Some(e),
            })?;
        raw["mcpServers"] = mcp_obj;

        // Atomic write: temp file → fsync → rename
        let temp_path = self.config_path.with_extension("vault-tmp");
        let content = serde_json::to_string_pretty(&raw)
            .map_err(|e| VaultError::Json {
                message: format!("Failed to serialize config: {}", e),
                source: Some(e),
            })?;

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| VaultError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        tokio::fs::write(&temp_path, &content)
            .await
            .map_err(|e| VaultError::Io {
                path: temp_path.clone(),
                source: e,
            })?;

        tokio::fs::rename(&temp_path, &self.config_path)
            .await
            .map_err(|e| VaultError::Io {
                path: self.config_path.clone(),
                source: e,
            })?;

        Ok(())
    }

    async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError> {
        let diff = self.diff(entries).await?;
        let timestamp = chrono::Utc::now();

        if diff.is_empty() {
            return Ok(SyncResult {
                agent_type: self.agent_type().to_string(),
                timestamp,
                diff,
                success: true,
                backup_path: None,
                error: None,
            });
        }

        // Create backup
        let backup_path = self.backup()?;

        // Read current config
        let mut config = self.read_config().await?;

        // Apply additions and updates
        for entry in entries {
            config
                .mcp_servers
                .insert(entry.name.clone(), Self::mcp_to_agent_config(entry));
        }

        // Apply removals (entries in config but not in vault entries)
        let vault_names: std::collections::HashSet<_> =
            entries.iter().map(|e| &e.name).collect();
        for removal in &diff.removals {
            config.mcp_servers.remove(&removal.name);
        }

        // Write and verify
        self.write_config(&config).await?;

        let valid = self.verify()?;
        if !valid {
            // Restore backup
            std::fs::copy(&backup_path, &self.config_path).map_err(|e| VaultError::Io {
                path: self.config_path.clone(),
                source: e,
            })?;
            return Err(VaultError::SyncVerificationFailed {
                agent: self.agent_type().to_string(),
                backup_path: backup_path.display().to_string(),
            });
        }

        Ok(SyncResult {
            agent_type: self.agent_type().to_string(),
            timestamp,
            diff,
            success: true,
            backup_path: Some(backup_path.display().to_string()),
            error: None,
        })
    }

    async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError> {
        let config = self.read_config().await?;
        let mut diff = SyncDiff::default();

        // Find additions and updates
        for entry in entries {
            match config.mcp_servers.get(&entry.name) {
                None => {
                    diff.additions.push(SyncEntry {
                        name: entry.name.clone(),
                        source: entry.source.to_string(),
                        version: entry.version.clone(),
                    });
                }
                Some(existing) => {
                    let mut changes = Vec::new();
                    if existing.command != entry.command {
                        changes.push(FieldChange {
                            field: "command".to_string(),
                            old_value: existing.command.clone(),
                            new_value: entry.command.clone(),
                        });
                    }
                    if existing.args != entry.args {
                        changes.push(FieldChange {
                            field: "args".to_string(),
                            old_value: format!("{:?}", existing.args),
                            new_value: format!("{:?}", entry.args),
                        });
                    }
                    if existing.env != entry.env_vars {
                        changes.push(FieldChange {
                            field: "env".to_string(),
                            old_value: format!("{:?}", existing.env),
                            new_value: format!("{:?}", entry.env_vars),
                        });
                    }
                    if !changes.is_empty() {
                        diff.updates.push(SyncUpdate {
                            name: entry.name.clone(),
                            changed_fields: changes,
                        });
                    }
                }
            }
        }

        // Find removals (entries in agent config that are vault-managed but not in entries)
        // NOTE: only remove entries we previously synced, not user-added ones
        let vault_names: std::collections::HashSet<_> =
            entries.iter().map(|e| e.name.as_str()).collect();
        for (name, _) in &config.mcp_servers {
            if !vault_names.contains(name.as_str()) {
                // TODO: Check sync_history to determine if this was vault-managed
                // For now, don't auto-remove (conservative approach)
            }
        }

        Ok(diff)
    }

    fn backup(&self) -> Result<PathBuf, VaultError> {
        if !self.config_path.exists() {
            return Err(VaultError::FileNotFound {
                path: self.config_path.clone(),
            });
        }

        std::fs::create_dir_all(&self.backup_dir).map_err(|e| VaultError::Io {
            path: self.backup_dir.clone(),
            source: e,
        })?;

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
        let backup_path = self.backup_dir.join(format!("{}.json", timestamp));

        std::fs::copy(&self.config_path, &backup_path).map_err(|e| VaultError::Io {
            path: backup_path.clone(),
            source: e,
        })?;

        Ok(backup_path)
    }

    fn verify(&self) -> Result<bool, VaultError> {
        if !self.config_path.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&self.config_path).map_err(|e| VaultError::Io {
            path: self.config_path.clone(),
            source: e,
        })?;

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
```

### 11.3 Adding a Custom Connector

To add support for a new AI agent:

1. **Create a new file** in `vault-connectors/src/`:

```rust
// vault-connectors/src/my_agent.rs

use crate::traits::{AgentConfig, AgentConnector};
use vault_core::models::agent::AgentType;

pub struct MyAgentConnector { /* ... */ }

#[async_trait]
impl AgentConnector for MyAgentConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::Custom("my-agent".to_string())
    }

    fn config_path(&self) -> &Path {
        // Return the path to the agent's config file
    }

    // ... implement all trait methods
}
```

2. **Register it** in `ConnectorRegistry::new()`:

```rust
registry.register(Box::new(my_agent::MyAgentConnector::new()));
```

3. **Add the variant** to `AgentType` enum (or use `Custom("my-agent")`).

### 11.4 Future: Dynamic Plugin Loading

Post v0.1.0, connectors will be loadable as dynamic libraries:

```
~/.agentvault/plugins/
├── connector-roocode.so     # Linux shared library
├── connector-hermes.dylib   # macOS dynamic library
└── connector-custom.wasm    # WASM plugin (future)
```

The plugin interface will use `libloading` for `.so`/`.dylib` and `wasmtime` for `.wasm`:

```rust
pub trait PluginConnector {
    fn create() -> Box<dyn AgentConnector>;
    fn metadata() -> PluginMetadata;
}
```

---

## Appendix A: Dependency Table

| Crate | Version | Purpose |
|---|---|---|
| `clap` | 4.x | CLI argument parsing |
| `serde` | 1.x | Serialization framework |
| `serde_json` | 1.x | JSON support |
| `toml` | 0.8.x | TOML support |
| `rusqlite` | 0.31.x | SQLite bindings |
| `chrono` | 0.4.x | Date/time handling |
| `dirs` | 5.x | Platform-specific directories |
| `sha2` | 0.10.x | SHA-256 checksums |
| `thiserror` | 1.x | Error derive macro |
| `anyhow` | 1.x | CLI-level error handling |
| `tracing` | 0.1.x | Structured logging |
| `tracing-subscriber` | 0.3.x | Log output formatting |
| `reqwest` | 0.12.x | HTTP client |
| `tokio` | 1.x | Async runtime |
| `async-trait` | 0.1.x | Async trait support |
| `colored` | 2.x | Terminal colors |
| `indicatif` | 0.17.x | Progress bars/spinners |
| `dialoguer` | 0.11.x | Interactive prompts |
| `tabled` | 0.15.x | Table formatting |
| `semver` | 1.x | Semantic version parsing |
| `ulid` | 1.x | ULID generation |

## Appendix B: Environment Variables

| Variable | Description | Default |
|---|---|---|
| `AGENTVAULT_DIR` | Override vault directory location | `~/.agentvault/` |
| `VAULT_LOG` | Set log level (trace, debug, info, warn, error) | `info` |
| `NO_COLOR` | Disable colored output | unset |
| `VAULT_NO_BACKUP` | Skip backup creation during sync | unset |

---

> **End of specification.** This document is sufficient for implementation. Each section maps directly to a module in the codebase and a phase in the [TODO tracker](./TODO.md).
