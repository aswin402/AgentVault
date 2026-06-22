# Changelog

All notable changes to **AgentVault** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned for v0.1.0 (MVP)

#### Added

- **Core CLI commands**
  - `vault init` — Initialize a new AgentVault workspace
  - `vault install` — Install MCP servers and capabilities
  - `vault remove` — Remove installed capabilities
  - `vault update` — Update installed capabilities to latest compatible versions
  - `vault list` — List all installed capabilities and their status
  - `vault search` — Search the local registry for available capabilities
  - `vault sync` — Synchronize capability configurations across all connected agents
  - `vault doctor` — Diagnose workspace health and connectivity issues
  - `vault status` — Display current workspace state and agent connections
  - `vault config` — View and modify AgentVault configuration
  - `vault import` — Import capability configurations from external sources
  - `vault export` — Export capability configurations for sharing or backup
- **Agent connectors**
  - Claude Code connector — Read/write Claude Code MCP configuration
  - Gemini CLI connector — Read/write Gemini CLI MCP configuration
  - OpenCode connector — Read/write OpenCode MCP configuration
  - Codex CLI connector — Read/write Codex CLI MCP configuration
- **SQLite-backed local registry** for capability metadata and state tracking
- **Config backup before sync** — Automatic snapshots of agent configs prior to any write operation
- **Environment variable management** per MCP server with secure storage
- **Version pinning** — Lock capabilities to specific versions to prevent unintended upgrades
- **Shell completions** for bash, zsh, fish, and PowerShell
- **Structured logging** with `tracing` for diagnostics and debugging
- **Cross-platform support** for Linux, macOS, and Windows

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

[Unreleased]: https://github.com/AswinkumarGP/AgentVault/compare/v0.0.0...HEAD
[0.0.0]: https://github.com/AswinkumarGP/AgentVault/releases/tag/v0.0.0
