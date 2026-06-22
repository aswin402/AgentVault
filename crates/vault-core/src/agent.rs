use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a registered agent connector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConnectorConfig {
    /// Unique identifier (ULID/UUID).
    pub id: String,

    /// The type of AI agent.
    pub agent_type: AgentType,

    /// Absolute path to the agent's configuration file.
    pub config_path: PathBuf,

    /// Whether this connector is enabled for sync operations.
    pub enabled: bool,

    /// Timestamp of the last successful sync, if any.
    pub last_synced: Option<DateTime<Utc>>,

    /// Whether to automatically sync when capabilities are installed/removed.
    pub auto_sync: bool,
}

/// Supported AI agent types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    ClaudeCode,
    GeminiCli,
    OpenCode,
    CodexCli,
    Cursor,
    Custom(String),
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::ClaudeCode => write!(f, "claude"),
            AgentType::GeminiCli => write!(f, "gemini"),
            AgentType::OpenCode => write!(f, "opencode"),
            AgentType::CodexCli => write!(f, "codex"),
            AgentType::Cursor => write!(f, "cursor"),
            AgentType::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl std::str::FromStr for AgentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Ok(AgentType::ClaudeCode),
            "gemini" | "gemini-cli" | "geminicli" => Ok(AgentType::GeminiCli),
            "opencode" | "open-code" => Ok(AgentType::OpenCode),
            "codex" | "codex-cli" | "codexcli" => Ok(AgentType::CodexCli),
            "cursor" => Ok(AgentType::Cursor),
            other => Ok(AgentType::Custom(other.to_string())),
        }
    }
}

/// Record of a single sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHistoryEntry {
    pub id: String,
    pub agent_type: String,
    pub action: String,
    pub diff_json: String,
    pub synced_at: DateTime<Utc>,
    pub success: bool,
    pub error: Option<String>,
}
