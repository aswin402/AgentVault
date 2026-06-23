use crate::traits::AgentConnector;
use std::path::{Path, PathBuf};
use vault_core::agent::AgentType;

pub struct ClaudeConnector {
    config_path: PathBuf,
    backup_dir: PathBuf,
}

impl Default for ClaudeConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeConnector {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        Self {
            config_path: home.join(".claude").join("claude_desktop_config.json"),
            backup_dir: home.join(".agentvault").join("backups").join("claude"),
        }
    }

    pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_path,
            backup_dir,
        }
    }
}

impl AgentConnector for ClaudeConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }

    fn config_path(&self) -> &Path {
        &self.config_path
    }

    fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}
