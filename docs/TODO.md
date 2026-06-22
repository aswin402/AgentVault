# AgentVault — TODO / Task Tracker

> **A local-first capability management system for AI agents, built in Rust.**

---

## Status Legend

| Symbol | Meaning |
|--------|---------|
| `- [ ]` | Not started |
| `- [x]` | Complete |
| `- [-]` | In progress |
| `- [~]` | Blocked / Waiting |
| `- [!]` | Needs revision |

---

## Phase 0: Project Bootstrap

- [x] Initialize Cargo workspace (`Cargo.toml` with `[workspace]` and `members = ["vault-cli", "vault-core", "vault-connectors"]`)
- [x] Create `vault-cli` crate with `Cargo.toml` and CLI dependencies (`clap`, `colored`, `indicatif`, `dialoguer`, `tabled`, `anyhow`, `tracing-subscriber`)
- [x] Create `vault-core` crate with `Cargo.toml` and core dependencies (`serde`, `toml`, `serde_json`, `rusqlite`, `semver`, `chrono`, `dirs`, `sha2`, `thiserror`, `tracing`, `reqwest`, `tokio`)
- [x] Create `vault-connectors` crate with `Cargo.toml` (depends on `vault-core`)
- [x] Create `.gitignore` for Rust (target/, *.swp, .env, *.db)
- [x] Define `VaultError` enum in `vault-core/src/error.rs` with variants:
  - [x] `Io` — filesystem and general I/O errors
  - [x] `Database` — SQLite operation failures
  - [x] `Config` — configuration parsing/validation errors
  - [x] `Network` — HTTP/download failures
  - [x] `Connector` — agent connector read/write failures
  - [x] `McpInstall` — MCP installation failures (npm, pip, git)
  - [x] `NotFound` — requested capability not found in registry
  - [x] `AlreadyExists` — duplicate install attempt
  - [x] `VersionConflict` — semver constraint violation
  - [x] `PermissionDenied` — filesystem permission errors
  - [x] `Serialization` — serde serialization/deserialization errors
- [x] Set up `tracing-subscriber` in `vault-cli/src/main.rs` with env filter (`VAULT_LOG`)
- [x] Define ALL CLI subcommands with `clap` derive API — each printing `"not yet implemented"`:
  - [x] `vault install <source>` — install a capability (MCP, skill, workflow)
  - [x] `vault remove <name>` — remove an installed capability
  - [x] `vault update [name]` — update one or all capabilities
  - [x] `vault list` — list installed capabilities
  - [x] `vault search <query>` — search for capabilities
  - [x] `vault sync <agent>` — sync capabilities to an agent config
  - [x] `vault status` — show vault health and summary
  - [x] `vault config` — view/set configuration values
  - [x] `vault init` — initialize a new vault
  - [x] `vault doctor` — diagnose environment issues
  - [x] `vault connector add <agent>` — register an agent connector
  - [x] `vault connector list` — list registered connectors
  - [x] `vault connector remove <agent>` — unregister an agent connector
  - [x] `vault export` — export current state to `vault.toml`
  - [x] `vault import <path>` — import capabilities from a `vault.toml`
- [x] Verify `cargo build --workspace` succeeds with zero errors
- [x] Verify `cargo clippy --workspace` passes with no warnings
- [x] Verify `cargo fmt --all -- --check` passes
- [x] Set up GitHub Actions CI workflow (`.github/workflows/ci.yml`):
  - [x] Matrix: stable + nightly Rust, Ubuntu + macOS
  - [x] Steps: checkout, cache, build, clippy, fmt check, test

---

## Phase 1: Storage Foundation

- [x] Define `VaultConfig` struct and implement `config.toml` parsing with serde
  - [x] Fields: `vault_dir`, `default_agent`, `sync_on_install`, `log_level`
  - [x] Default config generation on first run
  - [x] Config file location: `~/.agentvault/config.toml`
- [x] Design and implement SQLite schema with migrations:
  - [x] `mcps` table (id, name, version, source, transport, config_json, installed_at, updated_at, checksum)
  - [x] `skills` table (id, name, version, source, path, installed_at, updated_at)
  - [x] `workflows` table (id, name, version, source, definition_json, installed_at, updated_at)
  - [x] `capabilities` table (id, kind, ref_id, tags, description) — unified capability view
  - [x] `agent_configs` table (id, agent_type, config_path, last_synced, enabled)
  - [x] `sync_history` table (id, agent_type, action, diff_json, synced_at, success)
- [x] Implement `Registry` trait with CRUD operations:
  - [x] `insert_mcp()`, `get_mcp()`, `list_mcps()`, `update_mcp()`, `delete_mcp()`
  - [x] Equivalent methods for skills and workflows
  - [x] `search()` with fuzzy name matching
- [x] Implement `SqliteRegistry` struct (concrete implementation of `Registry`)
  - [x] Connection management with `rusqlite`
  - [x] Schema initialization on first open
  - [x] Transaction support for multi-step operations
- [x] Implement filesystem store (`~/.agentvault/` directory structure):
  - [x] `~/.agentvault/config.toml`
  - [x] `~/.agentvault/vault.db`
  - [x] `~/.agentvault/mcps/` — installed MCP server files
  - [x] `~/.agentvault/skills/` — installed skill directories
  - [x] `~/.agentvault/workflows/` — installed workflow definitions
  - [x] `~/.agentvault/backups/` — agent config backups
  - [x] `~/.agentvault/logs/` — sync and operation logs
- [x] Implement `vault init` command:
  - [x] Create directory structure
  - [x] Initialize SQLite database with schema
  - [x] Generate default `config.toml`
  - [x] Print success summary with directory layout
- [x] Implement `vault status` command:
  - [x] Show vault directory location
  - [x] Show counts: installed MCPs, skills, workflows
  - [x] Show registered agent connectors and last sync time
  - [x] Show database size and integrity check result
- [x] Implement `vault doctor` command:
  - [x] Check vault directory exists and is writable
  - [x] Check SQLite database integrity (`PRAGMA integrity_check`)
  - [x] Check for orphaned files (on disk but not in DB)
  - [x] Check for missing files (in DB but not on disk)
  - [x] Detect `npm`/`npx` availability and version
  - [x] Detect `uv`/`pip` availability and version
  - [x] Detect `git` availability and version
  - [x] Report all findings with pass/warn/fail indicators
- [x] Unit tests for all storage operations:
  - [x] Config parsing: valid, invalid, missing fields, defaults
  - [x] SQLite CRUD: insert, get, list, update, delete for each table
  - [x] Filesystem store: creation, listing, cleanup
  - [x] `vault init` idempotency (running twice doesn't corrupt)

---

## Phase 2: MCP Management Core

- [ ] Define MCP data models in `vault-core/src/models/mcp.rs`:
  - [ ] `McpEntry` — full installed MCP record (name, version, source, transport, config, env_vars, installed_at)
  - [ ] `McpSource` enum — `Npm { package }`, `PyPi { package }`, `GitHub { repo, ref }`, `Local { path }`
  - [ ] `McpTransport` enum — `Stdio { command, args }`, `Sse { url }`, `StreamableHttp { url }`
  - [ ] `McpConfig` — per-MCP configuration (env vars, args, transport settings)
- [ ] Implement `McpManager` trait:
  - [ ] `install(source: McpSource) -> Result<McpEntry>`
  - [ ] `remove(name: &str) -> Result<()>`
  - [ ] `update(name: &str) -> Result<McpEntry>`
  - [ ] `get(name: &str) -> Result<McpEntry>`
  - [ ] `list() -> Result<Vec<McpEntry>>`
- [ ] Implement `DefaultMcpManager` struct (concrete implementation)
- [ ] MCP install from npm:
  - [ ] Detect `npm` or `npx` on PATH
  - [ ] Run `npm install --prefix <vault_dir>/mcps/<name>` or equivalent
  - [ ] Parse `package.json` for binary entry point
  - [ ] Build `McpTransport::Stdio` with correct command and args
  - [ ] Register in SQLite via `Registry`
  - [ ] Compute and store SHA-256 checksum
- [ ] MCP install from PyPI:
  - [ ] Detect `uv` (preferred) or `pip` on PATH
  - [ ] Create isolated venv in `<vault_dir>/mcps/<name>/`
  - [ ] Run `uv pip install <package>` or `pip install <package>`
  - [ ] Detect entry point (console_scripts or module invocation)
  - [ ] Build `McpTransport::Stdio` with correct command and args
  - [ ] Register in SQLite via `Registry`
- [ ] MCP install from GitHub:
  - [ ] Run `git clone <repo> <vault_dir>/mcps/<name>`
  - [ ] Detect build system (`package.json` → npm, `Cargo.toml` → cargo, `pyproject.toml` → uv/pip)
  - [ ] Run appropriate build command
  - [ ] Detect entry point from build output
  - [ ] Build `McpTransport::Stdio` with correct command and args
  - [ ] Register in SQLite via `Registry`
- [ ] MCP install from local path:
  - [ ] Validate path exists and contains a runnable MCP server
  - [ ] Create symlink from `<vault_dir>/mcps/<name>` → local path
  - [ ] Detect transport from local config or prompt user
  - [ ] Register in SQLite via `Registry`
- [ ] Implement `vault install` command:
  - [ ] Auto-detect source type from input string (npm:, pip:, github:, local path, bare name)
  - [ ] Show progress spinner during install (`indicatif`)
  - [ ] Print success summary with installed version and transport info
  - [ ] Optionally trigger sync if `sync_on_install` is true in config
- [ ] Implement `vault remove` command:
  - [ ] Look up MCP in registry by name
  - [ ] Remove files from filesystem (`<vault_dir>/mcps/<name>`)
  - [ ] Delete record from SQLite
  - [ ] Print confirmation with removed capability details
- [ ] Implement `vault update` command:
  - [ ] Single update: `vault update <name>` — re-install latest version
  - [ ] Bulk update: `vault update --all` — iterate and update each installed MCP
  - [ ] Show before/after version comparison
  - [ ] Skip if already at latest version
- [ ] Implement `vault list` command:
  - [ ] `--mcps` flag: list only MCPs (default if no flags)
  - [ ] `--skills` flag: list only skills
  - [ ] `--workflows` flag: list only workflows
  - [ ] `--json` flag: output as JSON array
  - [ ] `--table` flag: output as formatted table (`tabled`)
  - [ ] Default: pretty table with name, version, source, transport, installed date
- [ ] Implement env var management per MCP:
  - [ ] `vault config set <mcp> <KEY> <VALUE>` — store env var for an MCP
  - [ ] `vault config get <mcp>` — retrieve env vars for an MCP
  - [ ] Mask secret values in terminal output (show `****` for API keys)
  - [ ] Store env vars encrypted or at minimum in a separate, permission-restricted file
- [ ] Implement version pinning and constraint checking:
  - [ ] Store pinned version in `McpEntry`
  - [ ] Support semver constraints (`^1.0`, `~1.2`, `>=1.0,<2.0`)
  - [ ] Check constraints before update; warn on breaking changes
  - [ ] `--force` flag to override version constraints
- [ ] Integration tests with temp directories:
  - [ ] Install from local path, verify registry entry and symlink
  - [ ] Remove installed MCP, verify cleanup
  - [ ] Update installed MCP, verify version change
  - [ ] List with various flag combinations
  - [ ] Version constraint enforcement

---

## Phase 3: Agent Connectors

- [ ] Define `AgentConnector` trait in `vault-connectors/src/traits.rs`:
  - [ ] `fn agent_type(&self) -> &str`
  - [ ] `fn config_path(&self) -> PathBuf`
  - [ ] `fn read_config(&self) -> Result<AgentConfig>`
  - [ ] `fn write_config(&self, config: &AgentConfig) -> Result<()>`
  - [ ] `fn sync(&self, mcps: &[McpEntry]) -> Result<SyncResult>`
  - [ ] `fn diff(&self, mcps: &[McpEntry]) -> Result<ConfigDiff>`
  - [ ] `fn backup(&self) -> Result<PathBuf>`
  - [ ] `fn verify(&self) -> Result<bool>`
- [ ] Implement Claude Code connector (`vault-connectors/src/claude.rs`):
  - [ ] Parse `~/.claude/claude_desktop_config.json`
  - [ ] Map `McpEntry` list → `"mcpServers": { "<name>": { "command": ..., "args": [...], "env": {...} } }` format
  - [ ] Preserve existing non-vault entries in config (merge, don't overwrite)
  - [ ] Backup existing config to `~/.agentvault/backups/claude/<timestamp>.json`
  - [ ] Write updated config atomically (write to temp file, then rename)
  - [ ] Verify written config is valid JSON
- [ ] Implement Gemini CLI connector (`vault-connectors/src/gemini.rs`):
  - [ ] Parse `~/.gemini/config/settings.json`
  - [ ] Map `McpEntry` list → Gemini CLI MCP configuration format
  - [ ] Preserve existing non-vault entries in config
  - [ ] Backup existing config to `~/.agentvault/backups/gemini/<timestamp>.json`
  - [ ] Write updated config atomically
  - [ ] Verify written config is valid JSON
- [ ] Implement OpenCode connector (`vault-connectors/src/opencode.rs`):
  - [ ] Research OpenCode MCP config file format and location
  - [ ] Implement `read_config` and `write_config`
  - [ ] Backup and atomic write
- [ ] Implement Codex CLI connector (`vault-connectors/src/codex.rs`):
  - [ ] Research Codex CLI MCP config file format and location
  - [ ] Implement `read_config` and `write_config`
  - [ ] Backup and atomic write
- [ ] Implement `vault sync <agent>` command:
  - [ ] Look up connector by agent name
  - [ ] Read current agent config
  - [ ] Compute diff between vault registry and agent config
  - [ ] Apply changes (add new MCPs, update changed MCPs, optionally remove unmanaged)
  - [ ] Log sync action to `sync_history` table
  - [ ] Print summary of changes applied
- [ ] Implement `vault sync --all` command:
  - [ ] Iterate over all registered connectors
  - [ ] Run sync for each, collecting results
  - [ ] Print summary table of all sync results
- [ ] Implement `vault sync --dry-run` flag:
  - [ ] Compute diff without writing any changes
  - [ ] Display diff in a readable format (added/removed/changed MCPs)
  - [ ] Support `--dry-run` with both single agent and `--all`
- [ ] Implement `vault connector add <agent>` command:
  - [ ] Accept agent type (claude, gemini, opencode, codex) and optional custom config path
  - [ ] Validate config file exists at expected or given path
  - [ ] Register connector in `agent_configs` table
  - [ ] Print confirmation
- [ ] Implement `vault connector list` command:
  - [ ] Query `agent_configs` table
  - [ ] Display table: agent type, config path, last synced, enabled status
- [ ] Implement `vault connector remove <agent>` command:
  - [ ] Remove connector from `agent_configs` table
  - [ ] Do NOT delete the agent's config file
  - [ ] Print confirmation
- [ ] Implement sync history logging to SQLite:
  - [ ] Log every sync action: agent, timestamp, action type, diff JSON, success/failure
  - [ ] `vault status` shows last sync per agent
- [ ] Integration tests with mock config files:
  - [ ] Claude connector: round-trip read → sync → read, verify MCP entries
  - [ ] Gemini connector: round-trip read → sync → read, verify MCP entries
  - [ ] Backup creation and content verification
  - [ ] Dry-run produces diff without file changes
  - [ ] Sync with empty vault removes vault-managed entries only
  - [ ] Sync preserves non-vault entries in agent configs

---

## Phase 4: Search & Discovery

- [ ] Implement local fuzzy search against registry:
  - [ ] Search by name (fuzzy substring match)
  - [ ] Search by tag
  - [ ] Search by description keyword
  - [ ] Rank results by relevance
- [ ] Implement npm registry search via API:
  - [ ] Query `https://registry.npmjs.org/-/v1/search?text=<query>&size=20`
  - [ ] Parse response, extract name, version, description, keywords
  - [ ] Filter results relevant to MCP servers (keyword heuristics)
- [ ] Implement `vault search` command:
  - [ ] Default: search local registry first, then npm
  - [ ] `--local` flag: search only local registry
  - [ ] `--npm` flag: search only npm registry
  - [ ] `--limit <n>` flag: limit number of results
  - [ ] Show source indicator (local ✓ / npm ↓) for each result
- [ ] Rich terminal output:
  - [ ] Formatted tables with `tabled` for list/search results
  - [ ] Colored status indicators: green (installed), yellow (update available), dim (not installed)
  - [ ] Progress spinners for network requests (`indicatif`)
  - [ ] Dimmed metadata (install date, source) for scannability
- [ ] Tests:
  - [ ] Local fuzzy search: exact match, partial match, no match, tag match
  - [ ] npm search response parsing (mock HTTP responses)
  - [ ] Output formatting with various result counts

---

## Phase 5: Manifest & Declarative Config

- [ ] Define `vault.toml` manifest format specification:
  - [ ] `[vault]` section: name, version, description
  - [ ] `[[mcp]]` entries: name, source, version constraint, env vars, transport overrides
  - [ ] `[[skill]]` entries: name, source, version constraint
  - [ ] `[[workflow]]` entries: name, source, version constraint
  - [ ] `[agents]` section: list of agent connectors to sync
- [ ] Implement `VaultManifest` struct and parser:
  - [ ] Deserialize `vault.toml` with serde
  - [ ] Validate all required fields present
  - [ ] Validate semver constraints are parseable
  - [ ] Validate source strings are well-formed
- [ ] Implement `vault export` command:
  - [ ] Read current vault state from SQLite
  - [ ] Serialize to `vault.toml` format
  - [ ] Write to `./vault.toml` (current directory) or `--output <path>`
  - [ ] Print summary of exported capabilities
- [ ] Implement `vault import` command:
  - [ ] Parse `vault.toml` from given path (default: `./vault.toml`)
  - [ ] Diff declared capabilities against current vault state
  - [ ] Install missing capabilities
  - [ ] Update capabilities where version constraints differ
  - [ ] Optionally remove capabilities not in manifest (`--prune` flag)
  - [ ] Show progress for each install/update operation
  - [ ] Print summary of changes applied
- [ ] Implement diff between manifest and current state:
  - [ ] `vault import --dry-run` to preview changes
  - [ ] Categorize: to install, to update, to remove (if `--prune`), unchanged
  - [ ] Display diff in readable table format
- [ ] Tests:
  - [ ] Round-trip: export → import on empty vault → verify state matches
  - [ ] Import with `--prune`: removes unlisted capabilities
  - [ ] Import idempotency: running twice produces no changes
  - [ ] Manifest validation: reject malformed manifests with clear errors
  - [ ] Partial failure: some installs succeed, some fail — verify rollback/report

---

## Phase 6: Skills & Workflows

- [ ] Define skill data models in `vault-core/src/models/skill.rs`:
  - [ ] `SkillEntry` — name, version, source, path, tags, description, installed_at
  - [ ] `SkillSource` enum — `Git { repo, ref }`, `Local { path }`
- [ ] Implement `SkillManager` trait and default implementation:
  - [ ] `install(source: SkillSource) -> Result<SkillEntry>`
  - [ ] `remove(name: &str) -> Result<()>`
  - [ ] `get(name: &str) -> Result<SkillEntry>`
  - [ ] `list() -> Result<Vec<SkillEntry>>`
- [ ] Skill installation from git repo:
  - [ ] Clone into `~/.agentvault/skills/<name>/`
  - [ ] Detect and validate `SKILL.md` presence
  - [ ] Parse skill metadata from YAML frontmatter
  - [ ] Register in SQLite
- [ ] Skill installation from local path:
  - [ ] Validate `SKILL.md` exists
  - [ ] Symlink into `~/.agentvault/skills/<name>/`
  - [ ] Register in SQLite
- [ ] Skill listing and search:
  - [ ] Extend `vault list --skills` to show skill-specific columns
  - [ ] Extend `vault search` to include skills in results
- [ ] Define workflow data models in `vault-core/src/models/workflow.rs`:
  - [ ] `WorkflowEntry` — name, version, source, steps, dependencies, installed_at
  - [ ] `WorkflowStep` — name, capability_ref, args, condition
- [ ] Implement `WorkflowManager` trait and default implementation:
  - [ ] `install(source: WorkflowSource) -> Result<WorkflowEntry>`
  - [ ] `remove(name: &str) -> Result<()>`
  - [ ] `get(name: &str) -> Result<WorkflowEntry>`
  - [ ] `list() -> Result<Vec<WorkflowEntry>>`
  - [ ] `validate(name: &str) -> Result<Vec<ValidationIssue>>` — check all deps are installed
- [ ] Implement `workflow.toml` definition parsing:
  - [ ] `[workflow]` section: name, version, description
  - [ ] `[[step]]` entries: name, uses (capability ref), args, depends_on
  - [ ] Validate DAG (no circular dependencies)
- [ ] Implement dependency resolution for workflows:
  - [ ] Topological sort of workflow steps
  - [ ] Check all referenced capabilities are installed in vault
  - [ ] Report missing dependencies with install suggestions
- [ ] Extend `vault install` / `vault remove` / `vault list` for skills and workflows:
  - [ ] `vault install --skill <source>` flag
  - [ ] `vault install --workflow <source>` flag
  - [ ] `vault remove` auto-detects capability type by name
  - [ ] `vault list` supports `--skills`, `--workflows`, `--all` flags
- [ ] Tests:
  - [ ] Skill install from local path, verify SKILL.md parsed
  - [ ] Workflow install, verify steps and deps parsed
  - [ ] Dependency resolution: valid DAG, circular DAG (should error), missing deps
  - [ ] List and search across all capability types

---

## Phase 7: Polish & Release

- [ ] Error message polish:
  - [ ] Review all `VaultError` variants for user-friendly display messages
  - [ ] Add contextual suggestions (e.g., "Run `vault init` first" when vault dir missing)
  - [ ] Ensure no raw panic messages reach the user
  - [ ] Add `--verbose` flag to show full error chain with `tracing`
- [ ] Help text polish for all commands:
  - [ ] Add `long_about` to every clap command with examples
  - [ ] Add `after_help` with common usage patterns
  - [ ] Verify `vault --help` and `vault <cmd> --help` are clear and complete
- [ ] Shell completions via `clap_complete`:
  - [ ] Generate Bash completions
  - [ ] Generate Zsh completions
  - [ ] Generate Fish completions
  - [ ] Generate PowerShell completions
  - [ ] Add `vault completions <shell>` subcommand
- [ ] Man page generation:
  - [ ] Generate man pages from clap definitions (`clap_mangen`)
  - [ ] Include in release artifacts
- [ ] Performance optimizations:
  - [ ] Lazy-load SQLite connection (don't open DB for `--help`)
  - [ ] Connection pooling for repeated DB operations in bulk commands
  - [ ] Parallelize `vault update --all` with `tokio::spawn`
  - [ ] Profile and optimize hot paths (list, search)
- [ ] Full integration test suite:
  - [ ] End-to-end: init → install MCPs → sync to agents → export → import on fresh vault
  - [ ] Edge cases: empty vault operations, duplicate installs, concurrent syncs
  - [ ] Error paths: network failure, permission denied, corrupt DB
- [ ] Binary release workflow (`.github/workflows/release.yml`):
  - [ ] Cross-compile for Linux x86_64 and aarch64
  - [ ] Cross-compile for macOS x86_64 and aarch64 (universal binary)
  - [ ] Cross-compile for Windows x86_64
  - [ ] Use `cross` for cross-compilation
  - [ ] Create GitHub Release with changelog and binaries
  - [ ] Attach SHA-256 checksums for all binaries
- [ ] `README.md` with full usage docs:
  - [ ] Project overview and motivation
  - [ ] Installation instructions (binary, cargo install, from source)
  - [ ] Quick start guide
  - [ ] Full command reference
  - [ ] Configuration reference
  - [ ] Connector setup guide per agent
  - [ ] Manifest format reference
  - [ ] Contributing guide
- [ ] Install script (`install.sh`):
  - [ ] Detect OS and architecture
  - [ ] Download correct binary from GitHub Releases
  - [ ] Verify checksum
  - [ ] Install to `~/.local/bin` or `/usr/local/bin`
  - [ ] Print post-install instructions
- [ ] Tag `v0.1.0` release:
  - [ ] Update version in all `Cargo.toml` files
  - [ ] Write `CHANGELOG.md` for v0.1.0
  - [ ] Create git tag and push
  - [ ] Verify CI release workflow triggers and completes

---

## Backlog / Future Ideas

- [ ] TUI dashboard with `ratatui` (installed MCPs, sync status, live logs)
- [ ] File watcher for auto-sync (detect agent config changes, re-sync automatically)
- [ ] Remote community registry (publish/discover MCPs, skills, workflows)
- [ ] Plugin system for connectors (dynamic `.so`/`.dylib` loading or WASM plugins)
- [ ] Cursor connector (research config format, implement read/write/sync)
- [ ] RooCode connector (research config format, implement read/write/sync)
- [ ] Hermes connector (research config format, implement read/write/sync)
- [ ] Capability abstraction layer (unified interface across MCPs, skills, workflows)
- [ ] MCP health checking (`vault doctor --check-mcps` — ping each MCP server, verify it responds)
- [ ] Telemetry (opt-in, anonymous usage stats for registry popularity)

---

## Known Issues

_No known issues yet._

---

## Technical Debt

_No technical debt yet._

---

> **Last updated:** 2026-06-22
