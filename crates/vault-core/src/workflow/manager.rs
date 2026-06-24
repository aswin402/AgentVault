use crate::error::VaultError;
use crate::registry::Registry;
use crate::workflow::models::{WorkflowEntry, WorkflowStep};
use crate::workflow::resolver::DependencyResolver;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub step_name: String,
    pub capability: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowSource {
    Git {
        repo: String,
        #[serde(rename = "ref")]
        ref_: Option<String>,
        subdirectory: Option<String>,
    },
    Local {
        path: PathBuf,
    },
}

#[async_trait]
pub trait WorkflowManager: Send + Sync {
    async fn install(&self, source: WorkflowSource) -> Result<WorkflowEntry, VaultError>;
    async fn remove(&self, name: &str) -> Result<(), VaultError>;
    fn get(&self, name: &str) -> Result<WorkflowEntry, VaultError>;
    fn list(&self) -> Result<Vec<WorkflowEntry>, VaultError>;
    fn validate(&self, name: &str) -> Result<Vec<ValidationIssue>, VaultError>;
}

pub struct DefaultWorkflowManager {
    registry: Arc<dyn Registry>,
    #[allow(dead_code)]
    vault_dir: PathBuf,
}

impl DefaultWorkflowManager {
    pub fn new(registry: Arc<dyn Registry>, vault_dir: PathBuf) -> Self {
        Self {
            registry,
            vault_dir,
        }
    }
}

// Struct to deserialize workflow.toml
#[derive(Debug, Deserialize)]
struct WorkflowToml {
    workflow: WorkflowMetadata,
    #[serde(rename = "step")]
    steps: Vec<WorkflowStepToml>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WorkflowMetadata {
    name: String,
    version: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct WorkflowStepToml {
    name: String,
    uses: String,
    #[serde(default)]
    args: HashMap<String, String>,
    #[serde(default)]
    depends_on: Vec<String>,
    condition: Option<String>,
}

fn parse_workflow_toml(content: &str) -> Result<WorkflowToml, String> {
    toml::from_str::<WorkflowToml>(content).map_err(|e| e.to_string())
}

#[async_trait]
impl WorkflowManager for DefaultWorkflowManager {
    async fn install(&self, source: WorkflowSource) -> Result<WorkflowEntry, VaultError> {
        let temp_dir;
        let toml_path = match &source {
            WorkflowSource::Local { path } => {
                if !path.exists() {
                    return Err(VaultError::NotFound {
                        kind: "local_path".to_string(),
                        name: path.display().to_string(),
                    });
                }
                if path.is_file() {
                    path.clone()
                } else {
                    let mut tp = path.join("workflow.toml");
                    if !tp.exists() {
                        tp = path.join("workflow.json"); // optional fallback
                    }
                    if !tp.exists() {
                        return Err(VaultError::NotFound {
                            kind: "workflow.toml".to_string(),
                            name: path.display().to_string(),
                        });
                    }
                    tp
                }
            }
            WorkflowSource::Git {
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

                let mut tp = path.join("workflow.toml");
                if !tp.exists() {
                    tp = path.join("workflow.json");
                }
                if !tp.exists() {
                    return Err(VaultError::NotFound {
                        kind: "workflow.toml".to_string(),
                        name: path.display().to_string(),
                    });
                }
                tp
            }
        };

        let content = std::fs::read_to_string(&toml_path)?;
        let parsed =
            parse_workflow_toml(&content).map_err(|e| VaultError::Config { message: e })?;

        let steps: Vec<WorkflowStep> = parsed
            .steps
            .into_iter()
            .map(|s| WorkflowStep {
                name: s.name,
                uses: s.uses,
                args: s.args,
                depends_on: s.depends_on,
                condition: s.condition,
            })
            .collect();

        // Validate DAG (topological sorting check)
        DependencyResolver::resolve(&steps)?;

        // Compute unique dependencies (all used capabilities)
        let mut deps_set = HashSet::new();
        for step in &steps {
            deps_set.insert(step.uses.clone());
        }
        let dependencies: Vec<String> = deps_set.into_iter().collect();

        let entry = WorkflowEntry {
            id: uuid::Uuid::new_v4().to_string(),
            name: parsed.workflow.name.clone(),
            description: parsed.workflow.description,
            steps,
            dependencies,
            installed_at: Utc::now(),
        };

        // Register in SQLite
        if self.registry.get_workflow(&parsed.workflow.name).is_ok() {
            self.registry.update_workflow(&entry)?;
        } else {
            self.registry.insert_workflow(&entry)?;
        }

        Ok(entry)
    }

    async fn remove(&self, name: &str) -> Result<(), VaultError> {
        self.registry.delete_workflow(name)?;
        Ok(())
    }

    fn get(&self, name: &str) -> Result<WorkflowEntry, VaultError> {
        self.registry.get_workflow(name)
    }

    fn list(&self) -> Result<Vec<WorkflowEntry>, VaultError> {
        self.registry.list_workflows()
    }

    fn validate(&self, name: &str) -> Result<Vec<ValidationIssue>, VaultError> {
        let entry = self.registry.get_workflow(name)?;
        let mut issues = Vec::new();

        for step in &entry.steps {
            if let Some((prefix, cap_name)) = step.uses.split_once(':') {
                match prefix {
                    "mcp" => {
                        if self.registry.get_mcp(cap_name).is_err() {
                            issues.push(ValidationIssue {
                                step_name: step.name.clone(),
                                capability: step.uses.clone(),
                                message: format!("Missing required MCP server: '{}'", cap_name),
                            });
                        }
                    }
                    "skill" => {
                        if self.registry.get_skill(cap_name).is_err() {
                            issues.push(ValidationIssue {
                                step_name: step.name.clone(),
                                capability: step.uses.clone(),
                                message: format!("Missing required Skill: '{}'", cap_name),
                            });
                        }
                    }
                    _ => {
                        issues.push(ValidationIssue {
                            step_name: step.name.clone(),
                            capability: step.uses.clone(),
                            message: format!("Unknown prefix type: '{}'", prefix),
                        });
                    }
                }
            } else {
                issues.push(ValidationIssue {
                    step_name: step.name.clone(),
                    capability: step.uses.clone(),
                    message: format!("Invalid format: '{}'", step.uses),
                });
            }
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::models::{McpEntry, McpSource, McpStatus, McpTransport};
    use crate::registry::SqliteRegistry;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_workflow_install_and_validate() {
        let temp_vault = tempdir().unwrap();
        let db_path = temp_vault.path().join("vault.db");
        let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
        let manager =
            DefaultWorkflowManager::new(registry.clone(), temp_vault.path().to_path_buf());

        // Create a dummy local workflow.toml
        let local_workflow_dir = tempdir().unwrap();
        let toml_path = local_workflow_dir.path().join("workflow.toml");
        let content = r#"
            [workflow]
            name = "review-workflow"
            version = "1.0.0"
            description = "A code review workflow"

            [[step]]
            name = "fetch-pr"
            uses = "mcp:github"
            depends_on = []

            [[step]]
            name = "analyze"
            uses = "mcp:analyzer"
            depends_on = ["fetch-pr"]
        "#;
        std::fs::write(&toml_path, content).unwrap();

        // Install
        let source = WorkflowSource::Local {
            path: local_workflow_dir.path().to_path_buf(),
        };
        let entry = manager.install(source).await.unwrap();
        assert_eq!(entry.name, "review-workflow");

        // Validate the workflow
        let issues = manager.validate("review-workflow").unwrap();
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|i| i.capability == "mcp:github"));
        assert!(issues.iter().any(|i| i.capability == "mcp:analyzer"));

        // Register dummy MCPs
        let mcp_github = McpEntry {
            id: "mcp1".to_string(),
            name: "github".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Npm {
                package: "is-number".to_string(),
            },
            install_path: PathBuf::from("/vault/mcps/github"),
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
        let mcp_analyzer = McpEntry {
            id: "mcp2".to_string(),
            name: "analyzer".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Npm {
                package: "is-number".to_string(),
            },
            install_path: PathBuf::from("/vault/mcps/analyzer"),
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
        registry.insert_mcp(&mcp_github).unwrap();
        registry.insert_mcp(&mcp_analyzer).unwrap();

        // Validate again - should be fully valid!
        let issues_after = manager.validate("review-workflow").unwrap();
        assert!(issues_after.is_empty());

        // Remove workflow
        manager.remove("review-workflow").await.unwrap();
        assert!(registry.get_workflow("review-workflow").is_err());
    }
}
