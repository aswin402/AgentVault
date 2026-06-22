use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// An installed skill in the vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillEntry {
    /// Unique identifier (ULID/UUID).
    pub id: String,

    /// Short machine-friendly name, unique within the vault.
    pub name: String,

    /// Human-readable description of what this skill does.
    pub description: Option<String>,

    /// Absolute path to the skill directory.
    pub path: PathBuf,

    /// Free-form tags for categorization and search.
    pub tags: Vec<String>,

    /// Where the skill was sourced from.
    pub source: SkillSource,

    /// Timestamp when this skill was first installed.
    pub installed_at: DateTime<Utc>,

    /// Which agents this skill should be synced to.
    pub agents: Vec<String>,
}

/// Where a skill was sourced from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillSource {
    Git {
        repo: String,
        #[serde(rename = "ref")]
        ref_: Option<String>,
        subdirectory: Option<String>,
    },
    Local {
        path: PathBuf,
    },
}
