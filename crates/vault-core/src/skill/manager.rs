use crate::error::VaultError;
use crate::registry::Registry;
use crate::skill::models::{SkillEntry, SkillSource};
use async_trait::async_trait;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[async_trait]
pub trait SkillManager: Send + Sync {
    async fn install(
        &self,
        source: SkillSource,
        agents: Vec<String>,
        tags: Vec<String>,
    ) -> Result<SkillEntry, VaultError>;

    async fn remove(&self, name: &str) -> Result<(), VaultError>;
    fn get(&self, name: &str) -> Result<SkillEntry, VaultError>;
    fn list(&self) -> Result<Vec<SkillEntry>, VaultError>;
}

pub struct DefaultSkillManager {
    registry: Arc<dyn Registry>,
    vault_dir: PathBuf,
}

impl DefaultSkillManager {
    pub fn new(registry: Arc<dyn Registry>, vault_dir: PathBuf) -> Self {
        Self {
            registry,
            vault_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillMetadata {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub fn parse_frontmatter(content: &str) -> Result<SkillMetadata, String> {
    let parts: Vec<&str> = content.split("---").collect();
    if parts.len() < 3 {
        return Err("Malformed frontmatter: missing '---' separators".to_string());
    }
    let frontmatter = parts[1];
    let mut name = String::new();
    let mut description = None;
    let mut version = None;
    let mut tags = Vec::new();

    let mut in_metadata = false;

    for line in frontmatter.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "metadata:" {
            in_metadata = true;
            continue;
        }
        if in_metadata && !line.starts_with("  ") && line.contains(':') && !line.starts_with(' ') {
            in_metadata = false;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if in_metadata {
                if key == "version" {
                    version = Some(value.to_string());
                }
            } else {
                match key {
                    "name" => name = value.to_string(),
                    "description" => description = Some(value.to_string()),
                    "version" => version = Some(value.to_string()),
                    "tags" => {
                        let value = value.trim_matches('[').trim_matches(']');
                        tags = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }
    }

    if name.is_empty() {
        return Err("Missing 'name' field in frontmatter".to_string());
    }

    Ok(SkillMetadata {
        name,
        description,
        version,
        tags: if tags.is_empty() { None } else { Some(tags) },
    })
}

fn read_skill_metadata(dir: &Path) -> Result<SkillMetadata, VaultError> {
    let mut path = dir.join("SKILL.md");
    if !path.exists() {
        path = dir.join("skill.md");
    }
    if !path.exists() {
        return Err(VaultError::NotFound {
            kind: "SKILL.md".to_string(),
            name: dir.display().to_string(),
        });
    }

    let content = std::fs::read_to_string(&path)?;
    parse_frontmatter(&content).map_err(|e| VaultError::Config { message: e })
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn clean_target_dir(path: &Path) -> Result<(), VaultError> {
    if let Ok(meta) = path.symlink_metadata() {
        if meta.is_dir() {
            if meta.file_type().is_symlink() {
                std::fs::remove_dir(path)?;
            } else {
                std::fs::remove_dir_all(path)?;
            }
        } else {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[async_trait]
impl SkillManager for DefaultSkillManager {
    async fn install(
        &self,
        source: SkillSource,
        agents: Vec<String>,
        tags: Vec<String>,
    ) -> Result<SkillEntry, VaultError> {
        let temp_dir;
        let skill_source_path = match &source {
            SkillSource::Local { path } => {
                if !path.exists() {
                    return Err(VaultError::NotFound {
                        kind: "local_path".to_string(),
                        name: path.display().to_string(),
                    });
                }
                path.clone()
            }
            SkillSource::Git {
                repo,
                ref_,
                subdirectory,
            } => {
                temp_dir = tempfile::tempdir()?;
                let temp_path = temp_dir.path();

                let status = tokio::process::Command::new("git")
                    .arg("clone")
                    .arg(repo)
                    .arg(temp_path)
                    .status()
                    .await?;
                if !status.success() {
                    return Err(VaultError::Config {
                        message: format!("Failed to clone repository: {}", repo),
                    });
                }

                if let Some(ref_val) = ref_ {
                    let status = tokio::process::Command::new("git")
                        .arg("-C")
                        .arg(temp_path)
                        .arg("checkout")
                        .arg(ref_val)
                        .status()
                        .await?;
                    if !status.success() {
                        return Err(VaultError::Config {
                            message: format!(
                                "Failed to checkout ref '{}' in repo: {}",
                                ref_val, repo
                            ),
                        });
                    }
                }

                let mut path = temp_path.to_path_buf();
                if let Some(sub) = subdirectory {
                    path = path.join(sub);
                }
                path
            }
        };

        // Read and parse skill metadata
        let metadata = read_skill_metadata(&skill_source_path)?;

        let final_install_path = self.vault_dir.join("skills").join(&metadata.name);
        if let Some(parent) = final_install_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        clean_target_dir(&final_install_path)?;

        // Move or link the files
        match &source {
            SkillSource::Local { path } => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(path, &final_install_path)?;
                #[cfg(windows)]
                std::os::windows::fs::symlink_dir(path, &final_install_path)?;
            }
            SkillSource::Git { .. } => {
                copy_dir_all(&skill_source_path, &final_install_path)?;
            }
        }

        // Merge tags
        let mut final_tags = tags;
        if let Some(ref frontmatter_tags) = metadata.tags {
            for tag in frontmatter_tags {
                if !final_tags.contains(tag) {
                    final_tags.push(tag.clone());
                }
            }
        }

        let entry = SkillEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            path: final_install_path,
            tags: final_tags,
            source: source.clone(),
            installed_at: Utc::now(),
            agents: agents.clone(),
        };

        // Check if skill with this name is already in the database
        // and update or replace, or return error. Wait, TODO.md says register in SQLite.
        // If it already exists, let's delete existing or update it.
        if self.registry.get_skill(&metadata.name).is_ok() {
            self.registry.update_skill(&entry)?;
        } else {
            self.registry.insert_skill(&entry)?;
        }

        Ok(entry)
    }

    async fn remove(&self, name: &str) -> Result<(), VaultError> {
        let entry = self.registry.get_skill(name)?;
        clean_target_dir(&entry.path)?;
        self.registry.delete_skill(name)?;
        Ok(())
    }

    fn get(&self, name: &str) -> Result<SkillEntry, VaultError> {
        self.registry.get_skill(name)
    }

    fn list(&self) -> Result<Vec<SkillEntry>, VaultError> {
        self.registry.list_skills()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SqliteRegistry;
    use tempfile::tempdir;

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = r#"---
name: rust-help
description: "Rust development helper"
version: 1.2.3
tags: [rust, helper, coding]
---
# Content of skill
Some details...
"#;
        let meta = parse_frontmatter(content).unwrap();
        assert_eq!(meta.name, "rust-help");
        assert_eq!(meta.description.unwrap(), "Rust development helper");
        assert_eq!(meta.version.unwrap(), "1.2.3");
        assert_eq!(meta.tags.unwrap(), vec!["rust", "helper", "coding"]);
    }

    #[test]
    fn test_parse_frontmatter_invalid() {
        let no_separators = "name: rust-help";
        assert!(parse_frontmatter(no_separators).is_err());

        let missing_name = r#"---
description: "Rust development helper"
---"#;
        assert!(parse_frontmatter(missing_name).is_err());
    }

    #[tokio::test]
    async fn test_skill_install_local_and_remove() {
        let temp_vault = tempdir().unwrap();
        let db_path = temp_vault.path().join("vault.db");
        let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
        let manager = DefaultSkillManager::new(registry.clone(), temp_vault.path().to_path_buf());

        // Create a dummy local skill directory
        let local_skill_dir = tempdir().unwrap();
        let skill_md_path = local_skill_dir.path().join("SKILL.md");
        let content = r#"---
name: test-skill
description: "A test skill"
version: 1.0.0
tags: [test, skill]
---
# Test Skill
"#;
        std::fs::write(&skill_md_path, content).unwrap();

        // Install local skill
        let source = SkillSource::Local {
            path: local_skill_dir.path().to_path_buf(),
        };
        let entry = manager
            .install(
                source,
                vec!["claude".to_string()],
                vec!["extra-tag".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(entry.name, "test-skill");
        assert_eq!(entry.description.unwrap(), "A test skill");
        assert!(entry.tags.contains(&"test".to_string()));
        assert!(entry.tags.contains(&"extra-tag".to_string()));

        // Verify registry record
        let registered = registry.get_skill("test-skill").unwrap();
        assert_eq!(registered.name, "test-skill");

        // Verify files/symlink in vault
        let expected_path = temp_vault.path().join("skills").join("test-skill");
        assert!(expected_path.exists());

        // Remove skill
        manager.remove("test-skill").await.unwrap();
        assert!(!expected_path.exists());
        assert!(registry.get_skill("test-skill").is_err());
    }
}
