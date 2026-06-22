use crate::agent::{AgentConnectorConfig, AgentType, SyncHistoryEntry};
use crate::capability::models::{CapabilityKind, CapabilityRecord};
use crate::error::VaultError;
use crate::mcp::models::{McpEntry, McpSource, McpStatus, McpTransport};
use crate::skill::models::{SkillEntry, SkillSource};
use crate::workflow::models::{WorkflowEntry, WorkflowStep};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MIGRATIONS: &[(i64, &str, &str)] =
    &[(1, "Initial schema", include_str!("sql/001_initial.sql"))];

/// The persistence interface for the capability registry.
pub trait Registry: Send + Sync {
    // MCP operations
    fn insert_mcp(&self, entry: &McpEntry) -> Result<(), VaultError>;
    fn get_mcp(&self, name: &str) -> Result<McpEntry, VaultError>;
    fn list_mcps(&self) -> Result<Vec<McpEntry>, VaultError>;
    fn update_mcp(&self, entry: &McpEntry) -> Result<(), VaultError>;
    fn delete_mcp(&self, name: &str) -> Result<(), VaultError>;

    // Skill operations
    fn insert_skill(&self, entry: &SkillEntry) -> Result<(), VaultError>;
    fn get_skill(&self, name: &str) -> Result<SkillEntry, VaultError>;
    fn list_skills(&self) -> Result<Vec<SkillEntry>, VaultError>;
    fn update_skill(&self, entry: &SkillEntry) -> Result<(), VaultError>;
    fn delete_skill(&self, name: &str) -> Result<(), VaultError>;

    // Workflow operations
    fn insert_workflow(&self, entry: &WorkflowEntry) -> Result<(), VaultError>;
    fn get_workflow(&self, name: &str) -> Result<WorkflowEntry, VaultError>;
    fn list_workflows(&self) -> Result<Vec<WorkflowEntry>, VaultError>;
    fn update_workflow(&self, entry: &WorkflowEntry) -> Result<(), VaultError>;
    fn delete_workflow(&self, name: &str) -> Result<(), VaultError>;

    // Agent Config operations
    fn insert_agent_config(&self, config: &AgentConnectorConfig) -> Result<(), VaultError>;
    fn get_agent_config(&self, agent_type: &str) -> Result<AgentConnectorConfig, VaultError>;
    fn list_agent_configs(&self) -> Result<Vec<AgentConnectorConfig>, VaultError>;
    fn delete_agent_config(&self, agent_type: &str) -> Result<(), VaultError>;

    // Sync History operations
    fn log_sync(&self, entry: &SyncHistoryEntry) -> Result<(), VaultError>;
    fn get_sync_history(
        &self,
        agent_type: &str,
        limit: usize,
    ) -> Result<Vec<SyncHistoryEntry>, VaultError>;

    // Unified Search operations
    fn search(&self, query: &str) -> Result<Vec<CapabilityRecord>, VaultError>;
}

/// A SQLite-backed implementation of the Registry trait.
pub struct SqliteRegistry {
    conn: Mutex<Connection>,
}

impl SqliteRegistry {
    /// Opens the registry database and runs pending migrations.
    pub fn new(db_path: &Path) -> Result<Self, VaultError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;

        // Set PRAGMAs outside of transactions using dedicated pragma_update
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        // Execute migrations
        run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

fn run_migrations(conn: &Connection) -> Result<(), VaultError> {
    // Ensure migrations table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS migrations (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            version         INTEGER NOT NULL UNIQUE,
            description     TEXT,
            applied_at      TEXT NOT NULL
        );",
        [],
    )?;

    // Get current version
    let mut stmt = conn.prepare("SELECT MAX(version) FROM migrations")?;
    let max_version: Option<i64> = stmt.query_row([], |row| row.get(0)).unwrap_or(None);
    let current_version = max_version.unwrap_or(0);

    for &(version, description, sql) in MIGRATIONS {
        if version > current_version {
            // Apply migration inside a transaction
            // We use unchecked_transaction here to bypass standard lifetime issues in this helper
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO migrations (version, description, applied_at) VALUES (?1, ?2, datetime('now'))",
                params![version, description],
            )?;
            tx.commit()?;
        }
    }
    Ok(())
}

// Helpers for row parsing
fn parse_datetime(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

fn map_mcp_row(row: &Row) -> rusqlite::Result<McpEntry> {
    let source_str: String = row.get("source_type")?;
    let source_val: String = row.get("source_value")?;
    let source_ref: Option<String> = row.get("source_ref")?;
    let source = match source_str.as_str() {
        "npm" => McpSource::Npm {
            package: source_val,
        },
        "pypi" => McpSource::PyPi {
            package: source_val,
        },
        "github" => McpSource::GitHub {
            repo: source_val,
            ref_: source_ref,
        },
        "local" => McpSource::Local {
            path: PathBuf::from(source_val),
        },
        "docker" => McpSource::Docker { image: source_val },
        _ => McpSource::Npm {
            package: source_val,
        }, // fallback
    };

    let transport_str: String = row.get("transport")?;
    let transport_url: Option<String> = row.get("transport_url")?;
    let transport = match transport_str.as_str() {
        "stdio" => McpTransport::Stdio,
        "sse" => McpTransport::Sse {
            url: transport_url.unwrap_or_default(),
        },
        "http" => McpTransport::StreamableHttp {
            url: transport_url.unwrap_or_default(),
        },
        _ => McpTransport::Stdio,
    };

    let status_str: String = row.get("status")?;
    let status_err: Option<String> = row.get("status_error")?;
    let status = match status_str.as_str() {
        "active" => McpStatus::Active,
        "disabled" => McpStatus::Disabled,
        "error" => McpStatus::Error {
            message: status_err.unwrap_or_default(),
        },
        _ => McpStatus::Active,
    };

    let args_json: String = row.get("args_json")?;
    let args: Vec<String> = serde_json::from_str(&args_json).unwrap_or_default();

    let env_json: String = row.get("env_json")?;
    let env_vars: HashMap<String, String> = serde_json::from_str(&env_json).unwrap_or_default();

    let agents_json: String = row.get("agents_json")?;
    let agents: Vec<String> = serde_json::from_str(&agents_json).unwrap_or_default();

    let tags_json: String = row.get("tags_json")?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

    let installed_at_str: String = row.get("installed_at")?;
    let updated_at_str: String = row.get("updated_at")?;

    Ok(McpEntry {
        id: row.get("id")?,
        name: row.get("name")?,
        display_name: row.get("display_name")?,
        version: row.get("version")?,
        source,
        install_path: PathBuf::from(row.get::<_, String>("install_path")?),
        command: row.get("command")?,
        args,
        env_vars,
        transport,
        status,
        installed_at: parse_datetime(&installed_at_str),
        updated_at: parse_datetime(&updated_at_str),
        checksum: row.get("checksum")?,
        agents,
        tags,
        description: row.get("description")?,
    })
}

impl Registry for SqliteRegistry {
    // ─── MCP Operations ──────────────────────────────────────────────

    fn insert_mcp(&self, entry: &McpEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let (source_type, source_value, source_ref) = match &entry.source {
            McpSource::Npm { package } => ("npm", package.clone(), None),
            McpSource::PyPi { package } => ("pypi", package.clone(), None),
            McpSource::GitHub { repo, ref_ } => ("github", repo.clone(), ref_.clone()),
            McpSource::Local { path } => ("local", path.display().to_string(), None),
            McpSource::Docker { image } => ("docker", image.clone(), None),
        };

        let (transport, transport_url) = match &entry.transport {
            McpTransport::Stdio => ("stdio", None),
            McpTransport::Sse { url } => ("sse", Some(url.clone())),
            McpTransport::StreamableHttp { url } => ("http", Some(url.clone())),
        };

        let (status, status_error) = match &entry.status {
            McpStatus::Active => ("active", None),
            McpStatus::Disabled => ("disabled", None),
            McpStatus::Error { message } => ("error", Some(message.clone())),
        };

        let args_json = serde_json::to_string(&entry.args)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let env_json = serde_json::to_string(&entry.env_vars)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let agents_json = serde_json::to_string(&entry.agents)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        // Check if already exists
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM mcps WHERE name = ?1)",
                params![entry.name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Err(VaultError::AlreadyExists {
                kind: "mcp".to_string(),
                name: entry.name.clone(),
            });
        }

        conn.execute(
            "INSERT INTO mcps (
                id, name, display_name, version, source_type, source_value, source_ref,
                install_path, command, args_json, env_json, transport, transport_url,
                status, status_error, checksum, agents_json, tags_json, description,
                installed_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                entry.id, entry.name, entry.display_name, entry.version, source_type, source_value, source_ref,
                entry.install_path.to_string_lossy(), entry.command, args_json, env_json, transport, transport_url,
                status, status_error, entry.checksum, agents_json, tags_json, entry.description,
                entry.installed_at.to_rfc3339(), entry.updated_at.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    fn get_mcp(&self, name: &str) -> Result<McpEntry, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM mcps WHERE name = ?1")?;
        let entry = stmt
            .query_row(params![name], map_mcp_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => VaultError::NotFound {
                    kind: "mcp".to_string(),
                    name: name.to_string(),
                },
                other => VaultError::Database(other),
            })?;
        Ok(entry)
    }

    fn list_mcps(&self) -> Result<Vec<McpEntry>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM mcps ORDER BY name ASC")?;
        let rows = stmt.query_map([], map_mcp_row)?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn update_mcp(&self, entry: &McpEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let (source_type, source_value, source_ref) = match &entry.source {
            McpSource::Npm { package } => ("npm", package.clone(), None),
            McpSource::PyPi { package } => ("pypi", package.clone(), None),
            McpSource::GitHub { repo, ref_ } => ("github", repo.clone(), ref_.clone()),
            McpSource::Local { path } => ("local", path.display().to_string(), None),
            McpSource::Docker { image } => ("docker", image.clone(), None),
        };

        let (transport, transport_url) = match &entry.transport {
            McpTransport::Stdio => ("stdio", None),
            McpTransport::Sse { url } => ("sse", Some(url.clone())),
            McpTransport::StreamableHttp { url } => ("http", Some(url.clone())),
        };

        let (status, status_error) = match &entry.status {
            McpStatus::Active => ("active", None),
            McpStatus::Disabled => ("disabled", None),
            McpStatus::Error { message } => ("error", Some(message.clone())),
        };

        let args_json = serde_json::to_string(&entry.args)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let env_json = serde_json::to_string(&entry.env_vars)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let agents_json = serde_json::to_string(&entry.agents)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        let affected = conn.execute(
            "UPDATE mcps SET
                display_name = ?2, version = ?3, source_type = ?4, source_value = ?5, source_ref = ?6,
                install_path = ?7, command = ?8, args_json = ?9, env_json = ?10, transport = ?11, transport_url = ?12,
                status = ?13, status_error = ?14, checksum = ?15, agents_json = ?16, tags_json = ?17, description = ?18,
                updated_at = ?19
            WHERE name = ?1",
            params![
                entry.name, entry.display_name, entry.version, source_type, source_value, source_ref,
                entry.install_path.to_string_lossy(), entry.command, args_json, env_json, transport, transport_url,
                status, status_error, entry.checksum, agents_json, tags_json, entry.description,
                entry.updated_at.to_rfc3339()
            ],
        )?;

        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "mcp".to_string(),
                name: entry.name.clone(),
            });
        }

        Ok(())
    }

    fn delete_mcp(&self, name: &str) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM mcps WHERE name = ?1", params![name])?;
        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "mcp".to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    // ─── Skill Operations ────────────────────────────────────────────

    fn insert_skill(&self, entry: &SkillEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let (source_type, source_value, source_ref, source_subdir) = match &entry.source {
            SkillSource::Git {
                repo,
                ref_,
                subdirectory,
            } => ("git", repo.clone(), ref_.clone(), subdirectory.clone()),
            SkillSource::Local { path } => ("local", path.display().to_string(), None, None),
        };

        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let agents_json = serde_json::to_string(&entry.agents)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        // Check if already exists
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM skills WHERE name = ?1)",
                params![entry.name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Err(VaultError::AlreadyExists {
                kind: "skill".to_string(),
                name: entry.name.clone(),
            });
        }

        conn.execute(
            "INSERT INTO skills (
                id, name, description, path, tags_json, source_type, source_value,
                source_ref, source_subdir, agents_json, installed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                entry.id,
                entry.name,
                entry.description,
                entry.path.to_string_lossy(),
                tags_json,
                source_type,
                source_value,
                source_ref,
                source_subdir,
                agents_json,
                entry.installed_at.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    fn get_skill(&self, name: &str) -> Result<SkillEntry, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM skills WHERE name = ?1")?;
        let entry = stmt
            .query_row(params![name], |row| {
                let source_str: String = row.get("source_type")?;
                let source_val: String = row.get("source_value")?;
                let source_ref: Option<String> = row.get("source_ref")?;
                let source_subdir: Option<String> = row.get("source_subdir")?;
                let source = match source_str.as_str() {
                    "git" => SkillSource::Git {
                        repo: source_val,
                        ref_: source_ref,
                        subdirectory: source_subdir,
                    },
                    "local" => SkillSource::Local {
                        path: PathBuf::from(source_val),
                    },
                    _ => SkillSource::Local {
                        path: PathBuf::from(source_val),
                    },
                };

                let tags_json: String = row.get("tags_json")?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                let agents_json: String = row.get("agents_json")?;
                let agents: Vec<String> = serde_json::from_str(&agents_json).unwrap_or_default();

                let installed_at_str: String = row.get("installed_at")?;

                Ok(SkillEntry {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    description: row.get("description")?,
                    path: PathBuf::from(row.get::<_, String>("path")?),
                    tags,
                    source,
                    installed_at: parse_datetime(&installed_at_str),
                    agents,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => VaultError::NotFound {
                    kind: "skill".to_string(),
                    name: name.to_string(),
                },
                other => VaultError::Database(other),
            })?;
        Ok(entry)
    }

    fn list_skills(&self) -> Result<Vec<SkillEntry>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM skills ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            let source_str: String = row.get("source_type")?;
            let source_val: String = row.get("source_value")?;
            let source_ref: Option<String> = row.get("source_ref")?;
            let source_subdir: Option<String> = row.get("source_subdir")?;
            let source = match source_str.as_str() {
                "git" => SkillSource::Git {
                    repo: source_val,
                    ref_: source_ref,
                    subdirectory: source_subdir,
                },
                "local" => SkillSource::Local {
                    path: PathBuf::from(source_val),
                },
                _ => SkillSource::Local {
                    path: PathBuf::from(source_val),
                },
            };

            let tags_json: String = row.get("tags_json")?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            let agents_json: String = row.get("agents_json")?;
            let agents: Vec<String> = serde_json::from_str(&agents_json).unwrap_or_default();

            let installed_at_str: String = row.get("installed_at")?;

            Ok(SkillEntry {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
                path: PathBuf::from(row.get::<_, String>("path")?),
                tags,
                source,
                installed_at: parse_datetime(&installed_at_str),
                agents,
            })
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn update_skill(&self, entry: &SkillEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let (source_type, source_value, source_ref, source_subdir) = match &entry.source {
            SkillSource::Git {
                repo,
                ref_,
                subdirectory,
            } => ("git", repo.clone(), ref_.clone(), subdirectory.clone()),
            SkillSource::Local { path } => ("local", path.display().to_string(), None, None),
        };

        let tags_json = serde_json::to_string(&entry.tags)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let agents_json = serde_json::to_string(&entry.agents)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        let affected = conn.execute(
            "UPDATE skills SET
                description = ?2, path = ?3, tags_json = ?4, source_type = ?5, source_value = ?6,
                source_ref = ?7, source_subdir = ?8, agents_json = ?9
            WHERE name = ?1",
            params![
                entry.name,
                entry.description,
                entry.path.to_string_lossy(),
                tags_json,
                source_type,
                source_value,
                source_ref,
                source_subdir,
                agents_json
            ],
        )?;

        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "skill".to_string(),
                name: entry.name.clone(),
            });
        }
        Ok(())
    }

    fn delete_skill(&self, name: &str) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM skills WHERE name = ?1", params![name])?;
        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "skill".to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    // ─── Workflow Operations ─────────────────────────────────────────

    fn insert_workflow(&self, entry: &WorkflowEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let steps_json = serde_json::to_string(&entry.steps)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let deps_json = serde_json::to_string(&entry.dependencies)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        // Check if already exists
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workflows WHERE name = ?1)",
                params![entry.name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Err(VaultError::AlreadyExists {
                kind: "workflow".to_string(),
                name: entry.name.clone(),
            });
        }

        conn.execute(
            "INSERT INTO workflows (
                id, name, description, steps_json, deps_json, installed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.id,
                entry.name,
                entry.description,
                steps_json,
                deps_json,
                entry.installed_at.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    fn get_workflow(&self, name: &str) -> Result<WorkflowEntry, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM workflows WHERE name = ?1")?;
        let entry = stmt
            .query_row(params![name], |row| {
                let steps_json: String = row.get("steps_json")?;
                let steps: Vec<WorkflowStep> =
                    serde_json::from_str(&steps_json).unwrap_or_default();

                let deps_json: String = row.get("deps_json")?;
                let dependencies: Vec<String> =
                    serde_json::from_str(&deps_json).unwrap_or_default();

                let installed_at_str: String = row.get("installed_at")?;

                Ok(WorkflowEntry {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    description: row.get("description")?,
                    steps,
                    dependencies,
                    installed_at: parse_datetime(&installed_at_str),
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => VaultError::NotFound {
                    kind: "workflow".to_string(),
                    name: name.to_string(),
                },
                other => VaultError::Database(other),
            })?;
        Ok(entry)
    }

    fn list_workflows(&self) -> Result<Vec<WorkflowEntry>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM workflows ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            let steps_json: String = row.get("steps_json")?;
            let steps: Vec<WorkflowStep> = serde_json::from_str(&steps_json).unwrap_or_default();

            let deps_json: String = row.get("deps_json")?;
            let dependencies: Vec<String> = serde_json::from_str(&deps_json).unwrap_or_default();

            let installed_at_str: String = row.get("installed_at")?;

            Ok(WorkflowEntry {
                id: row.get("id")?,
                name: row.get("name")?,
                description: row.get("description")?,
                steps,
                dependencies,
                installed_at: parse_datetime(&installed_at_str),
            })
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn update_workflow(&self, entry: &WorkflowEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let steps_json = serde_json::to_string(&entry.steps)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let deps_json = serde_json::to_string(&entry.dependencies)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        let affected = conn.execute(
            "UPDATE workflows SET
                description = ?2, steps_json = ?3, deps_json = ?4
            WHERE name = ?1",
            params![entry.name, entry.description, steps_json, deps_json],
        )?;

        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "workflow".to_string(),
                name: entry.name.clone(),
            });
        }
        Ok(())
    }

    fn delete_workflow(&self, name: &str) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM workflows WHERE name = ?1", params![name])?;
        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "workflow".to_string(),
                name: name.to_string(),
            });
        }
        Ok(())
    }

    // ─── Agent Connector Config Operations ───────────────────────────

    fn insert_agent_config(&self, config: &AgentConnectorConfig) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let last_synced_str = config.last_synced.map(|dt| dt.to_rfc3339());

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM agent_configs WHERE agent_type = ?1)",
                params![config.agent_type.to_string()],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if exists {
            return Err(VaultError::AlreadyExists {
                kind: "agent_config".to_string(),
                name: config.agent_type.to_string(),
            });
        }

        conn.execute(
            "INSERT INTO agent_configs (
                id, agent_type, config_path, enabled, last_synced, auto_sync
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                config.id,
                config.agent_type.to_string(),
                config.config_path.to_string_lossy(),
                if config.enabled { 1 } else { 0 },
                last_synced_str,
                if config.auto_sync { 1 } else { 0 }
            ],
        )?;

        Ok(())
    }

    fn get_agent_config(&self, agent_type: &str) -> Result<AgentConnectorConfig, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM agent_configs WHERE agent_type = ?1")?;
        let entry = stmt
            .query_row(params![agent_type], |row| {
                let last_synced_str: Option<String> = row.get("last_synced")?;
                let last_synced = last_synced_str.map(|s| parse_datetime(&s));

                let agent_str: String = row.get("agent_type")?;
                let agent_type = agent_str
                    .parse::<AgentType>()
                    .unwrap_or(AgentType::Custom(agent_str));

                Ok(AgentConnectorConfig {
                    id: row.get("id")?,
                    agent_type,
                    config_path: PathBuf::from(row.get::<_, String>("config_path")?),
                    enabled: row.get::<_, i32>("enabled")? == 1,
                    last_synced,
                    auto_sync: row.get::<_, i32>("auto_sync")? == 1,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => VaultError::NotFound {
                    kind: "agent_config".to_string(),
                    name: agent_type.to_string(),
                },
                other => VaultError::Database(other),
            })?;
        Ok(entry)
    }

    fn list_agent_configs(&self) -> Result<Vec<AgentConnectorConfig>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM agent_configs ORDER BY agent_type ASC")?;
        let rows = stmt.query_map([], |row| {
            let last_synced_str: Option<String> = row.get("last_synced")?;
            let last_synced = last_synced_str.map(|s| parse_datetime(&s));

            let agent_str: String = row.get("agent_type")?;
            let agent_type = agent_str
                .parse::<AgentType>()
                .unwrap_or(AgentType::Custom(agent_str));

            Ok(AgentConnectorConfig {
                id: row.get("id")?,
                agent_type,
                config_path: PathBuf::from(row.get::<_, String>("config_path")?),
                enabled: row.get::<_, i32>("enabled")? == 1,
                last_synced,
                auto_sync: row.get::<_, i32>("auto_sync")? == 1,
            })
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn delete_agent_config(&self, agent_type: &str) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM agent_configs WHERE agent_type = ?1",
            params![agent_type],
        )?;
        if affected == 0 {
            return Err(VaultError::NotFound {
                kind: "agent_config".to_string(),
                name: agent_type.to_string(),
            });
        }
        Ok(())
    }

    // ─── Sync History Operations ─────────────────────────────────────

    fn log_sync(&self, entry: &SyncHistoryEntry) -> Result<(), VaultError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_history (
                id, agent_type, timestamp, action, changes_json, status, error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.agent_type,
                entry.synced_at.to_rfc3339(),
                entry.action,
                entry.diff_json,
                if entry.success { "success" } else { "failure" },
                entry.error
            ],
        )?;
        Ok(())
    }

    fn get_sync_history(
        &self,
        agent_type: &str,
        limit: usize,
    ) -> Result<Vec<SyncHistoryEntry>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM sync_history WHERE agent_type = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![agent_type, limit], |row| {
            let timestamp_str: String = row.get("timestamp")?;
            let status_str: String = row.get("status")?;
            Ok(SyncHistoryEntry {
                id: row.get("id")?,
                agent_type: row.get("agent_type")?,
                action: row.get("action")?,
                diff_json: row.get("changes_json")?,
                synced_at: parse_datetime(&timestamp_str),
                success: status_str == "success",
                error: row.get("error")?,
            })
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    // ─── Unified Search Operation ────────────────────────────────────

    fn search(&self, query: &str) -> Result<Vec<CapabilityRecord>, VaultError> {
        let conn = self.conn.lock().unwrap();
        let mut results = Vec::new();
        let like_query = format!("%{}%", query);

        // Search MCPs
        let mut stmt = conn.prepare(
            "SELECT id, name, description, tags_json FROM mcps 
             WHERE name LIKE ?1 OR description LIKE ?1 OR tags_json LIKE ?1",
        )?;
        let mcp_rows = stmt.query_map(params![like_query], |row| {
            let tags_json: String = row.get("tags_json")?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(CapabilityRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                kind: CapabilityKind::Mcp,
                description: row.get("description")?,
                tags,
            })
        })?;
        for r in mcp_rows {
            results.push(r?);
        }

        // Search Skills
        let mut stmt = conn.prepare(
            "SELECT id, name, description, tags_json FROM skills 
             WHERE name LIKE ?1 OR description LIKE ?1 OR tags_json LIKE ?1",
        )?;
        let skill_rows = stmt.query_map(params![like_query], |row| {
            let tags_json: String = row.get("tags_json")?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(CapabilityRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                kind: CapabilityKind::Skill,
                description: row.get("description")?,
                tags,
            })
        })?;
        for r in skill_rows {
            results.push(r?);
        }

        // Search Workflows
        let mut stmt = conn.prepare(
            "SELECT id, name, description FROM workflows 
             WHERE name LIKE ?1 OR description LIKE ?1",
        )?;
        let workflow_rows = stmt.query_map(params![like_query], |row| {
            Ok(CapabilityRecord {
                id: row.get("id")?,
                name: row.get("name")?,
                kind: CapabilityKind::Workflow,
                description: row.get("description")?,
                tags: vec![], // workflows don't store explicit tags in schema
            })
        })?;
        for r in workflow_rows {
            results.push(r?);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_registry_mcp_ops() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("vault.db");
        let registry = SqliteRegistry::new(&db_path).unwrap();

        let mcp = McpEntry {
            id: "01J5XQ9Z".to_string(),
            name: "filesystem".to_string(),
            display_name: Some("Filesystem Access".to_string()),
            version: "1.0.0".to_string(),
            source: McpSource::Npm {
                package: "@anthropic/mcp-filesystem".to_string(),
            },
            install_path: PathBuf::from("/tmp/mcp-filesystem"),
            command: "npx".to_string(),
            args: vec!["@anthropic/mcp-filesystem".to_string()],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec!["filesystem".to_string()],
            description: None,
        };

        registry.insert_mcp(&mcp).unwrap();

        let fetched = registry.get_mcp("filesystem").unwrap();
        assert_eq!(fetched.name, "filesystem");
        assert_eq!(fetched.version, "1.0.0");

        let list = registry.list_mcps().unwrap();
        assert_eq!(list.len(), 1);

        let mut updated = mcp.clone();
        updated.version = "1.0.1".to_string();
        registry.update_mcp(&updated).unwrap();

        let fetched_updated = registry.get_mcp("filesystem").unwrap();
        assert_eq!(fetched_updated.version, "1.0.1");

        registry.delete_mcp("filesystem").unwrap();
        assert!(registry.get_mcp("filesystem").is_err());
    }
}
