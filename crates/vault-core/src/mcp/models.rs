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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum McpStatus {
    #[default]
    Active,
    Disabled,
    Error {
        message: String,
    },
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

impl std::str::FromStr for McpSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((prefix, rest)) = s.split_once(':') {
            // Check if it's a Windows drive letter (e.g. C:\...)
            if prefix.len() == 1 && prefix.chars().next().unwrap().is_ascii_alphabetic() {
                return Ok(McpSource::Local {
                    path: PathBuf::from(s),
                });
            }

            match prefix.to_lowercase().as_str() {
                "npm" => {
                    return Ok(McpSource::Npm {
                        package: rest.to_string(),
                    })
                }
                "pypi" => {
                    return Ok(McpSource::PyPi {
                        package: rest.to_string(),
                    })
                }
                "local" => {
                    return Ok(McpSource::Local {
                        path: PathBuf::from(rest),
                    })
                }
                "github" => {
                    let parts: Vec<&str> = rest.split('@').collect();
                    let repo = parts[0].to_string();
                    let ref_ = parts.get(1).map(|s| s.to_string());
                    return Ok(McpSource::GitHub { repo, ref_ });
                }
                "docker" => {
                    return Ok(McpSource::Docker {
                        image: rest.to_string(),
                    })
                }
                _ => {
                    return Err(format!("Unknown source prefix: '{}'. Supported prefixes are: npm, pypi, local, github, docker.", prefix));
                }
            }
        }

        // Fallbacks
        if s.starts_with('/')
            || s.starts_with("./")
            || s.starts_with("../")
            || (s.contains('/') && !s.contains(':') && std::path::Path::new(s).exists())
        {
            Ok(McpSource::Local {
                path: PathBuf::from(s),
            })
        } else if s.contains('/') {
            let parts: Vec<&str> = s.split('@').collect();
            let repo = parts[0].to_string();
            let ref_ = parts.get(1).map(|s| s.to_string());
            Ok(McpSource::GitHub { repo, ref_ })
        } else {
            Ok(McpSource::Npm {
                package: s.to_string(),
            })
        }
    }
}
