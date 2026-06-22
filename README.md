# AgentVault 🛡️

> central local capability management system for AI agents.
> **Install once. Use everywhere.**

AgentVault acts as a centralized local capability layer that all your AI agents (Claude Code, Gemini CLI, OpenCode, Cursor, RooCode, etc.) can access. Instead of installing, configuring, and updating Model Context Protocol (MCP) servers, skills, and workflows separately for every agent, AgentVault manages them in one central repository and synchronizes their configurations dynamically.

---

## The Problem & Solution

Today developers use multiple AI agents. Each agent requires separate MCP installations, tool configurations, environment variables, and skill definitions. This leads to duplication, wasted disk space, configuration chaos, and version conflicts.

**AgentVault solves this by providing:**
1. **Unified Storage**: Centralized directory structure (`~/.agentvault/`) for all capabilities.
2. **SQLite Registry**: Structured relational tracking of MCPs, skills, workflows, configurations, and sync histories.
3. **Smart Connectors**: Native, secure read/write adapters to update and sync configs for different agents.
4. **Safety Defaults**: Automatic, encrypted backups of agent configurations before any synchronization is executed.

---

## Directory Layout

AgentVault maintains a clean directory structure under the user's home folder:

```text
~/.agentvault/
├── config.toml         # Central configurations (logs, paths, secrets filtering)
├── vault.db            # SQLite database registry
├── mcps/               # Centralized node/python environments for MCP servers
├── skills/             # Registered capability skills and prompt templates
├── workflows/          # Workflows and task runners
├── backups/            # Pre-synchronization agent configuration backups
└── logs/               # Detailed operation and sync execution logs
```

---

## Project Structure

This project is built in Rust using a modular Cargo workspace:

* **[`crates/vault-cli`](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-cli)**: Binary crate housing the command line interface logic (`vault`), arguments parsing, output tables, and TUI progress indicators.
* **[`crates/vault-core`](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-core)**: Core library crate containing SQLite registry management, migrations, configuration parsing, metadata schemas, and installer managers.
* **[`crates/vault-connectors`](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-connectors)**: Library crate holding adapters for various AI agents (Claude Code, Gemini CLI, OpenCode, Codex CLI).

---

## Implemented Commands (v0.0.1)

### `vault init`
Initializes a new AgentVault workspace under `~/.agentvault/`. It automatically creates all necessary directory pathways, compiles the SQLite registry schema, runs migrations, and generates a default `config.toml`.

### `vault status`
Reports the status of the vault environment:
- Active vault workspace paths.
- Total installed capability counts (MCPs, skills, workflows).
- SQLite registry metadata (disk size, status verification).
- Active agent connectors lists and their sync histories.

### `vault doctor`
Performs comprehensive diagnostic checks:
- Verifies directory exists and has write/read privileges.
- Performs SQLite integrity check (`PRAGMA integrity_check`).
- Scans database records vs. filesystem binaries for missing or orphaned capability assets.
- Resolves available compiler and runtime tools (`npm`, `npx`, `uv`, `pip`, `git`) and reports versions.

---

## Getting Started

### Prerequisites

You need Rust installed (v1.75+ recommended).

### Building

To build the workspace binaries, run:

```bash
cargo build --release
```

The resulting executable will be available at `./target/release/vault`.

### Running Tests

To run the unit and integration tests:

```bash
cargo test
```

### Running Diagnostics

After building, initialize the vault and run the diagnostic doctor command:

```bash
# Initialize storage and DB
cargo run --bin vault -- init

# Run system and registry health checks
cargo run --bin vault -- doctor

# Check current configuration and status
cargo run --bin vault -- status
```

---

## Development Roadmap

- [x] **Phase 0: Project Bootstrap** — Workspace setup, CLI command tree parsing, and CI pipelines.
- [x] **Phase 1: Storage Foundation** — SQLite registry database schema, TOML configuration manager, file system mappings, initialization, and diagnostic logs.
- [ ] **Phase 2: MCP Management Core** — Multi-runtime installer (npm, pip, venv, uv, raw git, local compilation) and database metadata registry sync.
- [ ] **Phase 3: Agent Configuration Connectors** — Claude Code, Gemini CLI, and OpenCode connectors implementation.
- [ ] **Phase 4: Sync Engine & CLI Completion** — Synchronizer matching agent templates, rollback backups, and dependency verifications.

---

## License

This project is licensed under the MIT License - see the `LICENSE` file for details.
