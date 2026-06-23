use crate::traits::AgentConnector;
use std::path::{Path, PathBuf};
use vault_core::agent::AgentType;

pub struct CodexConnector {
    config_path: PathBuf,
    backup_dir: PathBuf,
}

impl Default for CodexConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexConnector {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not determine home directory");
        Self {
            config_path: home.join(".codex").join("config.json"),
            backup_dir: home.join(".agentvault").join("backups").join("codex"),
        }
    }

    pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
        Self {
            config_path,
            backup_dir,
        }
    }
}

impl AgentConnector for CodexConnector {
    fn agent_type(&self) -> AgentType {
        AgentType::CodexCli
    }

    fn config_path(&self) -> &Path {
        &self.config_path
    }

    fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }
}
