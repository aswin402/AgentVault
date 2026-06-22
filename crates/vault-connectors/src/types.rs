use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentMcpConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub raw: Value,
    #[serde(default)]
    pub mcp_servers: HashMap<String, AgentMcpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncEntry {
    pub name: String,
    pub source: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldChange {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncUpdate {
    pub name: String,
    pub changed_fields: Vec<FieldChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SyncDiff {
    pub additions: Vec<SyncEntry>,
    pub removals: Vec<SyncEntry>,
    pub updates: Vec<SyncUpdate>,
}

impl SyncDiff {
    pub fn is_empty(&self) -> bool {
        self.additions.is_empty() && self.removals.is_empty() && self.updates.is_empty()
    }

    pub fn change_count(&self) -> usize {
        self.additions.len() + self.removals.len() + self.updates.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncResult {
    pub agent_type: String,
    pub timestamp: DateTime<Utc>,
    pub diff: SyncDiff,
    pub success: bool,
    pub backup_path: Option<String>,
    pub error: Option<String>,
}
