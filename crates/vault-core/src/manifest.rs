use crate::error::VaultError;
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
}
