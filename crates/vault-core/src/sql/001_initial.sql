-- Initial schema for AgentVault SQLite registry.

-- ============================================================
-- MCPs Table
-- Stores all installed MCP server metadata.
-- ============================================================
CREATE TABLE IF NOT EXISTS mcps (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID/UUID
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

CREATE INDEX IF NOT EXISTS idx_mcps_name ON mcps(name);
CREATE INDEX IF NOT EXISTS idx_mcps_source_type ON mcps(source_type);
CREATE INDEX IF NOT EXISTS idx_mcps_status ON mcps(status);

-- ============================================================
-- Skills Table
-- Stores installed skill metadata.
-- ============================================================
CREATE TABLE IF NOT EXISTS skills (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID/UUID
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

CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

-- ============================================================
-- Workflows Table
-- Stores workflow definitions.
-- ============================================================
CREATE TABLE IF NOT EXISTS workflows (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID/UUID
    name            TEXT    NOT NULL UNIQUE,          -- machine-friendly name
    description     TEXT,                             -- human-readable description
    steps_json      TEXT    NOT NULL DEFAULT '[]',    -- JSON array of WorkflowStep objects
    deps_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of dependency names
    installed_at    TEXT    NOT NULL                  -- ISO 8601 timestamp
);

CREATE INDEX IF NOT EXISTS idx_workflows_name ON workflows(name);

-- ============================================================
-- Capabilities Table
-- Higher-level bundles of MCPs + skills + workflows.
-- ============================================================
CREATE TABLE IF NOT EXISTS capabilities (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID/UUID
    name            TEXT    NOT NULL UNIQUE,          -- capability name
    description     TEXT,                             -- human-readable description
    mcps_json       TEXT    NOT NULL DEFAULT '[]',    -- JSON array of required MCP names
    skills_json     TEXT    NOT NULL DEFAULT '[]',    -- JSON array of required skill names
    workflows_json  TEXT    NOT NULL DEFAULT '[]'     -- JSON array of required workflow names
);

CREATE INDEX IF NOT EXISTS idx_capabilities_name ON capabilities(name);

-- ============================================================
-- Agent Configs Table
-- Registered agent connectors and their sync state.
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_configs (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID/UUID
    agent_type      TEXT    NOT NULL UNIQUE,          -- 'claude', 'gemini', 'opencode', etc.
    config_path     TEXT    NOT NULL,                 -- absolute path to agent's config file
    enabled         INTEGER NOT NULL DEFAULT 1,      -- 1=enabled, 0=disabled
    last_synced     TEXT,                             -- ISO 8601 timestamp or NULL
    auto_sync       INTEGER NOT NULL DEFAULT 0       -- 1=auto-sync on install, 0=manual
);

CREATE INDEX IF NOT EXISTS idx_agent_configs_type ON agent_configs(agent_type);

-- ============================================================
-- Sync History Table
-- Audit log of all sync operations.
-- ============================================================
CREATE TABLE IF NOT EXISTS sync_history (
    id              TEXT    PRIMARY KEY NOT NULL,    -- ULID/UUID
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

CREATE INDEX IF NOT EXISTS idx_sync_history_agent ON sync_history(agent_type);
CREATE INDEX IF NOT EXISTS idx_sync_history_timestamp ON sync_history(timestamp);

-- ============================================================
-- Migrations Table
-- Tracks applied schema migrations for forward-compatibility.
-- ============================================================
CREATE TABLE IF NOT EXISTS migrations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    version         INTEGER NOT NULL UNIQUE,          -- schema version
    description     TEXT,                             -- migration description
    applied_at      TEXT    NOT NULL                  -- ISO 8601 timestamp
);
