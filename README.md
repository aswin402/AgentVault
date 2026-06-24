# AgentVault 🛡️

> **Central local capability management layer for AI agents.**
> **Install once. Use everywhere.**

AgentVault acts as a centralized local capability registry that all your AI coding agents (Claude Code, Gemini CLI, OpenCode, Codex CLI, Cursor, etc.) can access. Instead of installing, configuring, and updating Model Context Protocol (MCP) servers, skills, and workflows separately for every agent, AgentVault manages them in one central repository and synchronizes their configurations dynamically.

---

## 🚀 Key Features

* **Unified Storage**: Centralized directory structure (`~/.agentvault/`) for all capability binaries, configuration templates, and settings.
* **SQLite Registry (WAL Mode)**: Relational tracking of MCPs, skills, workflows, agent connector paths, and execution sync histories.
* **Smart Connectors**: Native, secure adapters to read/write capability configurations for **Claude Code**, **Gemini CLI**, **OpenCode**, and **Codex CLI**.
* **Safety Defaults**: Automatic, encrypted/structured backups before any synchronization is executed.
* **Skills Management**: Register prompt directories containing a `SKILL.md` (with YAML frontmatter) from local paths or git clone sources.
* **Workflow Runner**: Parse and validate multi-step execution graphs defined in `workflow.toml` using Kahn's topological sorting algorithm (with cycle detection and unresolved capability checks).
* **Parallel CLI Updates**: Runs check for all update operations concurrently utilizing Tokio-spawned tasks.
* **Error suggestion contexts**: Human-friendly suggetions mapped dynamically at the CLI boundary, print the complete cause chain with `--verbose`.
* **Shell Autocompletions & Man Pages**: Autocompletions for Bash, Zsh, Fish, and PowerShell, and man pages generated automatically during compile time.

---

## 📂 Directory Layout

AgentVault maintains a clean directory structure under the user's home folder:

```text
~/.agentvault/
├── config.toml         # Central configurations (logs, paths, secrets filtering)
├── vault.db            # SQLite database registry (WAL mode)
├── mcps/               # Centralized node/python environments for MCP servers
├── skills/             # Registered capability skills and prompt templates
├── workflows/          # Workflows and task runners
├── backups/            # Pre-synchronization agent configuration backups
└── logs/               # Detailed operation and sync execution logs
```

---

## 🛠️ Installation

```bash
# Install the latest stable version
curl -fsSL https://raw.githubusercontent.com/aswin402/AgentVault/main/install.sh | bash
```

Alternatively, you can compile from source:
```bash
git clone https://github.com/aswin402/AgentVault.git
cd AgentVault
cargo build --release
cp target/release/vault ~/.local/bin/
```

---

## 📖 Command Reference

### `vault init`
Initialize a new vault workspace directory. Generates the default configuration file and SQLite registry database.
```bash
vault init [--force] [--dir <custom_path>]
```

### `vault install`
Install a new capability (MCP, Skill, or Workflow) into the vault.
```bash
# Install an MCP from NPM
vault install npm:@anthropic/mcp-filesystem --args '/home/user/projects'

# Install an MCP from PyPI
vault install pypi:mcp-server-memory

# Install a local directory as a Skill
vault install local:/path/to/my-skill --skill

# Install a Workflow
vault install local:/path/to/workflow.toml --workflow
```

### `vault remove`
Remove an installed capability. Automatically detects if the capability is an MCP, Skill, or Workflow.
```bash
vault remove my-mcp [--keep-files] [--force]
```

### `vault update`
Update installed capabilities to their latest versions concurrently.
```bash
vault update my-mcp
vault update --all
```

### `vault list`
List registered capabilities.
```bash
vault list [--mcps] [--skills] [--workflows] [--json] [--detail]
```

### `vault search`
Fuzzy search capabilities locally or look up packages in the remote NPM registry.
```bash
vault search memory
```

### `vault sync`
Synchronize installed capabilities to the configuration file of one or all agent connectors.
```bash
vault sync claude
vault sync --all [--force] [--prune]
```

### `vault status`
Show overall health status, active paths, capability counts, and recent sync history.
```bash
vault status [--json]
```

### `vault config`
View or modify configuration parameters.
```bash
vault config
vault config default_agent claude
```

### `vault doctor`
Diagnose vault health, database integrity, missing binaries, and prerequisites.
```bash
vault doctor [--fix]
```

### `vault connector`
Manage active agent connectors.
```bash
vault connector add claude
vault connector list
```

### `vault export` / `vault import`
Export or import vault manifest state in TOML or JSON format.
```bash
vault export --output manifest.toml
vault import manifest.toml [--replace]
```

### `vault completions`
Generate shell autocompletions.
```bash
source <(vault completions zsh)
```

---

## 🧪 Running Tests

To run the unit and integration test suite:
```bash
cargo test
```

To check compiler guidelines and Clippy warnings:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 📄 License

This project is licensed under the MIT License - see the `LICENSE` file for details.
