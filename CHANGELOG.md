# Changelog

All notable changes to **AgentVault** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

## [0.2.1] - 2026-06-24

### Added

- **Interactive TUI Dashboard (`vault ui`)**: A minimalist, multi-pane terminal interface built with `ratatui` and `crossterm`.
  - Built-in color themes (`slate`, `nord`, `dracula`, `monokai`) toggleable dynamically with key `t` or loaded via `--theme` flags.
  - Explore installed capabilities (MCP servers, skills, workflows) with a detailed metadata explorer.
  - Interactive shortcuts to run sync configurations (`s`), trigger doctor health checks (`d`), or update selected MCP servers (`u`) directly from the dashboard.
  - Multi-pane interface with action logs tracing sync/update operations in real time.

## [0.2.0] - 2026-06-24

### Added

- **Dynamic MCP Server Gateway (`vault serve`)**: Run AgentVault itself as a standardized, stdio-compliant Model Context Protocol (MCP) server. Exposes capability listing, installation, and removal operations as native MCP tools, allowing AI agents to dynamically query and manage their own local environment capabilities.
- **E2E Integration Testing**: Stream E2E JSON-RPC tests inside `test_e2e_serve_mcp` checking handshake, tool discovery, and routing parameters.

## [0.1.0] - 2026-06-24

### Added

- **Core CLI commands**
  - `vault install <source>` — Install MCP servers and capabilities
  - `vault remove <name>` — Remove installed capabilities
  - `vault update [name]` — Update installed capabilities to latest compatible versions
  - `vault list` — List all installed capabilities and their status
  - `vault search <query>` — Search the local registry for available capabilities
  - `vault sync <agent>` — Synchronize capability configurations across all connected agents
  - `vault config` — View and modify AgentVault configuration
  - `vault import` — Import capability configurations from external sources
  - `vault export` — Export capability configurations for sharing or backup
  - `vault completions <shell>` — Generate shell autocompletions script
- **Capability Management Extensions**
  - **Skills Management**: Model and install capability folders with `SKILL.md` frontmatter, supporting local paths and git repo cloning
  - **Workflows Management**: Support execution of multi-step `workflow.toml` graphs with Kahn's algorithm cycle detection and unresolved capability checks
- **Agent connectors**
  - Claude Code connector — Read/write Claude Code MCP configuration
  - Gemini CLI connector — Read/write Gemini CLI MCP configuration
  - OpenCode connector — Read/write OpenCode MCP configuration
  - Codex CLI connector — Read/write Codex CLI MCP configuration
- **Environment variable management** per MCP server with secure storage
- **Version pinning** — Lock capabilities to specific versions to prevent unintended upgrades
- **Shell completions** for bash, zsh, fish, and PowerShell
- **Cross-platform support** for Linux, macOS, and Windows
- **Man Pages** — Generated at build time via `clap_mangen`
- **Performance Optimizations** — Parallel updates check via tokio spawn and SQLite WAL mode connection settings
- **User-Facing Polish** — Mapped suggestion context for error messages at the CLI boundary and global `--verbose` flag to dump the cause chain

---

### Planned for v0.2.0

#### Added

- **Skill management** — Install, remove, and synchronize agent skills across tools
- **Workflow management** — Define and manage multi-step agent workflows
- **Capability abstraction** with dependency resolution across MCP servers, skills, and workflows
- **Additional agent connectors**
  - Cursor connector — Read/write Cursor MCP configuration
  - RooCode connector — Read/write RooCode MCP configuration

---

### Planned for v0.3.0

#### Added

- **TUI dashboard** built with `ratatui` for interactive capability management
- **File watcher** for automatic sync on configuration changes
- **Remote capability registry** — Discover and install capabilities from a shared registry
- **Plugin system** for custom third-party connectors

## [0.0.2] - 2026-06-23

### Added

- **MCP Management Core (Phase 2)**:
  - Implemented `McpManager` and `DefaultMcpManager` supporting installing MCP servers from NPM, PyPI (utilizing `uv`/`pip` in virtual environment virtualenv), and Local folders (symlinked).
  - Wired install, remove, list, and update subcommands in CLI.
- **Agent Connectors (Phase 3)**:
  - Defined generic `AgentConnector` trait and default methods for shared JSON config operations to maximize code reuse (reduced ~940 duplicate lines).
  - Implemented Claude Code, Gemini CLI, OpenCode, and Codex CLI connectors.
  - Implemented atomic config updates, backup, and JSON schema verification safeguards.
  - Implemented database `sync_history` logging and `SyncEngine` runner.
  - Wired `vault sync` and `vault connector` CLI subcommands.
- **Search & Discovery (Phase 4)**:
  - Implemented local fuzzy search registry matching (exact name, partial name, tag, and description matching).
  - Implemented live remote npm registry search via API.
  - Wired `vault search` CLI subcommand using tabled layouts and indicatif spinners.
- **Manifest & Declarative Config (Phase 5 - Tasks 1 & 2)**:
  - Designed TOML manifest schema for `vault.toml`.
  - Implemented `VaultManifest` model, parser, and semantic validation rules.
  - Wired `vault export` CLI subcommand to export database state to TOML/JSON.

---

## [0.0.1] - 2026-06-22

### Added

- **Project Scaffolding & Setup (Phase 0)**:
  - Centralized Cargo workspace configuration with `vault-cli`, `vault-core`, and `vault-connectors` member crates.
  - Implemented command-line parser definitions using `clap` derive API for all subcommands (install, remove, update, list, search, sync, status, config, init, doctor, connectors, export, import).
  - Setup lint rules, formatting conventions, tests, and CI/CD pipelines via GitHub Actions (Ubuntu + macOS, stable + nightly).
- **Storage Foundation (Phase 1)**:
  - SQLite registry migration execution to support tables for `mcps`, `skills`, `workflows`, `capabilities`, `agent_configs`, and `sync_history`.
  - Concrete implementation of registry CRUD operations with automatic table schema initialization on setup.
  - Configuration manager storing TOML configuration mapping settings (vault dir, log level, backups path, secret masking).
  - Standard directory layout setup inside the filesystem.
- **Completed CLI Commands**:
  - `vault init` — Automates creation of the SQLite registry, configuration file templates, and resource subdirectories inside `~/.agentvault`.
  - `vault status` — Displays detailed active information on vault directory, database metadata size/integrity, resource counts, and connected agents.
  - `vault doctor` — Runs comprehensive diagnostics on system tools (`npm`/`npx`, `uv`/`pip`, `git`), database integrity (`PRAGMA integrity_check`), missing/orphaned registry files, and storage folder read/write permissions.
- **Unit and Integration Tests**:
  - Validated SQLite registry capability metadata insertion, querying, updates, and deletion.
  - Validated config save/load cycles and logging setup.

---

## [0.0.0] - 2026-06-22

### Added

- Initial project scaffolding with Cargo workspace structure
- Project README with vision, architecture overview, and roadmap
- MIT license
- Contribution guidelines and code of conduct
- CI/CD foundation with GitHub Actions
- Documentation skeleton (`docs/`)
- `.gitignore` and editor configuration

---

[Unreleased]: https://github.com/aswin402/AgentVault/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/aswin402/AgentVault/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/aswin402/AgentVault/compare/v0.0.2...v0.1.0
[0.0.2]: https://github.com/aswin402/AgentVault/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/aswin402/AgentVault/compare/v0.0.0...v0.0.1
[0.0.0]: https://github.com/aswin402/AgentVault/releases/tag/v0.0.0
