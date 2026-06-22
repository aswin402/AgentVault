#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Connector error ({agent}): {message}")]
    Connector { agent: String, message: String },

    #[error("MCP installation failed ({source_type}): {message}")]
    McpInstall {
        source_type: String,
        message: String,
    },

    #[error("Not found: {kind} '{name}'")]
    NotFound { kind: String, name: String },

    #[error("Already exists: {kind} '{name}'")]
    AlreadyExists { kind: String, name: String },

    #[error("Version conflict for '{name}': wanted {wanted}, found {found}")]
    VersionConflict {
        name: String,
        wanted: String,
        found: String,
    },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Serialization error: {0}")]
    Serialization(String),
}
