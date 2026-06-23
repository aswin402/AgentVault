use crate::traits::AgentConnector;
use std::path::{Path, PathBuf};
use vault_core::agent::AgentType;

pub struct GeminiConnector {
    config_path: PathBuf,
    backup_dir: PathBuf,
}

impl Default for GeminiConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiConnector {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        Self {
            config_path: home.join(".gemini").join("config").join("settings.json"),
            backup_dir: home.join(".agentvault").join("backups").join("gemini"),
        }
    }

    pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_path,
            backup_dir,
        }
    }
}

impl AgentConnector for GeminiConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::GeminiCli
    }

    fn config_path(&self) -> &Path {
        &self.config_path
    }

    fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}
