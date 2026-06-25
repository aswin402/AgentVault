# AgentVault CLI Reference

This document provides a comprehensive command-line reference for the `vault` binary.

## Global Options

These options apply to the `vault` command itself and can be specified before any subcommand.

* `-v`, `--verbose`
  Enable verbose output (show debug-level logs).
* `-q`, `--quiet`
  Suppress all output except errors.
* `--vault-dir <DIR>`
  Override the vault directory (default: `~/.agentvault/`). Can also be set via the `AGENTVAULT_DIR` environment variable.

---

## Commands

### `vault init`

Initialize a new AgentVault workspace. Creates the `~/.agentvault/` directory structure, initializes the SQLite registry database, and generates a default configuration file (`config.toml`).

This command is safe to run multiple times; it will not overwrite your installed capabilities.

#### Usage
```bash
vault init [OPTIONS]
```

#### Options
* `-d`, `--dir <DIR>`
  Override the vault directory location.
* `-f`, `--force`
  Force re-initialization (resets configuration to defaults, but preserves installed capabilities).

#### Examples
```bash
vault init
vault init --dir ~/custom-vault
```

---

### `vault install`

Install an MCP server, skill, or workflow into the vault.

The command automatically infers the source type based on the format:
- `npm:<package>`: Install from the npm registry.
- `pypi:<package>`: Install from PyPI.
- `github:<owner/repo>`: Clone from a GitHub repository.
- `local:<path>`: Reference a local directory.
- `docker:<image>`: Run from a Docker image.
- `<bare-name>`: Search the package registries to auto-detect.

#### Usage
```bash
vault install [OPTIONS] <SOURCE>
```

#### Options
* `-S`, `--source-type <TYPE>`
  Override the auto-detected source type (choices: `npm`, `pypi`, `github`, `local`, `docker`).
* `-V`, `--version <VERSION>`
  Version constraint (default: `"latest"`). Supports semver (e.g., `"^1.0"`).
* `-n`, `--name <NAME>`
  Explicit display name for this capability in the vault.
* `-e`, `--env <KEY=VALUE>`
  Environment variables to pass to the MCP server. Can be specified multiple times. Supports reference to external env vars via `KEY=env:VAR_NAME`.
* `-a`, `--args <ARG>`
  Arguments passed to the MCP server command. Can be specified multiple times.
* `-t`, `--transport <stdio|sse|http>`
  Transport protocol to use (default: `stdio`).
* `--url <URL>`
  URL for SSE or HTTP transports (required if transport is not stdio).
* `--agent <AGENT>`
  Target agents to sync this capability to. Can be specified multiple times. If omitted, syncs to all registered agents.
* `--tag <TAG>`
  Categorization tags. Can be specified multiple times.
* `-y`, `--yes`
  Skip confirmation prompts.
* `--skill`
  Install as a skill instead of an MCP server.
* `--workflow`
  Install as a workflow instead of an MCP server.

#### Examples
```bash
vault install npm:@anthropic/mcp-filesystem --args '/home/user/workspace'
vault install pypi:mcp-server-git --name git --env GITHUB_TOKEN=env:GITHUB_TOKEN
vault install local:/home/user/my-skill --skill
```

---

### `vault remove`

Remove an installed capability from the vault. Deletes the capability's files from disk and removes its entry from the registry database.

*Note: This does not automatically update agent configs. You must run `vault sync` after removal.*

#### Usage
```bash
vault remove [OPTIONS] <NAME>
```

#### Options
* `-f`, `--force`
  Skip confirmation prompt.
* `--keep-files`
  Remove from the registry but keep the files on disk.

#### Examples
```bash
vault remove filesystem
vault remove git --force
```

---

### `vault update`

Update installed capabilities to their latest versions, respecting version constraints defined at installation time.

#### Usage
```bash
vault update [OPTIONS] [NAME]
```

#### Options
* `-a`, `--all`
  Update all installed capabilities.
* `--dry-run`
  Show what would be updated without downloading or modifying files.
* `-f`, `--force`
  Bypass version constraints and force updates to the latest release.

#### Examples
```bash
vault update filesystem
vault update --all --dry-run
```

---

### `vault list`

List installed capabilities. By default, displays a summary of all capability types in a clean table.

#### Usage
```bash
vault list [OPTIONS]
```

#### Options
* `-m`, `--mcps`
  Show only MCP servers.
* `-s`, `--skills`
  Show only skills.
* `-w`, `--workflows`
  Show only workflows.
* `-a`, `--all`
  Show all capability types (default).
* `--json`
  Output the list as a raw JSON array.
* `--table`
  Force output as a formatted table.
* `--detail`
  Show full details including versions, paths, environment variables, and commands.

#### Examples
```bash
vault list
vault list --mcps --detail
vault list --skills --json
```

---

### `vault search`

Search for capabilities in the local registry or external registries.

#### Usage
```bash
vault search [OPTIONS] <QUERY>
```

#### Options
* `-s`, `--source <registry|npm|pypi|github>`
  Search source (local registry, npm, pypi, or github).
* `-l`, `--limit <LIMIT>`
  Maximum number of results to display (default: `20`).
* `--json`
  Output results in JSON format.

#### Examples
```bash
vault search filesystem
vault search memory --source npm --limit 5
```

---

### `vault sync`

Synchronize vault capabilities to target AI agent configurations. Backs up configurations before writing.

#### Usage
```bash
vault sync [OPTIONS] [AGENT]
```

#### Options
* `-a`, `--all`
  Sync to all registered agents.
* `--dry-run`
  Preview changes without modifying configurations.
* `-f`, `--force`
  Bypass safety merges and overwrite the agent's MCP config entirely.
* `--backup <true|false>`
  Explicitly control whether to create a backup file.
* `--prune`
  Remove any MCP entries from the agent configuration that are not in the vault.

#### Examples
```bash
vault sync claude
vault sync --all --prune
vault sync --all --dry-run
```

---

### `vault status`

Show health, paths, configuration summaries, and a general vault status.

#### Usage
```bash
vault status [OPTIONS]
```

#### Options
* `--json`
  Output status details in JSON format.

#### Examples
```bash
vault status
```

---

### `vault config`

View or modify AgentVault configuration.

#### Usage
```bash
# View entire config
vault config

# Get value of a config key
vault config <KEY>

# Set value of a config key
vault config <KEY> <VALUE>
```

#### Options
* `-l`, `--list`
  Print the configuration as simple `key=value` pairs.
* `--reset`
  Reset the entire configuration back to system defaults.

#### Examples
```bash
vault config
vault config default_agent
vault config default_agent claude
```

---

### `vault doctor`

Diagnose workspace and system environment issues. It verifies database integrity, vault directory structures, environment variables, connector health, and responsiveness of servers.

#### Usage
```bash
vault doctor [OPTIONS]
```

#### Options
* `-f`, `--fix`
  Attempt to automatically fix detected diagnostic issues.
* `--check-mcps`
  Attempt to run and query each installed MCP server to verify that it is fully responsive.

#### Examples
```bash
vault doctor
vault doctor --fix --check-mcps
```

---

### `vault connector`

Manage the agent connectors that connect AgentVault to external agent systems.

#### Subcommands

#### `vault connector add`
Register a new agent connector.
* **Usage**: `vault connector add <AGENT_TYPE> [OPTIONS]`
* **Options**:
  - `-c`, `--config-path <PATH>`: Override the default configuration path for the agent.
  - `--auto-sync`: Enable automatic synchronization for this agent when capabilities change.
* **Examples**:
  - `vault connector add claude`
  - `vault connector add cursor --config-path ~/.cursor/settings.json`

#### `vault connector list`
List all registered agent connectors.
* **Usage**: `vault connector list [OPTIONS]`
* **Options**:
  - `--json`: Output as JSON.

#### `vault connector remove`
Remove a registered agent connector.
* **Usage**: `vault connector remove <AGENT_TYPE> [OPTIONS]`
* **Options**:
  - `-f`, `--force`: Skip confirmation prompt.

---

### `vault export`

Export your current vault state (registry configurations, installed list, metadata) to a portable manifest file.

#### Usage
```bash
vault export [OPTIONS]
```

#### Options
* `-f`, `--format <toml|json>`
  Output file format (default: `toml`).
* `-o`, `--output <FILE>`
  Output file path (default: `./vault.toml` or `./vault.json`).

#### Examples
```bash
vault export
vault export --format json --output my-capabilities.json
```

---

### `vault import`

Import capabilities into your vault from an exported manifest file.

#### Usage
```bash
vault import [OPTIONS] <FILE>
```

#### Options
* `--dry-run`
  Preview changes without modifying the database or disk.
* `--merge`
  Add new capabilities and update existing ones (default strategy).
* `--replace`
  Replace the entire vault state. Capabilities not listed in the manifest will be removed.

#### Examples
```bash
vault import vault.toml
vault import vault.toml --replace --dry-run
```

---

### `vault serve`

Start AgentVault as a Model Context Protocol (MCP) server. This exposes the vault's management APIs (such as `list_capabilities`, `install_capability`, `remove_capability`, `update_capability`, `set_capability_env`, `search_registry`, `doctor_check`) as standardized MCP tools.

It can also run in gateway mode to dynamically aggregate all installed MCP servers into a single endpoint.

#### Usage
```bash
vault serve [OPTIONS]
```

#### Options
* `--gateway`
  Start in gateway mode: spawns all installed MCP servers in the background and aggregates their tool definitions under namespaced tool names (e.g. `server_name__tool_name`) behind a single unified stdio endpoint.

#### Examples
```bash
vault serve
vault serve --gateway
```

---

### `vault ui`

Start the interactive TUI dashboard. Provides a terminal-based interface to browse, monitor, configure, install, and update your capabilities.

#### Usage
```bash
vault ui [OPTIONS]
```

#### Options
* `-t`, `--theme <THEME>`
  Initial color theme to use (e.g. `slate`, `nord`, `dracula`, `monokai`).

#### Examples
```bash
vault ui --theme dracula
```

---

### `vault watch`

Watch agent configuration files and auto-sync capabilities whenever changes are detected. Can be run in the foreground or as a persistent daemon process.

#### Usage
```bash
vault watch [OPTIONS]
```

#### Options
* `-d`, `--daemon`
  Run in the background as a system daemon process.

#### Examples
```bash
vault watch
vault watch --daemon
```

---

### `vault completions`

Generate shell completion scripts for your favorite shell.

#### Usage
```bash
vault completions <SHELL>
```

#### Shell Choices
- `bash`
- `zsh`
- `fish`
- `powershell`

#### Examples
```bash
# Generate completions for Zsh
vault completions zsh

# Load completions immediately in Zsh
source <(vault completions zsh)
```
