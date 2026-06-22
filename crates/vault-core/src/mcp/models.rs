use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// This is the central data structure for an installed MCP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpEntry {
    /// Unique identifier (ULID/UUID format).
    pub id: String,

    /// Short machine-friendly name, unique within the vault.
    pub name: String,

    /// Optional human-friendly display name.
    pub display_name: Option<String>,

    /// Installed version string.
    pub version: String,

    /// Where the MCP was installed from.
    pub source: McpSource,

    /// Absolute path to the MCP's installation directory.
    pub install_path: PathBuf,

    /// The executable command to launch this MCP server.
    pub command: String,

    /// Arguments passed to the command.
    pub args: Vec<String>,

    /// Environment variables required by this MCP server.
    pub env_vars: HashMap<String, String>,

    /// Transport protocol the MCP server uses.
    pub transport: McpTransport,

    /// Current operational status.
    pub status: McpStatus,

    /// Timestamp when this MCP was first installed.
    pub installed_at: DateTime<Utc>,

    /// Timestamp of the most recent update.
    pub updated_at: DateTime<Utc>,

    /// SHA-256 checksum of the installed package.
    pub checksum: Option<String>,

    /// Which agents this MCP should be synced to.
    pub agents: Vec<String>,

    /// Free-form tags for categorization.
    pub tags: Vec<String>,

    /// Optional human-readable description.
    pub description: Option<String>,
}

/// Where an MCP server was sourced from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpSource {
    Npm {
        package: String,
    },
    PyPi {
        package: String,
    },
    GitHub {
        repo: String,
        #[serde(rename = "ref")]
        ref_: Option<String>,
    },
    Local {
        path: PathBuf,
    },
    Docker {
        image: String,
    },
}

/// Transport protocol for communicating with the MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Sse { url: String },
    StreamableHttp { url: String },
}

/// Operational status of an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum McpStatus {
    Active,
    Disabled,
    Error { message: String },
}

impl Default for McpStatus {
    fn default() -> Self {
        McpStatus::Active
    }
}

impl std::fmt::Display for McpSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpSource::Npm { package } => write!(f, "npm:{}", package),
            McpSource::PyPi { package } => write!(f, "pypi:{}", package),
            McpSource::GitHub { repo, ref_ } => {
                write!(f, "github:{}", repo)?;
                if let Some(r) = ref_ {
                    write!(f, "@{}", r)?;
                }
                Ok(())
            }
            McpSource::Local { path } => write!(f, "local:{}", path.display()),
            McpSource::Docker { image } => write!(f, "docker:{}", image),
        }
    }
}

impl std::fmt::Display for McpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpTransport::Stdio => write!(f, "stdio"),
            McpTransport::Sse { url } => write!(f, "sse:{}", url),
            McpTransport::StreamableHttp { url } => write!(f, "http:{}", url),
        }
    }
}

impl std::fmt::Display for McpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpStatus::Active => write!(f, "active"),
            McpStatus::Disabled => write!(f, "disabled"),
            McpStatus::Error { message } => write!(f, "error: {}", message),
        }
    }
}
