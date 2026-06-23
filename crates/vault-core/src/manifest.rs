use crate::error::VaultError;
use crate::registry::Registry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultManifest {
    pub vault: VaultMetadata,
    #[serde(default)]
    pub mcp: Vec<McpManifestEntry>,
    #[serde(default)]
    pub skill: Vec<SkillManifestEntry>,
    #[serde(default)]
    pub workflow: Vec<WorkflowManifestEntry>,
    #[serde(default)]
    pub agents: AgentsManifestSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VaultMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpManifestEntry {
    pub name: String,
    pub source: String,
    #[serde(default = "default_version_constraint")]
    pub version: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub args: Vec<String>,
}

fn default_version_constraint() -> String {
    "latest".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillManifestEntry {
    pub name: String,
    pub source: String,
    #[serde(default = "default_version_constraint")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowManifestEntry {
    pub name: String,
    pub source: String,
    #[serde(default = "default_version_constraint")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AgentsManifestSection {
    #[serde(default)]
    pub sync: Vec<String>,
}

impl VaultManifest {
    /// Create a VaultManifest from the current SQLite registry database.
    pub fn from_registry(registry: &dyn Registry) -> Result<Self, VaultError> {
        let mcps = registry.list_mcps()?;
        let skills = registry.list_skills()?;
        let workflows = registry.list_workflows()?;
        let agent_configs = registry.list_agent_configs()?;

        let mcp_entries = mcps
            .into_iter()
            .map(|m| McpManifestEntry {
                name: m.name,
                source: m.source.to_string(),
                version: m.version,
                env: m.env_vars,
                args: m.args,
            })
            .collect();

        let skill_entries = skills
            .into_iter()
            .map(|s| SkillManifestEntry {
                name: s.name,
                source: s.source.to_string(),
                version: "latest".to_string(),
            })
            .collect();

        let workflow_entries = workflows
            .into_iter()
            .map(|w| WorkflowManifestEntry {
                name: w.name,
                source: "local".to_string(),
                version: "latest".to_string(),
            })
            .collect();

        let sync_agents = agent_configs
            .into_iter()
            .filter(|ac| ac.enabled)
            .map(|ac| ac.agent_type.to_string())
            .collect();

        Ok(VaultManifest {
            vault: VaultMetadata {
                name: "AgentVault".to_string(),
                version: "0.0.1".to_string(),
                description: Some("Declarative AgentVault Manifest".to_string()),
            },
            mcp: mcp_entries,
            skill: skill_entries,
            workflow: workflow_entries,
            agents: AgentsManifestSection { sync: sync_agents },
        })
    }

    /// Parse a TOML string into a VaultManifest.
    pub fn parse(toml_content: &str) -> Result<Self, VaultError> {
        let manifest: Self = toml::from_str(toml_content)
            .map_err(|e| VaultError::Serialization(format!("TOML deserialization error: {}", e)))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Serialize the manifest into a pretty TOML string.
    pub fn to_toml_string(&self) -> Result<String, VaultError> {
        toml::to_string_pretty(self)
            .map_err(|e| VaultError::Serialization(format!("TOML serialization error: {}", e)))
    }

    /// Validate the manifest structure and content constraints.
    pub fn validate(&self) -> Result<(), VaultError> {
        if self.vault.name.trim().is_empty() {
            return Err(VaultError::Config {
                message: "Vault name in metadata cannot be empty".to_string(),
            });
        }

        // Validate MCPs
        for entry in &self.mcp {
            if entry.name.trim().is_empty() {
                return Err(VaultError::Config {
                    message: "MCP name cannot be empty".to_string(),
                });
            }

            // Check source is parseable
            let _: crate::mcp::models::McpSource =
                entry.source.parse().map_err(|e| VaultError::Config {
                    message: format!(
                        "Invalid source '{}' for MCP '{}': {}",
                        entry.source, entry.name, e
                    ),
                })?;

            // Check version constraint
            if entry.version != "latest" && !entry.version.trim().is_empty() {
                semver::VersionReq::parse(&entry.version).map_err(|e| VaultError::Config {
                    message: format!(
                        "Invalid version constraint '{}' for MCP '{}': {}",
                        entry.version, entry.name, e
                    ),
                })?;
            }
        }

        // Validate Skills
        for entry in &self.skill {
            if entry.name.trim().is_empty() {
                return Err(VaultError::Config {
                    message: "Skill name cannot be empty".to_string(),
                });
            }

            if entry.source.trim().is_empty() {
                return Err(VaultError::Config {
                    message: format!("Skill '{}' source cannot be empty", entry.name),
                });
            }

            if entry.version != "latest" && !entry.version.trim().is_empty() {
                semver::VersionReq::parse(&entry.version).map_err(|e| VaultError::Config {
                    message: format!(
                        "Invalid version constraint '{}' for skill '{}': {}",
                        entry.version, entry.name, e
                    ),
                })?;
            }
        }

        // Validate Workflows
        for entry in &self.workflow {
            if entry.name.trim().is_empty() {
                return Err(VaultError::Config {
                    message: "Workflow name cannot be empty".to_string(),
                });
            }

            if entry.source.trim().is_empty() {
                return Err(VaultError::Config {
                    message: format!("Workflow '{}' source cannot be empty", entry.name),
                });
            }

            if entry.version != "latest" && !entry.version.trim().is_empty() {
                semver::VersionReq::parse(&entry.version).map_err(|e| VaultError::Config {
                    message: format!(
                        "Invalid version constraint '{}' for workflow '{}': {}",
                        entry.version, entry.name, e
                    ),
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let toml_str = r#"
            [vault]
            name = "my-vault"
            version = "1.0.0"
            description = "A description of the vault"

            [[mcp]]
            name = "filesystem"
            source = "npm:@modelcontextprotocol/server-filesystem"
            version = "^0.1.0"
            args = ["/path/to/shared"]
            
            [mcp.env]
            PORT = "3000"

            [[skill]]
            name = "git-operations"
            source = "local:/home/user/skills/git"
            version = "latest"

            [agents]
            sync = ["claude", "gemini"]
        "#;

        let manifest = VaultManifest::parse(toml_str).unwrap();
        assert_eq!(manifest.vault.name, "my-vault");
        assert_eq!(manifest.mcp.len(), 1);
        assert_eq!(manifest.mcp[0].name, "filesystem");
        assert_eq!(manifest.mcp[0].env.get("PORT").unwrap(), "3000");
        assert_eq!(manifest.skill.len(), 1);
        assert_eq!(manifest.agents.sync, vec!["claude", "gemini"]);
    }

    #[test]
    fn test_parse_invalid_manifests() {
        // Missing name
        let toml_str = r#"
            [vault]
            name = ""
            version = "1.0.0"
        "#;
        assert!(VaultManifest::parse(toml_str).is_err());

        // Invalid semver constraint
        let toml_str = r#"
            [vault]
            name = "my-vault"
            version = "1.0.0"

            [[mcp]]
            name = "filesystem"
            source = "npm:@modelcontextprotocol/server-filesystem"
            version = "invalid-semver-123"
        "#;
        assert!(VaultManifest::parse(toml_str).is_err());

        // Invalid source format
        let toml_str = r#"
            [vault]
            name = "my-vault"
            version = "1.0.0"

            [[mcp]]
            name = "filesystem"
            source = "invalid_prefix:something"
        "#;
        assert!(VaultManifest::parse(toml_str).is_err());
    }

    #[test]
    fn test_from_registry() {
        use crate::agent::{AgentConnectorConfig, AgentType};
        use crate::mcp::models::{McpEntry, McpSource, McpStatus, McpTransport};
        use crate::registry::SqliteRegistry;
        use chrono::Utc;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("vault.db");
        let registry = SqliteRegistry::new(&db_path).unwrap();

        // 1. Insert dummy MCP
        let mcp = McpEntry {
            id: "mcp1".to_string(),
            name: "fs".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Npm {
                package: "filesystem".to_string(),
            },
            install_path: std::path::PathBuf::from("/vault/mcps/fs"),
            command: "node".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec![],
            description: None,
        };
        registry.insert_mcp(&mcp).unwrap();

        // 2. Insert dummy Agent configuration
        let agent = AgentConnectorConfig {
            id: "agent1".to_string(),
            agent_type: AgentType::ClaudeCode,
            config_path: std::path::PathBuf::from("/home/.claude.json"),
            enabled: true,
            last_synced: None,
            auto_sync: false,
        };
        registry.insert_agent_config(&agent).unwrap();

        // 3. Generate manifest and assert
        let manifest = VaultManifest::from_registry(&registry).unwrap();
        assert_eq!(manifest.vault.name, "AgentVault");
        assert_eq!(manifest.mcp.len(), 1);
        assert_eq!(manifest.mcp[0].name, "fs");
        assert_eq!(manifest.mcp[0].source, "npm:filesystem");
        assert_eq!(manifest.agents.sync, vec!["claude"]);
    }
}
