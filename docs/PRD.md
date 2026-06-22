# AgentVault — Product Requirements Document

> **Version:** 1.0.0
> **Status:** Draft
> **Author:** AgentVault Core Team
> **Created:** 2026-06-22
> **Last Updated:** 2026-06-22

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Target Users](#3-target-users)
4. [Product Goals](#4-product-goals)
5. [Core Concepts](#5-core-concepts)
6. [Functional Requirements](#6-functional-requirements)
7. [Non-Functional Requirements](#7-non-functional-requirements)
8. [User Stories](#8-user-stories)
9. [Success Metrics](#9-success-metrics)
10. [Risks and Mitigations](#10-risks-and-mitigations)
11. [MVP Scope vs Future Scope](#11-mvp-scope-vs-future-scope)
12. [Tech Stack](#12-tech-stack)

---

## 1. Executive Summary

**AgentVault** is a local-first capability management system for AI agents. It provides a single, unified interface for installing, configuring, versioning, and syncing capabilities — MCP servers, skills, workflows, and bundled capability packs — across every AI coding agent a developer uses.

**The core promise: Install once. Use everywhere.**

Today's AI-assisted development landscape is fragmented. Developers routinely work with multiple AI agents — Claude Code, Gemini CLI, OpenCode, Codex CLI, Cursor, RooCode, Hermes, and more. Each agent has its own configuration format, its own directory for MCP servers, its own skill definitions, and its own mechanism for environment variables and tool permissions. The result is a sprawling, duplicated, brittle configuration surface that grows linearly with every new agent adopted.

AgentVault eliminates this fragmentation tax. It acts as a **capability orchestration layer** — a single source of truth at `~/.agentvault/` that knows what you have installed, what versions are running, what environment variables are required, and how to project that state into the native configuration format of each connected agent. When you run `vault install mcp/github`, AgentVault installs the GitHub MCP server once, resolves its dependencies, stores its configuration centrally, and then syncs it to Claude Code's `claude_desktop_config.json`, Gemini CLI's `settings.json`, OpenCode's config, and any other connected agent — automatically.

AgentVault is not a new AI agent. It is the **infrastructure layer beneath all of them** — the package manager, configuration manager, and sync engine that makes the multi-agent development workflow tenable at scale.

---

## 2. Problem Statement

### 2.1 The Multi-Agent Reality

Modern developers are no longer using a single AI coding assistant. The competitive landscape has produced a proliferation of capable, specialized agents:

| Agent | Primary Interface | Config Location |
|-------|------------------|-----------------|
| **Claude Code** | CLI / IDE | `~/.claude/` |
| **Gemini CLI** | CLI / IDE | `~/.gemini/` |
| **OpenCode** | CLI | `~/.config/opencode/` |
| **Codex CLI** | CLI | `~/.codex/` |
| **Cursor** | IDE | `~/.cursor/` |
| **RooCode** | IDE Extension | `.roo/` (project-level) |
| **Hermes** | CLI | `~/.hermes/` |

Each agent brings unique strengths — different models, different tool-use paradigms, different context window strategies. Power users rationally choose to keep multiple agents in their toolkit, switching between them based on task requirements.

### 2.2 The Fragmentation Tax

This multi-agent reality creates a compounding set of pain points:

#### 2.2.1 Duplication of Installations

Every MCP server must be installed separately for each agent. The `@modelcontextprotocol/server-filesystem` package might exist in three different `node_modules` directories, consuming disk space and bandwidth for each redundant installation. A developer with 5 MCP servers across 3 agents has **15 installation points** to manage instead of 5.

#### 2.2.2 Wasted Storage

MCP servers and their dependency trees are not trivial. A single MCP server with its `node_modules` can consume 50–200 MB. Multiply by the number of agents, and developers are burning gigabytes of disk space on identical copies of identical packages.

#### 2.2.3 Configuration Chaos

Each agent stores MCP configurations in a different format:

- Claude Code uses a JSON object in `claude_desktop_config.json` with `mcpServers` keys
- Gemini CLI uses a `settings.json` with a different schema
- OpenCode uses TOML-based configuration
- Cursor embeds MCP config in its own settings format

There is no standard. A developer who wants the same set of MCP servers available everywhere must manually author and maintain parallel configuration files in different formats, in different locations, with different key names.

#### 2.2.4 Version Conflicts and Drift

Without centralized version management, different agents can end up running different versions of the same MCP server. This leads to subtle behavioral differences — a query that works in Claude Code fails in Gemini CLI because the GitHub MCP server is v1.2.0 in one and v1.0.3 in the other. There is no mechanism to detect, report, or resolve these inconsistencies.

#### 2.2.5 Discovery Fragmentation

Skills, prompts, and workflows are scattered across agent-specific directories. A useful skill written for Claude Code (`~/.claude/commands/`) cannot be discovered or reused by Gemini CLI (`~/.gemini/config/skills/`). Knowledge and automation remain siloed within each agent's ecosystem.

#### 2.2.6 Environment Variable Sprawl

MCP servers frequently require API keys and secrets — `GITHUB_TOKEN`, `POSTGRES_CONNECTION_STRING`, `ANTHROPIC_API_KEY`. Each agent may handle environment variable injection differently. Developers end up scattering secrets across `.env` files, shell profiles, and agent-specific config files, with no centralized view of what credentials are in use or where they are stored.

#### 2.2.7 Maintenance Burden

Every update, every new MCP server, every configuration change must be replicated across all agents manually. This is not a one-time cost — it is an ongoing operational burden that scales with the number of agents and capabilities in use. It discourages experimentation ("I won't try this new MCP server because I'd have to set it up in four places") and increases the risk of stale or broken configurations.

### 2.3 The Missing Layer

The AI agent ecosystem has package registries (npm, PyPI), configuration formats (JSON, TOML, YAML), and runtime environments — but it is missing the **orchestration layer** that ties them together across agents. AgentVault is that missing layer.

---

## 3. Target Users

### 3.1 Primary: AI Power Users

Developers who use **two or more AI coding agents** as part of their daily workflow. They are comfortable with the command line, actively manage their development environment, and feel the pain of fragmented configuration acutely. They are early adopters, tool-builders, and multiplier-effect users who influence their teams' tooling choices.

**Characteristics:**
- Use 2–5 AI agents regularly
- Have 3–10 MCP servers installed
- Spend 30+ minutes per month on agent configuration maintenance
- Value local-first, privacy-respecting tools
- Prefer CLI over GUI for infrastructure management

### 3.2 Secondary: Teams Standardizing AI Tooling

Engineering teams that want to ensure consistent AI agent capabilities across all team members. Today, there is no mechanism for a tech lead to say "everyone on this team should have the GitHub MCP server, the Postgres MCP server, and the code-review skill installed and configured identically." AgentVault's `vault.toml` manifest file enables this — teams can check a declarative capability manifest into their repository and run `vault sync` to ensure consistency.

**Characteristics:**
- 3–50 person engineering teams
- Mixed agent preferences across team members
- Need reproducible, auditable agent configurations
- Want to version-control their team's AI tooling setup

### 3.3 Tertiary: MCP Server and Skill Authors

Developers who build and distribute MCP servers, skills, or workflows. AgentVault provides a standard packaging format and distribution mechanism that works across all agents, rather than requiring authors to write integration instructions for each agent individually.

---

## 4. Product Goals

### 4.1 Install Once, Use Everywhere

A capability installed through AgentVault is immediately available to every connected agent. No manual replication. No per-agent configuration. One install command, universal availability.

### 4.2 Local-First

All data lives on the developer's machine at `~/.agentvault/`. No cloud accounts, no telemetry, no remote dependencies for core functionality. The registry is a local SQLite database. Configuration is local TOML files. MCP server binaries and packages are stored locally. AgentVault works fully offline after initial installation of capabilities.

### 4.3 Agent-Agnostic

AgentVault does not favor any particular AI agent. It treats all agents as equal sync targets through a plugin-based **Agent Connector** architecture. Adding support for a new agent is a matter of writing a new connector — no changes to the core system required.

### 4.4 Single Source of Truth

`~/.agentvault/` is the canonical location for all capability metadata, configuration, and state. Agent-specific configuration files are **derived outputs** — generated and maintained by AgentVault's sync engine. Developers should never need to manually edit an agent's MCP config file again.

### 4.5 Declarative and Reproducible

Capabilities can be declared in a `vault.toml` manifest file. Running `vault sync` on a machine with this manifest file will converge the local AgentVault state to match the declaration — installing missing capabilities, removing unlisted ones, and updating versions as specified. This enables reproducible agent environments across machines and team members.

### 4.6 Zero-Friction Adoption

AgentVault must be installable in under 60 seconds. It must detect existing agent installations and offer to import their current MCP configurations. The first `vault sync` should make the developer's life better, not worse.

---

## 5. Core Concepts

### 5.1 MCP (Model Context Protocol Server)

An **MCP server** is an external tool that runs as a separate process and communicates with AI agents via the Model Context Protocol. MCP servers extend agent capabilities beyond text generation — they provide structured access to filesystems, databases, APIs, browsers, and other external systems.

**Examples:**
- `@modelcontextprotocol/server-filesystem` — Read/write files on the local filesystem
- `@modelcontextprotocol/server-github` — Interact with GitHub repositories, issues, PRs
- `@modelcontextprotocol/server-postgres` — Query PostgreSQL databases
- `@anthropic/mcp-server-puppeteer` — Browser automation via Puppeteer
- `@anthropic/mcp-server-memory` — Persistent memory across conversations

**AgentVault's role with MCPs:**
- Install and manage the underlying packages (npm, pip, cargo, binary)
- Store canonical configuration (command, args, env vars)
- Pin and manage versions with semver constraints
- Sync MCP server definitions to each agent's native config format
- Manage required environment variables centrally

### 5.2 Skill

A **skill** is a reusable instruction set that teaches an AI agent how to perform a specific task or follow a specific methodology. Skills are typically Markdown files with structured frontmatter, containing prompts, guidelines, and contextual instructions.

**Examples:**
- A "code-review" skill that instructs agents on how to perform thorough code reviews
- A "git-workflow" skill that enforces conventional commits and branch naming
- A "rust-best-practices" skill with idiomatic Rust patterns and anti-patterns
- A "debugging" skill that teaches systematic root-cause analysis

**AgentVault's role with Skills:**
- Store skills centrally in `~/.agentvault/skills/`
- Sync skills to each agent's native skill directory format
  - Claude Code: `~/.claude/commands/`
  - Gemini CLI: `~/.gemini/config/skills/<name>/SKILL.md`
  - OpenCode: agent-specific skill format
- Enable skill discovery across agents (a skill installed for Claude Code is also available in Gemini CLI)
- Support skill versioning and updates

### 5.3 Workflow

A **workflow** is a composed sequence of skills and tool invocations designed to accomplish a multi-step task. Workflows define the order of operations, decision points, data flow between steps, and success criteria.

**Examples:**
- A "feature-development" workflow: create branch → write tests → implement → review → merge
- A "database-migration" workflow: generate migration → validate schema → apply → verify
- A "incident-response" workflow: diagnose → hotfix → test → deploy → postmortem

**AgentVault's role with Workflows:**
- Store workflow definitions in `~/.agentvault/workflows/`
- Resolve workflow dependencies (a workflow referencing a skill ensures that skill is installed)
- Provide a standard workflow definition format that can be translated to agent-specific formats
- Enable workflow sharing and reuse across agents

### 5.4 Capability

A **capability** is AgentVault's highest-level abstraction. It is a bundle that can contain any combination of MCP servers, skills, and workflows, along with metadata about their interdependencies. Capabilities provide **automatic dependency resolution** — installing a capability installs everything it needs.

**Examples:**
- A "full-stack-dev" capability that bundles filesystem MCP, GitHub MCP, Postgres MCP, a code-review skill, and a git-workflow skill
- A "data-engineering" capability that bundles Postgres MCP, a SQL-best-practices skill, and a migration workflow
- A "security-audit" capability that bundles specific MCP servers for SAST/DAST tools and a security review skill

**Capability manifest structure:**
```toml
[capability]
name = "full-stack-dev"
version = "1.0.0"
description = "Everything you need for full-stack development with AI agents"

[capability.mcps]
filesystem = { version = "^1.0" }
github = { version = "^1.2" }
postgres = { version = "^0.5" }

[capability.skills]
code-review = { version = "^2.0" }
git-workflow = { version = "^1.0" }

[capability.workflows]
feature-development = { version = "^1.0" }
```

**AgentVault's role with Capabilities:**
- Resolve the full dependency tree when a capability is installed
- Track which individual MCPs, skills, and workflows were installed as part of which capability
- Enable atomic install/remove of capability bundles
- Prevent orphaned dependencies when capabilities are removed

### 5.5 Agent Connector

An **agent connector** is a plugin that understands a specific AI agent's configuration format and directory structure. Connectors are responsible for translating AgentVault's internal canonical representation of capabilities into the native format expected by each agent.

**Each connector must implement:**
- **Detection:** Determine if the agent is installed on the system
- **Read:** Parse the agent's existing configuration to enable import
- **Write:** Generate the agent's native configuration from AgentVault state
- **Sync:** Reconcile differences between AgentVault state and agent config
- **Validate:** Verify that the generated configuration is valid for the agent

**Built-in connectors (MVP):**

| Connector | Agent | Config Path | Format |
|-----------|-------|-------------|--------|
| `claude` | Claude Code | `~/.claude/claude_desktop_config.json` | JSON |
| `gemini` | Gemini CLI | `~/.gemini/settings.json` | JSON |
| `opencode` | OpenCode | `~/.config/opencode/config.toml` | TOML |
| `codex` | Codex CLI | `~/.codex/config.json` | JSON |

**Connector plugin interface:**
```rust
pub trait AgentConnector: Send + Sync {
    /// Unique identifier for this connector (e.g., "claude", "gemini")
    fn id(&self) -> &str;

    /// Human-readable name (e.g., "Claude Code")
    fn display_name(&self) -> &str;

    /// Check if the agent is installed on this system
    fn detect(&self) -> Result<bool>;

    /// Read the agent's current MCP configuration
    fn read_config(&self) -> Result<AgentConfig>;

    /// Write AgentVault state to the agent's config format
    fn write_config(&self, state: &VaultState) -> Result<()>;

    /// Compute the diff between current agent config and desired state
    fn diff(&self, state: &VaultState) -> Result<SyncDiff>;

    /// Validate that a proposed config would be accepted by the agent
    fn validate(&self, config: &AgentConfig) -> Result<Vec<ValidationWarning>>;
}
```

---

## 6. Functional Requirements

### 6.1 MCP Management

#### FR-MCP-01: Install MCP Server
Users must be able to install an MCP server from a supported package source.

```bash
vault install mcp/filesystem                    # Latest version
vault install mcp/github@1.2.0                  # Specific version
vault install mcp/postgres --version "^0.5"     # Semver constraint
vault install mcp/custom --source ./local/path  # Local source
```

**Behavior:**
- Resolve the package from the appropriate registry (npm, PyPI, crates.io, or custom)
- Download and install the package to `~/.agentvault/mcps/<name>/`
- Create a canonical configuration entry in the local registry
- Prompt for required environment variables if not already configured
- Trigger an automatic sync to all connected agents (unless `--no-sync` is passed)
- Display a summary of what was installed and where it was synced

#### FR-MCP-02: Remove MCP Server
Users must be able to remove an installed MCP server.

```bash
vault remove mcp/filesystem
vault remove mcp/github --force  # Skip confirmation
```

**Behavior:**
- Remove the package from `~/.agentvault/mcps/<name>/`
- Remove the configuration entry from the local registry
- Remove the MCP server definition from all connected agent configs
- Warn if the MCP is a dependency of an installed capability
- Prompt for confirmation unless `--force` is passed

#### FR-MCP-03: Update MCP Server
Users must be able to update installed MCP servers.

```bash
vault update mcp/filesystem             # Update to latest within constraint
vault update mcp/github@2.0.0           # Update to specific version
vault update --all                       # Update all installed MCPs
vault update --dry-run                   # Show what would be updated
```

**Behavior:**
- Check for available updates against the source registry
- Respect version pinning constraints in `vault.toml`
- Download and install the new version
- Re-sync all connected agents
- Display a changelog or version diff when available

#### FR-MCP-04: Search for MCP Servers
Users must be able to search for available MCP servers.

```bash
vault search filesystem                  # Search by name
vault search --tag database              # Search by tag
vault search --source npm                # Filter by source registry
```

**Behavior:**
- Query configured registries for matching packages
- Display results with name, version, description, download count, and trust level
- Indicate which results are already installed locally

#### FR-MCP-05: List Installed MCP Servers
Users must be able to list all installed MCP servers with their status.

```bash
vault list                               # List all capabilities
vault list mcps                          # List MCPs only
vault list mcps --verbose                # Detailed view with versions, paths, env vars
vault list mcps --json                   # JSON output for scripting
```

**Behavior:**
- Display name, version, source, install date, and sync status for each MCP
- Indicate which agents each MCP is synced to
- Flag any version mismatches or sync failures

#### FR-MCP-06: Version Pinning
Users must be able to pin MCP server versions using semver constraints.

```bash
vault pin mcp/github@1.2.0              # Pin to exact version
vault pin mcp/github ">=1.0, <2.0"      # Pin to range
vault unpin mcp/github                   # Remove version pin
```

#### FR-MCP-07: Environment Variable Management
Users must be able to manage environment variables required by MCP servers.

```bash
vault env set GITHUB_TOKEN ghp_xxxx     # Set an env var
vault env get GITHUB_TOKEN              # Get an env var (masked by default)
vault env list                           # List all managed env vars
vault env list --show-values             # List with unmasked values
vault env remove GITHUB_TOKEN           # Remove an env var
vault env import .env                    # Import from .env file
```

**Behavior:**
- Store environment variables in `~/.agentvault/env.vault` (encrypted at rest)
- Associate env vars with the MCP servers that require them
- Inject env vars into agent configs during sync (respecting each agent's env var format)
- Warn when an MCP server is installed but its required env vars are not set

### 6.2 Skill Management

#### FR-SKILL-01: Install Skill
```bash
vault install skill/code-review          # From registry
vault install skill/custom --source ./path/to/skill  # Local
```

#### FR-SKILL-02: Remove Skill
```bash
vault remove skill/code-review
```

#### FR-SKILL-03: List Skills
```bash
vault list skills
vault list skills --verbose
```

#### FR-SKILL-04: Sync Skills to Agents
Skills must be synced to each agent's native skill format:
- Claude Code: copied/symlinked to `~/.claude/commands/`
- Gemini CLI: copied/symlinked to `~/.gemini/config/skills/<name>/SKILL.md`
- Other agents: as defined by their respective connectors

### 6.3 Workflow Management

#### FR-WF-01: Install Workflow
```bash
vault install workflow/feature-dev       # From registry
vault install workflow/custom --source ./path  # Local
```

#### FR-WF-02: Remove Workflow
```bash
vault remove workflow/feature-dev
```

#### FR-WF-03: List Workflows
```bash
vault list workflows
vault list workflows --verbose
```

### 6.4 Capability Management

#### FR-CAP-01: Install Capability Bundle
```bash
vault install capability/full-stack-dev
```

**Behavior:**
- Resolve the full dependency tree (MCPs + skills + workflows)
- Install all components that are not already installed
- Track the association between the capability and its components
- Sync all new components to connected agents

#### FR-CAP-02: Remove Capability Bundle
```bash
vault remove capability/full-stack-dev
```

**Behavior:**
- Remove components that are not dependencies of other installed capabilities
- Preserve shared components with a reference count mechanism
- Update the registry and sync agents

### 6.5 Agent Integration

#### FR-AGENT-01: Detect Installed Agents
```bash
vault agents                             # List detected agents
vault agents --verbose                   # Show config paths and sync status
```

**Behavior:**
- Scan for known agent installations using each connector's detection logic
- Report which agents are detected, their config file locations, and current sync status

#### FR-AGENT-02: Sync to Agents
```bash
vault sync                               # Sync all capabilities to all agents
vault sync --agent claude                # Sync to specific agent only
vault sync --dry-run                     # Show what would change
vault sync --force                       # Overwrite manual agent config changes
```

**Behavior:**
- For each connected agent, compute the diff between current agent config and desired state
- Apply changes to agent config files
- Report what was added, removed, or updated in each agent's config
- Preserve any agent-specific configuration that AgentVault does not manage (non-destructive merge)

#### FR-AGENT-03: Import from Agent
```bash
vault import --agent claude              # Import existing MCPs from Claude Code
vault import --agent gemini              # Import from Gemini CLI
vault import --all                       # Import from all detected agents
```

**Behavior:**
- Read the agent's current MCP configuration
- Create corresponding entries in the AgentVault registry
- Detect and resolve conflicts (same MCP with different versions across agents)
- Present a summary for user confirmation before committing

#### FR-AGENT-04: Agent-Specific Overrides
Users must be able to specify per-agent overrides for capabilities.

```toml
# vault.toml
[mcps.github]
version = "^1.2"

[mcps.github.agents.claude]
env = { GITHUB_TOKEN = "claude-specific-token" }

[mcps.github.agents.gemini]
enabled = false  # Don't sync GitHub MCP to Gemini
```

### 6.6 Registry

#### FR-REG-01: Local SQLite Registry
AgentVault must maintain a local SQLite database at `~/.agentvault/registry.db` tracking:
- All installed capabilities (MCPs, skills, workflows, capability bundles)
- Version information and semver constraints
- Installation source and timestamp
- Dependency relationships between capabilities
- Sync status per agent (last synced timestamp, hash of synced config)
- Environment variable associations

#### FR-REG-02: Registry Integrity
```bash
vault doctor                             # Check registry integrity
vault doctor --fix                       # Attempt to repair issues
```

**Behavior:**
- Verify that all registry entries correspond to actual files on disk
- Verify that all managed files on disk have corresponding registry entries
- Check for orphaned dependencies
- Validate environment variable completeness
- Report and optionally repair inconsistencies

### 6.7 CLI Interface

The CLI must follow modern CLI conventions with clear, composable subcommands.

#### Command Summary

| Command | Description |
|---------|-------------|
| `vault install <type>/<name>` | Install a capability |
| `vault remove <type>/<name>` | Remove a capability |
| `vault update [<type>/<name>]` | Update capabilities |
| `vault list [type]` | List installed capabilities |
| `vault search <query>` | Search for capabilities |
| `vault sync [--agent <name>]` | Sync to agent configs |
| `vault status` | Show overall vault status |
| `vault agents` | List detected agents and sync status |
| `vault import --agent <name>` | Import from an agent's existing config |
| `vault env <subcommand>` | Manage environment variables |
| `vault config <subcommand>` | Manage AgentVault configuration |
| `vault doctor` | Check vault health and integrity |
| `vault init` | Initialize a new `vault.toml` in current directory |
| `vault pin <type>/<name>@<version>` | Pin a capability version |
| `vault unpin <type>/<name>` | Unpin a capability version |

#### CLI Design Principles
- **Colored output:** Use ANSI colors for status indicators (green = success, yellow = warning, red = error)
- **Progress indicators:** Show progress bars for long-running operations (downloads, syncs)
- **JSON output:** Support `--json` flag on all list/status commands for scripting
- **Dry-run:** Support `--dry-run` on all mutating commands
- **Verbosity levels:** Support `-v`, `-vv`, `-vvv` for increasing detail
- **Confirmation prompts:** Require confirmation for destructive operations unless `--force` is passed
- **Shell completions:** Generate completions for bash, zsh, fish, PowerShell

### 6.8 Configuration

#### FR-CFG-01: Global Configuration
AgentVault stores its global configuration at `~/.agentvault/config.toml`.

```toml
[vault]
# Default sync behavior
auto_sync = true          # Automatically sync after install/remove/update
sync_interval = "manual"  # "manual" | "on-change" | "hourly" | "daily"

[vault.defaults]
trust_level = "community"  # "official" | "verified" | "community" | "local"

[agents]
# Per-agent enable/disable
claude = { enabled = true, path = "~/.claude/" }
gemini = { enabled = true, path = "~/.gemini/" }
opencode = { enabled = true, path = "~/.config/opencode/" }
codex = { enabled = false }  # Disabled

[registries]
# Capability registries to search
default = "https://registry.agentvault.dev"
npm = { url = "https://registry.npmjs.org", type = "npm" }
```

#### FR-CFG-02: Project-Level Manifest (`vault.toml`)
Developers can place a `vault.toml` file in their project root to declare required capabilities.

```toml
[vault]
min_version = "0.1.0"

[mcps]
filesystem = { version = "^1.0" }
github = { version = "^1.2", env = ["GITHUB_TOKEN"] }
postgres = { version = "^0.5", env = ["DATABASE_URL"] }

[skills]
code-review = { version = "^2.0" }
git-workflow = { version = "^1.0" }

[workflows]
feature-development = { version = "^1.0" }

[agents]
# Project-level agent overrides
claude = { enabled = true }
gemini = { enabled = true }
```

**Behavior:**
- `vault sync` in a directory with `vault.toml` uses the manifest as the desired state
- Missing capabilities are installed automatically (with user confirmation)
- Extra capabilities not in the manifest are left untouched (additive merge)
- `vault init` creates a starter `vault.toml` based on current vault state

---

## 7. Non-Functional Requirements

### 7.1 Performance

| Operation | Target Latency | Notes |
|-----------|---------------|-------|
| `vault list` | < 50ms | Local SQLite query, no I/O beyond DB |
| `vault status` | < 100ms | Registry query + filesystem stat checks |
| `vault sync` | < 500ms | Per-agent config generation and write |
| `vault install` (cached) | < 2s | Local package extraction, no network |
| `vault install` (network) | < 30s | Depends on package size and network speed |
| `vault search` | < 3s | Network-dependent, with local cache |
| CLI startup time | < 50ms | No lazy initialization on critical path |

**Implementation strategies:**
- Pre-compile all SQL queries as prepared statements
- Use memory-mapped I/O for large file operations
- Cache registry query results for the duration of a single CLI invocation
- Use async I/O for network operations with concurrent downloads
- Profile and benchmark all hot paths in CI

### 7.2 Security

#### 7.2.1 Environment Variable Protection
- Environment variables containing secrets (API keys, tokens) must be encrypted at rest in `~/.agentvault/env.vault`
- Use OS-native credential storage where available (macOS Keychain, Linux `secret-service`, Windows Credential Manager)
- Fall back to file-based encryption with a user-provided passphrase
- Never log, print, or include secrets in error messages (mask with `***`)

#### 7.2.2 Trust Levels
Every capability in the registry has an assigned trust level:

| Level | Description | Verification |
|-------|-------------|-------------|
| **Official** | Published by AgentVault core team | Cryptographically signed |
| **Verified** | Published by known authors, reviewed | Author signature verified |
| **Community** | Published by community members | Checksum verified only |
| **Local** | Installed from local filesystem | No verification |

- Users can configure minimum trust levels in `config.toml`
- Installing capabilities below the configured trust threshold requires explicit `--trust` flag
- Display trust level prominently during install and in `vault list`

#### 7.2.3 Permission Controls
- AgentVault must never modify agent configs without explicit user action (`vault sync` or `auto_sync = true`)
- All file writes should use atomic operations (write to temp file, then rename) to prevent corruption
- Agent config backups are created before every sync operation at `~/.agentvault/backups/`
- Rollback mechanism: `vault sync --rollback` restores the previous agent config state

#### 7.2.4 Supply Chain Security
- Verify package checksums after download before installation
- Support pinning specific package hashes in `vault.toml` for reproducible builds
- Log all install/update operations in an audit trail at `~/.agentvault/audit.log`

### 7.3 Extensibility

#### 7.3.1 Plugin Architecture for Connectors
- Agent connectors are implemented as Rust trait objects loaded at runtime
- Built-in connectors ship with the binary
- Community connectors can be distributed as separate binaries that implement the connector protocol
- Connector API is versioned and backward-compatible within major versions

#### 7.3.2 Hook System
- Pre-sync and post-sync hooks allow users to run custom scripts
- Pre-install and post-install hooks for package customization
- Hooks are configured in `config.toml` or `vault.toml`

```toml
[hooks]
pre_sync = "~/.agentvault/hooks/pre-sync.sh"
post_sync = "~/.agentvault/hooks/post-sync.sh"
post_install = "~/.agentvault/hooks/post-install.sh"
```

### 7.4 Reliability

#### 7.4.1 Crash Safety
- All registry mutations are wrapped in SQLite transactions
- File system operations use atomic writes (temp file + rename)
- Incomplete installations are rolled back automatically
- A lock file (`~/.agentvault/.lock`) prevents concurrent vault operations

#### 7.4.2 Backup and Recovery
- Agent config backups are stored at `~/.agentvault/backups/<agent>/<timestamp>/`
- Configurable backup retention (default: last 10 syncs per agent)
- `vault sync --rollback` restores the most recent backup
- `vault doctor --fix` can rebuild the registry from on-disk state

#### 7.4.3 Graceful Degradation
- If an agent connector fails, sync continues for other agents (partial success)
- If a registry query fails, CLI falls back to cached data with a warning
- Network failures during search/install produce clear error messages with retry instructions

### 7.5 Portability

- Support Linux, macOS, and Windows
- Respect XDG Base Directory Specification on Linux
- Use platform-appropriate default paths via the `dirs` crate
- Single static binary distribution (no runtime dependencies)
- Shell completion scripts for bash, zsh, fish, and PowerShell

---

## 8. User Stories

### US-01: First-Time Setup
**As a** developer using Claude Code and Gemini CLI,
**I want to** install AgentVault and import my existing MCP configurations,
**So that** I have a unified view of all my agent capabilities immediately.

**Acceptance Criteria:**
- `vault agents` detects both Claude Code and Gemini CLI installations
- `vault import --all` reads MCP configs from both agents
- Duplicate MCPs (same server in both agents) are deduplicated with the higher version kept
- After import, `vault list` shows all unique MCPs with their source agents
- Running `vault sync` makes both agents' configs consistent

### US-02: Install a New MCP Server
**As a** developer,
**I want to** install the GitHub MCP server once and have it available in all my agents,
**So that** I don't have to configure it separately in each agent.

**Acceptance Criteria:**
- `vault install mcp/github` downloads and installs the server
- AgentVault prompts for `GITHUB_TOKEN` if not already configured
- After installation, `vault sync` updates Claude Code, Gemini CLI, and OpenCode configs
- The GitHub MCP server appears in each agent's config with the correct format
- The MCP server is functional when invoked from any connected agent

### US-03: Remove an MCP Server
**As a** developer,
**I want to** remove an MCP server from all agents with a single command,
**So that** I don't have leftover configurations in some agents.

**Acceptance Criteria:**
- `vault remove mcp/postgres` removes the server from AgentVault's registry
- `vault sync` removes the Postgres MCP entry from all connected agent configs
- The package files are deleted from `~/.agentvault/mcps/postgres/`
- If the MCP is part of a capability bundle, the user is warned before removal

### US-04: Version Consistency
**As a** developer,
**I want to** ensure all my agents use the same version of each MCP server,
**So that** I get consistent behavior regardless of which agent I'm using.

**Acceptance Criteria:**
- `vault status` shows version information for each MCP across all agents
- `vault status` flags any version mismatches between agents
- `vault sync --force` resolves mismatches by applying AgentVault's canonical version
- `vault pin mcp/github@1.2.0` locks the version and prevents automatic updates

### US-05: Team Standardization
**As a** tech lead,
**I want to** define a `vault.toml` in our repository specifying required capabilities,
**So that** every team member has a consistent AI tooling setup.

**Acceptance Criteria:**
- A `vault.toml` file in the project root declares required MCPs, skills, and workflows
- `vault sync` in the project directory detects the manifest and installs missing capabilities
- The manifest supports version constraints (e.g., `version = "^1.2"`)
- Missing environment variables are reported with clear instructions
- `vault status` in the project directory reports compliance with the manifest

### US-06: Manage Secrets Securely
**As a** developer,
**I want to** manage my API keys and tokens centrally,
**So that** I don't have secrets scattered across multiple agent config files.

**Acceptance Criteria:**
- `vault env set GITHUB_TOKEN ghp_xxxx` stores the token encrypted
- `vault env list` shows env vars with values masked by default
- `vault sync` injects env vars into agent configs using each agent's native mechanism
- `vault env get GITHUB_TOKEN --show` reveals the value after confirmation
- Env vars are never written to log files or error messages in plaintext

### US-07: Discover New Capabilities
**As a** developer,
**I want to** search for available MCP servers, skills, and workflows,
**So that** I can discover and adopt new capabilities easily.

**Acceptance Criteria:**
- `vault search database` returns relevant MCP servers, skills, and workflows
- Results include name, description, version, trust level, and install count
- Already-installed capabilities are marked in search results
- `vault search --tag productivity` filters by tag
- Search results link to documentation or source repositories

### US-08: Audit and Health Check
**As a** developer,
**I want to** check the health of my AgentVault installation,
**So that** I can identify and fix configuration issues before they cause problems.

**Acceptance Criteria:**
- `vault doctor` checks registry integrity, file existence, env var completeness, and sync status
- Issues are categorized as errors (must fix), warnings (should fix), and info (nice to know)
- `vault doctor --fix` automatically repairs issues where possible
- A clear report is generated showing what was checked and what was found

### US-09: Selective Agent Sync
**As a** developer,
**I want to** control which capabilities are synced to which agents,
**So that** I can customize each agent's toolset without losing central management.

**Acceptance Criteria:**
- `vault.toml` supports per-agent overrides (`[mcps.github.agents.claude]`)
- Individual capabilities can be disabled for specific agents (`enabled = false`)
- `vault sync --agent claude` syncs only to Claude Code
- `vault list --agent gemini` shows only capabilities synced to Gemini CLI

### US-10: Update All Capabilities
**As a** developer,
**I want to** update all my MCP servers to their latest versions with a single command,
**So that** I stay current without manual effort.

**Acceptance Criteria:**
- `vault update --all` checks for updates to all installed MCPs
- A summary of available updates is displayed before proceeding
- Version pins are respected (pinned MCPs are skipped with a note)
- `vault update --dry-run` shows what would be updated without making changes
- After updating, `vault sync` propagates the new versions to all agents

### US-11: Offline Operation
**As a** developer working on an airplane,
**I want to** use AgentVault to manage and sync my existing capabilities without network access,
**So that** I can reconfigure my agents even when offline.

**Acceptance Criteria:**
- `vault list`, `vault status`, `vault sync`, and `vault remove` work fully offline
- `vault install` from local sources works offline
- `vault install` from remote sources produces a clear "no network" error with guidance
- `vault search` checks local cache first, then reports network unavailability

### US-12: Rollback a Sync
**As a** developer,
**I want to** undo a sync operation that broke an agent's configuration,
**So that** I can quickly recover from configuration errors.

**Acceptance Criteria:**
- Every `vault sync` creates a timestamped backup of each modified agent config
- `vault sync --rollback` restores the most recent backup for each agent
- `vault sync --rollback --agent claude` restores only Claude Code's config
- Backup history is visible via `vault status --backups`
- At least the last 10 backups per agent are retained

---

## 9. Success Metrics

### 9.1 Adoption Metrics

| Metric | Target (6 months) | Measurement |
|--------|-------------------|-------------|
| GitHub stars | 1,000+ | GitHub API |
| Monthly active installs | 500+ | Download statistics |
| Agent connectors available | 4+ (built-in) | Repository count |
| Community connectors | 2+ | Third-party repositories |

### 9.2 Usage Metrics (opt-in anonymous telemetry)

| Metric | Target | Measurement |
|--------|--------|-------------|
| Avg. MCPs managed per user | 5+ | Opt-in usage report |
| Avg. agents connected per user | 2.5+ | Opt-in usage report |
| Sync operations per user per week | 10+ | Opt-in usage report |
| `vault doctor` success rate | > 95% | Opt-in usage report |

### 9.3 Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| CLI crash rate | < 0.1% | Error reporting |
| Sync correctness | > 99.9% | Integration tests |
| Test coverage | > 80% | CI pipeline |
| Average issue resolution time | < 7 days | GitHub Issues |

### 9.4 User Satisfaction Signals

- **Time saved:** Users report spending < 5 min/month on agent configuration (vs. 30+ min/month before)
- **Discovery:** Users install 2x more capabilities after adopting AgentVault (reduced friction → more experimentation)
- **Retention:** 70%+ of users who install AgentVault are still using it after 30 days
- **NPS:** Net Promoter Score > 50 (surveyed quarterly)

---

## 10. Risks and Mitigations

### Risk 1: Agent Config Format Changes

**Risk:** AI agents frequently update their configuration formats. A Claude Code update that changes the MCP config schema could break AgentVault's sync.

**Impact:** High — broken sync means AgentVault becomes harmful rather than helpful.

**Mitigation:**
- Each connector includes format version detection and supports multiple schema versions
- Integration tests run against real agent config formats, updated with every agent release
- Connectors validate output against the target agent's expected schema before writing
- Rapid-response process for shipping connector patches when agent formats change
- Config backups ensure recoverability even if a sync writes an invalid config

### Risk 2: Security of Centralized Secrets

**Risk:** Centralizing environment variables (API keys, tokens) in `~/.agentvault/env.vault` creates a high-value target. A single compromise exposes all secrets.

**Impact:** Critical — leaked API keys can cause financial damage, data breaches.

**Mitigation:**
- Encrypt secrets at rest using OS-native credential storage (Keychain, secret-service, Credential Manager)
- File permissions set to `600` (owner-only read/write)
- Support hardware security key integration for high-security environments (future)
- Audit logging of all secret access operations
- Option to use external secret managers (HashiCorp Vault, 1Password CLI) as backends (future)

### Risk 3: Agent Ecosystem Consolidation

**Risk:** The AI agent market consolidates around 1–2 dominant agents, reducing the need for multi-agent management.

**Impact:** Medium — reduces the core value proposition.

**Mitigation:**
- AgentVault provides value even with a single agent (centralized management, version pinning, declarative config)
- Position AgentVault as the "dotfiles for AI agents" — valuable for reproducibility regardless of agent count
- Expand scope to include project-level capability management (team standardization use case)
- Build community around skills and workflows that transcend individual agents

### Risk 4: Adoption Friction

**Risk:** Developers are reluctant to add another tool to their workflow. "I'll just keep doing it manually."

**Impact:** High — directly affects adoption.

**Mitigation:**
- Zero-config import: `vault import --all` migrates existing configs in one command
- Immediate value: first sync resolves version inconsistencies the user didn't know they had
- Non-destructive: AgentVault never deletes agent configs it doesn't manage (additive merge)
- Single binary: no runtime dependencies, install via `curl | sh` or `cargo install`
- Ship with excellent documentation and a 2-minute quickstart video

### Risk 5: MCP Protocol Evolution

**Risk:** The Model Context Protocol itself evolves significantly, introducing breaking changes to how MCP servers are configured and launched.

**Impact:** Medium — may require fundamental changes to how AgentVault manages MCPs.

**Mitigation:**
- Abstract MCP management behind an internal interface that can adapt to protocol changes
- Active participation in MCP specification discussions
- Version-aware MCP handling: support running different MCP protocol versions simultaneously
- Maintain backward compatibility with older MCP servers for at least 2 major versions

### Risk 6: Data Loss from Sync Bugs

**Risk:** A bug in the sync engine overwrites or corrupts an agent's configuration file, causing data loss of manual agent-specific settings.

**Impact:** High — erodes user trust immediately.

**Mitigation:**
- Non-destructive merge: AgentVault only modifies keys it manages, preserving all other configuration
- Pre-sync backup: every sync creates a timestamped copy of the agent's config before any modification
- Diff preview: `vault sync --dry-run` shows exactly what would change
- Atomic writes: config files are written to a temp file first, then atomically renamed
- Comprehensive integration tests covering merge scenarios with manual config entries

### Risk 7: Connector Maintenance Burden

**Risk:** Maintaining connectors for a growing number of agents becomes unsustainable for a small team.

**Impact:** Medium — stale connectors with broken compatibility frustrate users.

**Mitigation:**
- Well-defined, stable connector trait interface that makes writing connectors straightforward
- Community connector contributions via the plugin architecture
- Automated connector testing against real agent installations in CI
- Prioritize connectors for agents with the largest user bases
- Provide a connector development guide and template to lower contribution barriers

---

## 11. MVP Scope vs Future Scope

### 11.1 MVP (v0.1.0)

The MVP focuses on the core value loop: **install an MCP → sync to agents → manage centrally.**

#### In Scope

| Feature | Priority | Status |
|---------|----------|--------|
| **MCP install/remove/update/list** | P0 | Planned |
| **Claude Code connector** | P0 | Planned |
| **Gemini CLI connector** | P0 | Planned |
| **Local SQLite registry** | P0 | Planned |
| **`vault sync` to connected agents** | P0 | Planned |
| **`vault import` from agents** | P0 | Planned |
| **`vault status` overview** | P0 | Planned |
| **`vault doctor` health check** | P1 | Planned |
| **Environment variable management** | P1 | Planned |
| **Version pinning (semver)** | P1 | Planned |
| **`vault.toml` project manifest** | P1 | Planned |
| **Config backup before sync** | P1 | Planned |
| **`--dry-run` on mutating commands** | P1 | Planned |
| **`--json` output for scripting** | P2 | Planned |
| **Shell completions (bash, zsh, fish)** | P2 | Planned |

#### Out of Scope (MVP)

| Feature | Reason |
|---------|--------|
| Skills management | Focus MVP on MCPs which have the clearest cross-agent value |
| Workflows management | Depends on skills being stable first |
| Capability bundles | Higher-order abstraction; needs MCPs and skills working first |
| Remote registry / search | MVP uses local-only management; search comes in v0.2 |
| OpenCode / Codex / Cursor connectors | Stretch goals; Claude + Gemini cover the majority of users |
| Encrypted secret storage | MVP uses plaintext env vars in `env.toml` with restrictive file permissions |
| GUI / TUI dashboard | CLI-only for MVP |
| Plugin system for connectors | Built-in connectors only for MVP |
| Auto-sync / file watching | Manual `vault sync` only for MVP |

### 11.2 v0.2.0 — Extended Agent Support & Skills

| Feature | Description |
|---------|-------------|
| OpenCode connector | Sync to OpenCode's TOML-based config |
| Codex CLI connector | Sync to Codex CLI's config format |
| Skill management | Install, remove, list, sync skills across agents |
| Remote registry | Search and install from a central capability registry |
| `vault search` | Query the registry for MCPs and skills |
| Encrypted env var storage | OS-native credential storage for secrets |

### 11.3 v0.3.0 — Workflows & Capabilities

| Feature | Description |
|---------|-------------|
| Workflow management | Install, remove, list, sync workflows |
| Capability bundles | Install bundled MCPs + skills + workflows |
| Dependency resolution | Automatic dependency tree resolution for capabilities |
| Cursor connector | Sync to Cursor IDE's MCP config |
| RooCode connector | Sync to RooCode's project-level config |

### 11.4 v1.0.0 — Production Ready

| Feature | Description |
|---------|-------------|
| Plugin architecture for connectors | Dynamically loadable connector plugins |
| Auto-sync with file watching | Watch `vault.toml` and agent configs for changes |
| TUI dashboard | Interactive terminal UI for managing the vault |
| Team sharing | Push/pull capability sets to/from a team registry |
| Hook system | Pre/post hooks for sync and install operations |
| Audit logging | Complete audit trail of all vault operations |
| Signed packages | Cryptographic verification of capability packages |

---

## 12. Tech Stack

AgentVault is built in **Rust** for performance, reliability, and single-binary distribution.

### 12.1 Core Dependencies

| Crate | Purpose | Justification |
|-------|---------|---------------|
| **[clap](https://crates.io/crates/clap)** `v4.x` | CLI argument parsing | Industry standard for Rust CLIs. Derive macros for type-safe arg definitions. Auto-generates help text and shell completions. |
| **[serde](https://crates.io/crates/serde)** `v1.x` | Serialization/deserialization | The Rust ecosystem's universal serialization framework. Required for JSON, TOML, and any structured data handling. |
| **[toml](https://crates.io/crates/toml)** `v0.8.x` | TOML parsing and generation | For `vault.toml` manifests and `config.toml`. Integrates seamlessly with serde. |
| **[serde_json](https://crates.io/crates/serde_json)** `v1.x` | JSON parsing and generation | For reading and writing agent config files (Claude Code, Gemini CLI use JSON). |
| **[rusqlite](https://crates.io/crates/rusqlite)** `v0.31.x` | SQLite database | Local registry storage. Bundled SQLite (no external dependency). Zero-config, single-file database with ACID transactions. |
| **[reqwest](https://crates.io/crates/reqwest)** `v0.12.x` | HTTP client | For downloading MCP packages and querying remote registries. Async with tokio. TLS support built-in. |
| **[tokio](https://crates.io/crates/tokio)** `v1.x` | Async runtime | Industry-standard async runtime for Rust. Required for concurrent downloads, file watching, and non-blocking I/O. |
| **[indicatif](https://crates.io/crates/indicatif)** `v0.17.x` | Progress bars and spinners | Rich terminal progress indicators for downloads, syncs, and long-running operations. |
| **[colored](https://crates.io/crates/colored)** `v2.x` | Terminal colors | Ergonomic ANSI color output for status messages, errors, and warnings. |
| **[thiserror](https://crates.io/crates/thiserror)** `v1.x` | Error types | Derive macro for creating well-structured error enums with display formatting. |
| **[semver](https://crates.io/crates/semver)** `v1.x` | Semantic versioning | Parse, compare, and match semver version strings and requirements. Essential for version pinning and update logic. |
| **[dirs](https://crates.io/crates/dirs)** `v5.x` | Platform directories | Cross-platform resolution of home directories, config dirs, and data dirs. Respects XDG on Linux, standard paths on macOS/Windows. |

### 12.2 Additional Dependencies

| Crate | Purpose | Justification |
|-------|---------|---------------|
| **[tracing](https://crates.io/crates/tracing)** | Structured logging | Async-aware structured logging for debugging and audit trails. |
| **[tracing-subscriber](https://crates.io/crates/tracing-subscriber)** | Log output formatting | Configurable log output (console, file, JSON). |
| **[sha2](https://crates.io/crates/sha2)** | Checksumming | SHA-256 checksums for package integrity verification. |
| **[tempfile](https://crates.io/crates/tempfile)** | Temporary files | Atomic file writes (write to temp → rename) for crash safety. |
| **[walkdir](https://crates.io/crates/walkdir)** | Directory traversal | Efficient recursive directory scanning for file discovery. |
| **[dialoguer](https://crates.io/crates/dialoguer)** | Interactive prompts | Confirmation dialogs, selection menus, and input prompts for the CLI. |
| **[tabled](https://crates.io/crates/tabled)** | Table formatting | Pretty-printed tables for `vault list` and `vault status` output. |
| **[chrono](https://crates.io/crates/chrono)** | Date/time handling | Timestamps for audit logs, backups, and sync metadata. |

### 12.3 Development Dependencies

| Tool/Crate | Purpose |
|------------|---------|
| **[assert_cmd](https://crates.io/crates/assert_cmd)** | CLI integration testing |
| **[predicates](https://crates.io/crates/predicates)** | Test assertions |
| **[assert_fs](https://crates.io/crates/assert_fs)** | Filesystem test fixtures |
| **[mockall](https://crates.io/crates/mockall)** | Mock trait implementations |
| **[insta](https://crates.io/crates/insta)** | Snapshot testing for CLI output |
| **[criterion](https://crates.io/crates/criterion)** | Benchmarking |
| **cargo-deny** | License and dependency auditing |
| **cargo-clippy** | Lint checking |
| **cargo-fmt** | Code formatting |

### 12.4 Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                     CLI Layer (clap)                     │
│   vault install │ remove │ sync │ list │ search │ ...   │
├─────────────────────────────────────────────────────────┤
│                   Command Handlers                       │
│   InstallCmd │ RemoveCmd │ SyncCmd │ ListCmd │ ...       │
├─────────────────────────────────────────────────────────┤
│                    Core Engine                           │
│  ┌──────────┐  ┌───────────┐  ┌──────────────────────┐  │
│  │ Registry │  │ Resolver  │  │   Sync Engine        │  │
│  │ (SQLite) │  │ (semver)  │  │                      │  │
│  └──────────┘  └───────────┘  │  ┌────────────────┐  │  │
│                               │  │ Claude Conn.   │  │  │
│  ┌──────────┐  ┌───────────┐  │  ├────────────────┤  │  │
│  │ Package  │  │ Config    │  │  │ Gemini Conn.   │  │  │
│  │ Manager  │  │ Manager   │  │  ├────────────────┤  │  │
│  │ (reqwest)│  │ (toml)    │  │  │ OpenCode Conn. │  │  │
│  └──────────┘  └───────────┘  │  ├────────────────┤  │  │
│                               │  │ Codex Conn.    │  │  │
│  ┌──────────┐  ┌───────────┐  │  └────────────────┘  │  │
│  │ Env Var  │  │ Backup    │  └──────────────────────┘  │
│  │ Store    │  │ Manager   │                             │
│  └──────────┘  └───────────┘                             │
├─────────────────────────────────────────────────────────┤
│                  Storage Layer                           │
│  ~/.agentvault/                                         │
│  ├── config.toml         # Global configuration          │
│  ├── registry.db         # SQLite capability registry    │
│  ├── env.vault           # Encrypted environment vars    │
│  ├── audit.log           # Operation audit trail         │
│  ├── mcps/               # Installed MCP servers         │
│  │   ├── filesystem/                                     │
│  │   ├── github/                                         │
│  │   └── postgres/                                       │
│  ├── skills/             # Installed skills              │
│  ├── workflows/          # Installed workflows           │
│  └── backups/            # Agent config backups          │
│      ├── claude/                                         │
│      └── gemini/                                         │
└─────────────────────────────────────────────────────────┘
```

### 12.5 Build and Distribution

| Aspect | Approach |
|--------|----------|
| **Build system** | Cargo (Rust's native build system) |
| **CI/CD** | GitHub Actions with matrix builds (Linux, macOS, Windows) |
| **Binary distribution** | Static binaries via GitHub Releases |
| **Package managers** | `cargo install agentvault`, Homebrew tap, AUR |
| **Install script** | `curl -fsSL https://agentvault.dev/install.sh \| sh` |
| **Minimum Rust version** | 1.75.0 (for stable async trait support) |
| **Binary size target** | < 15 MB (release build with LTO and strip) |

---

## Appendix A: Directory Structure

```
~/.agentvault/
├── config.toml              # Global AgentVault configuration
├── registry.db              # SQLite database tracking all installed capabilities
├── env.vault                # Encrypted environment variables
├── audit.log                # Append-only audit log of all operations
├── .lock                    # File lock for concurrent operation prevention
├── mcps/                    # Installed MCP servers
│   ├── filesystem/
│   │   ├── package/         # The actual MCP server package
│   │   └── manifest.toml    # AgentVault metadata for this MCP
│   ├── github/
│   └── postgres/
├── skills/                  # Installed skills
│   ├── code-review/
│   │   ├── SKILL.md
│   │   └── manifest.toml
│   └── git-workflow/
├── workflows/               # Installed workflows
│   └── feature-dev/
│       ├── workflow.toml
│       └── manifest.toml
├── backups/                 # Agent config backups
│   ├── claude/
│   │   ├── 2026-06-22T10-00-00/
│   │   └── 2026-06-21T15-30-00/
│   └── gemini/
├── cache/                   # Download cache for packages
│   └── npm/
└── hooks/                   # User-defined hook scripts
    ├── pre-sync.sh
    └── post-sync.sh
```

## Appendix B: Glossary

| Term | Definition |
|------|-----------|
| **AgentVault** | The capability management system described by this document |
| **Capability** | A high-level bundle of MCPs, skills, and workflows |
| **Connector** | A plugin that translates AgentVault state to an agent's native config format |
| **MCP** | Model Context Protocol — a standard for tools that extend AI agent capabilities |
| **MCP Server** | A process implementing the MCP that provides specific functionality |
| **Registry** | The local SQLite database tracking installed capabilities |
| **Skill** | A reusable instruction set that teaches agents specific tasks |
| **Sync** | The process of propagating AgentVault state to agent config files |
| **Vault** | The `~/.agentvault/` directory containing all AgentVault data |
| **Workflow** | A composed sequence of skills and tools for multi-step tasks |

---

> **This document is the source of truth for AgentVault's product direction.** All implementation decisions, architectural trade-offs, and scope negotiations should reference this PRD. Changes to this document require review and versioned updates.
