use crate::error::VaultError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Global AgentVault configuration, stored in ~/.agentvault/config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Root directory for all vault data.
    /// Default: ~/.agentvault/
    #[serde(default = "default_vault_dir")]
    pub vault_dir: PathBuf,

    /// Default agent to sync to when no agent is specified.
    pub default_agent: Option<String>,

    /// If true, automatically sync to all registered agents after install/remove/update.
    #[serde(default)]
    pub sync_on_install: bool,

    /// If true, create a backup of agent configs before every sync.
    #[serde(default = "default_true")]
    pub backup_before_sync: bool,

    /// Maximum number of backups to keep per agent.
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,

    /// Log level for file logging. One of: trace, debug, info, warn, error.
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// If true, mask sensitive environment variable values in CLI output.
    #[serde(default = "default_true")]
    pub mask_secrets: bool,

    /// Patterns for identifying secret env var names (case-insensitive substring match).
    #[serde(default = "default_secret_patterns")]
    pub secret_patterns: Vec<String>,
}

fn default_vault_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".agentvault")
}

fn default_true() -> bool {
    true
}

fn default_max_backups() -> usize {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_secret_patterns() -> Vec<String> {
    vec![
        "token".to_string(),
        "key".to_string(),
        "secret".to_string(),
        "password".to_string(),
        "credential".to_string(),
    ]
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            vault_dir: default_vault_dir(),
            default_agent: None,
            sync_on_install: false,
            backup_before_sync: true,
            max_backups: default_max_backups(),
            log_level: default_log_level(),
            mask_secrets: true,
            secret_patterns: default_secret_patterns(),
        }
    }
}

/// Resolves the vault directory using CLI override, environment variable, or default.
pub fn resolve_vault_dir(cli_override: Option<&str>) -> PathBuf {
    if let Some(o) = cli_override {
        PathBuf::from(o)
    } else if let Ok(env_val) = std::env::var("AGENTVAULT_DIR") {
        if !env_val.is_empty() {
            return PathBuf::from(env_val);
        }
        default_vault_dir()
    } else {
        default_vault_dir()
    }
}

impl VaultConfig {
    /// Loads configuration from the specified path.
    pub fn load(path: &Path) -> Result<Self, VaultError> {
        if !path.exists() {
            return Err(VaultError::Config {
                message: format!("Config file not found at {}", path.display()),
            });
        }
        let content = std::fs::read_to_string(path)?;
        let config: VaultConfig = toml::from_str(&content).map_err(|e| VaultError::Config {
            message: format!("Failed to parse config TOML: {}", e),
        })?;
        Ok(config)
    }

    /// Saves configuration to the specified path atomically.
    pub fn save(&self, path: &Path) -> Result<(), VaultError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| VaultError::Config {
            message: format!("Failed to serialize config to TOML: {}", e),
        })?;

        // Atomic write via tempfile
        let temp_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let mut temp_file =
            tempfile::NamedTempFile::new_in(temp_dir).map_err(|e| VaultError::Io(e))?;
        use std::io::Write;
        temp_file.write_all(content.as_bytes())?;
        temp_file
            .persist(path)
            .map_err(|e| VaultError::Io(e.error))?;

        Ok(())
    }

    /// Determines if a variable name matches any of the secret patterns.
    pub fn is_secret(&self, name: &str) -> bool {
        if !self.mask_secrets {
            return false;
        }
        let lower_name = name.to_lowercase();
        self.secret_patterns
            .iter()
            .any(|pattern| lower_name.contains(&pattern.to_lowercase()))
    }
}
