use crate::traits::AgentConnector;
use std::path::{Path, PathBuf};
use vault_core::agent::AgentType;

pub struct OpenCodeConnector {
    config_path: PathBuf,
    backup_dir: PathBuf,
}

impl Default for OpenCodeConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenCodeConnector {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                if cfg!(target_os = "windows") {
                    dirs::config_dir().unwrap_or_else(|| home.join("AppData").join("Roaming"))
                } else {
                    home.join(".config")
                }
            });
        Self {
            config_path: config_dir.join("opencode").join("config.json"),
            backup_dir: home.join(".agentvault").join("backups").join("opencode"),
        }
    }

    pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_path,
            backup_dir,
        }
    }
}

impl AgentConnector for OpenCodeConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::OpenCode
    }

    fn config_path(&self) -> &Path {
        &self.config_path
    }

    fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}
