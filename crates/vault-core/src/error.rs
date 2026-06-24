#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Failed to perform I/O operation: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database operation failed: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Agent connector '{agent}' error: {message}")]
    Connector { agent: String, message: String },

    #[error("Failed to install from {source_type}: {message}")]
    McpInstall {
        source_type: String,
        message: String,
    },

    #[error("{kind} '{name}' not found in vault")]
    NotFound { kind: String, name: String },

    #[error("{kind} '{name}' is already installed")]
    AlreadyExists { kind: String, name: String },

    #[error("Version conflict for '{name}': wanted {wanted}, found {found}")]
    VersionConflict {
        name: String,
        wanted: String,
        found: String,
    },

    #[error("Permission denied to path: {path}")]
    PermissionDenied { path: String },

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl VaultError {
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            VaultError::Io(_) => Some("Check file permissions and disk space"),
            VaultError::Database(_) => Some("Run 'vault doctor' to check database health"),
            VaultError::Config { .. } => Some("Run 'vault init' to regenerate defaults"),
            VaultError::Network(_) => Some("Check your internet connection and try again"),
            VaultError::Connector { .. } => Some("Verify agent config file exists and is valid"),
            VaultError::McpInstall { .. } => Some("Run 'vault doctor' to verify prerequisites"),
            VaultError::NotFound { .. } => Some("Run 'vault list' to see installed items"),
            VaultError::AlreadyExists { .. } => Some("Use 'vault update' to update it"),
            VaultError::VersionConflict { .. } => Some("Use '--force' to override"),
            VaultError::PermissionDenied { .. } => Some("Check folder permissions"),
            VaultError::Serialization(_) => None,
        }
    }
}
