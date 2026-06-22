use crate::error::VaultError;
use std::path::Path;

/// Creates the complete ~/.agentvault/ directory structure.
pub fn initialize_vault_directories(vault_dir: &Path) -> Result<(), VaultError> {
    let subdirs = ["mcps", "skills", "workflows", "backups", "logs"];

    // Create the root directory
    std::fs::create_dir_all(vault_dir)?;

    // Create the subdirectories
    for subdir in &subdirs {
        let path = vault_dir.join(subdir);
        std::fs::create_dir_all(&path)?;
    }

    Ok(())
}
