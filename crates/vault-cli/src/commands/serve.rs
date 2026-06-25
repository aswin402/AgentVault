use crate::cli::ServeArgs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vault_core::config::resolve_vault_dir;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::mcp::models::McpSource;
use vault_core::registry::{Registry, SqliteRegistry};
use vault_core::search::SearchEngine;
use vault_core::skill::manager::{DefaultSkillManager, SkillManager};
use vault_core::skill::models::SkillSource;
use vault_core::store::initialize_vault_directories;
use vault_core::workflow::manager::{DefaultWorkflowManager, WorkflowManager};

#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

pub async fn handle(_args: ServeArgs, vault_dir_override: Option<&str>) -> Result<()> {
    let vault_dir = resolve_vault_dir(vault_dir_override);
    initialize_vault_directories(&vault_dir)?;

    let db_path = vault_dir.join("vault.db");
    let registry = Arc::new(SqliteRegistry::new(&db_path)?);

    let mcp_manager = Arc::new(DefaultMcpManager::new(registry.clone(), vault_dir.clone()));
    let skill_manager = Arc::new(DefaultSkillManager::new(
        registry.clone(),
        vault_dir.clone(),
    ));
    let workflow_manager = Arc::new(DefaultWorkflowManager::new(
        registry.clone(),
        vault_dir.clone(),
    ));

    let mut reader = BufReader::new(tokio::io::stdin()).lines();
    let mut writer = tokio::io::stdout();

    while let Some(line) = reader.next_line().await? {
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(json!({
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    })),
                };
                let resp_str = serde_json::to_string(&err_resp)?;
                writer
                    .write_all(format!("{}\n", resp_str).as_bytes())
                    .await?;
                writer.flush().await?;
                continue;
            }
        };
        if req.jsonrpc != "2.0" {
            let err_resp = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(json!({
                    "code": -32600,
                    "message": "Invalid Request: jsonrpc version must be '2.0'"
                })),
            };
            let resp_str = serde_json::to_string(&err_resp)?;
            writer
                .write_all(format!("{}\n", resp_str).as_bytes())
                .await?;
            writer.flush().await?;
            continue;
        }
        let response = handle_request(
            req,
            registry.clone(),
            mcp_manager.clone(),
            skill_manager.clone(),
            workflow_manager.clone(),
        )
        .await;

        if let Some(resp) = response {
            let resp_str = serde_json::to_string(&resp)?;
            writer
                .write_all(format!("{}\n", resp_str).as_bytes())
                .await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

async fn handle_request(
    req: JsonRpcRequest,
    registry: Arc<SqliteRegistry>,
    mcp_manager: Arc<DefaultMcpManager>,
    skill_manager: Arc<DefaultSkillManager>,
    workflow_manager: Arc<DefaultWorkflowManager>,
) -> Option<JsonRpcResponse> {
    let method = req.method.as_str();

    match method {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "agentvault",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(result),
                error: None,
            })
        }
        "notifications/initialized" => None,
        "tools/list" => {
            let tools = json!([
                {
                    "name": "list_capabilities",
                    "description": "List all installed capabilities in AgentVault (MCPs, Skills, and Workflows).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "install_capability",
                    "description": "Install a new capability (MCP, Skill, or Workflow) into the vault.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "source": { "type": "string", "description": "Format: npm:<pkg>, pypi:<pkg>, github:<repo>, local:<path>" },
                            "name": { "type": "string", "description": "Optional custom name override" },
                            "is_skill": { "type": "boolean", "description": "Set true to install as a skill" },
                            "is_workflow": { "type": "boolean", "description": "Set true to install as a workflow (toml path)" }
                        },
                        "required": ["source"]
                    }
                },
                {
                    "name": "remove_capability",
                    "description": "Remove a capability from the vault.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "The name of the capability to remove" }
                        },
                        "required": ["name"]
                    }
                },
                {
                    "name": "update_capability",
                    "description": "Update an installed MCP server to its latest version.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "The name of the MCP server to update" },
                            "force": { "type": "boolean", "description": "Force update even if already at latest version" }
                        },
                        "required": ["name"]
                    }
                },
                {
                    "name": "get_capability_details",
                    "description": "Get full metadata for a specific installed capability.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "The name of the capability to get details for" }
                        },
                        "required": ["name"]
                    }
                },
                {
                    "name": "set_capability_env",
                    "description": "Set an environment variable on an installed MCP server.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "The name of the MCP server" },
                            "key": { "type": "string", "description": "The environment variable name" },
                            "value": { "type": "string", "description": "The environment variable value" }
                        },
                        "required": ["name", "key", "value"]
                    }
                },
                {
                    "name": "search_registry",
                    "description": "Search for MCP servers available to install.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query string" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "doctor_check",
                    "description": "Run diagnostic health checks on the vault.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]);
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(json!({ "tools": tools })),
                error: None,
            })
        }
        "tools/call" => {
            let params = req.params.clone().unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            let call_res = execute_tool_call(
                name,
                arguments,
                registry,
                mcp_manager,
                skill_manager,
                workflow_manager,
            )
            .await;

            match call_res {
                Ok(text_content) => {
                    let result = json!({
                        "content": [
                            {
                                "type": "text",
                                "text": text_content
                            }
                        ]
                    });
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: req.id,
                        result: Some(result),
                        error: None,
                    })
                }
                Err(e) => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(json!({
                        "code": -32603,
                        "message": format!("Tool execution failed: {}", e)
                    })),
                }),
            }
        }
        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": format!("Method not found: {}", method)
            })),
        }),
    }
}

async fn execute_tool_call(
    name: &str,
    args: Value,
    registry: Arc<SqliteRegistry>,
    mcp_manager: Arc<DefaultMcpManager>,
    skill_manager: Arc<DefaultSkillManager>,
    workflow_manager: Arc<DefaultWorkflowManager>,
) -> Result<String> {
    match name {
        "list_capabilities" => {
            let mcps = registry.list_mcps()?;
            let skills = registry.list_skills()?;
            let workflows = registry.list_workflows()?;

            let summary = json!({
                "mcps": mcps.iter().map(|m| json!({
                    "name": m.name,
                    "version": m.version,
                    "status": format!("{:?}", m.status),
                    "description": m.description
                })).collect::<Vec<_>>(),
                "skills": skills.iter().map(|s| json!({
                    "name": s.name,
                    "description": s.description
                })).collect::<Vec<_>>(),
                "workflows": workflows.iter().map(|w| json!({
                    "name": w.name,
                    "steps_count": w.steps.len(),
                    "description": w.description
                })).collect::<Vec<_>>()
            });

            Ok(serde_json::to_string_pretty(&summary)?)
        }
        "install_capability" => {
            let source_str = args
                .get("source")
                .and_then(|v| v.as_str())
                .context("Missing source parameter")?;
            let name_str = args.get("name").and_then(|v| v.as_str());
            let is_skill = args
                .get("is_skill")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let is_workflow = args
                .get("is_workflow")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if is_skill {
                let source = if source_str.starts_with("git:") || source_str.contains("github.com")
                {
                    let repo = source_str
                        .strip_prefix("git:")
                        .unwrap_or(source_str)
                        .to_string();
                    SkillSource::Git {
                        repo,
                        ref_: None,
                        subdirectory: None,
                    }
                } else {
                    let path = std::path::PathBuf::from(
                        source_str.strip_prefix("local:").unwrap_or(source_str),
                    );
                    SkillSource::Local { path }
                };
                let entry = skill_manager.install(source, vec![], vec![]).await?;
                Ok(format!("Successfully installed skill '{}'", entry.name))
            } else if is_workflow {
                let path = std::path::PathBuf::from(
                    source_str.strip_prefix("local:").unwrap_or(source_str),
                );
                let entry = workflow_manager
                    .install(vault_core::workflow::manager::WorkflowSource::Local { path })
                    .await?;
                Ok(format!("Successfully installed workflow '{}'", entry.name))
            } else {
                let (source_type, val) = if let Some(stripped) = source_str.strip_prefix("npm:") {
                    ("npm", stripped)
                } else if let Some(stripped) = source_str.strip_prefix("pypi:") {
                    ("pypi", stripped)
                } else if let Some(stripped) = source_str.strip_prefix("github:") {
                    ("github", stripped)
                } else if let Some(stripped) = source_str.strip_prefix("local:") {
                    ("local", stripped)
                } else {
                    ("npm", source_str)
                };

                let mcp_source = match source_type {
                    "npm" => McpSource::Npm {
                        package: val.to_string(),
                    },
                    "pypi" => McpSource::PyPi {
                        package: val.to_string(),
                    },
                    "github" => McpSource::GitHub {
                        repo: val.to_string(),
                        ref_: None,
                    },
                    "local" => McpSource::Local {
                        path: std::path::PathBuf::from(val),
                    },
                    _ => McpSource::Npm {
                        package: val.to_string(),
                    },
                };

                let entry = mcp_manager
                    .install(
                        name_str.unwrap_or(val),
                        mcp_source,
                        "latest",
                        vec![],
                        HashMap::new(),
                        vec![],
                        vec![],
                        None,
                    )
                    .await?;

                Ok(format!(
                    "Successfully installed MCP server '{}' (version: {})",
                    entry.name, entry.version
                ))
            }
        }
        "remove_capability" => {
            let name_str = args
                .get("name")
                .and_then(|v| v.as_str())
                .context("Missing name parameter")?;

            let mut removed = false;
            if let Ok(entry) = registry.get_mcp(name_str) {
                mcp_manager.remove(&entry.name, false).await?;
                removed = true;
            } else if let Ok(entry) = registry.get_skill(name_str) {
                skill_manager.remove(&entry.name).await?;
                removed = true;
            } else if let Ok(entry) = registry.get_workflow(name_str) {
                workflow_manager.remove(&entry.name).await?;
                removed = true;
            }

            if removed {
                Ok(format!("Successfully removed capability '{}'", name_str))
            } else {
                Err(anyhow::anyhow!(
                    "Capability '{}' not found in vault",
                    name_str
                ))
            }
        }
        "update_capability" => {
            let name_str = args
                .get("name")
                .and_then(|v| v.as_str())
                .context("Missing name parameter")?;
            let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

            let entry = mcp_manager.update(name_str, force).await?;
            Ok(format!(
                "Successfully updated '{}' to version {}",
                entry.name, entry.version
            ))
        }
        "get_capability_details" => {
            let name_str = args
                .get("name")
                .and_then(|v| v.as_str())
                .context("Missing name parameter")?;

            if let Ok(entry) = registry.get_mcp(name_str) {
                let masked_env: HashMap<&str, &str> =
                    entry.env_vars.keys().map(|k| (k.as_str(), "***")).collect();
                let details = json!({
                    "type": "mcp",
                    "name": entry.name,
                    "version": entry.version,
                    "source": format!("{:?}", entry.source),
                    "status": format!("{:?}", entry.status),
                    "env_vars": masked_env,
                    "command": entry.command,
                    "args": entry.args,
                    "description": entry.description,
                    "tags": entry.tags,
                    "agents": entry.agents,
                    "transport": format!("{:?}", entry.transport),
                    "installed_at": entry.installed_at.to_rfc3339(),
                    "updated_at": entry.updated_at.to_rfc3339()
                });
                Ok(serde_json::to_string_pretty(&details)?)
            } else if let Ok(entry) = registry.get_skill(name_str) {
                let details = json!({
                    "type": "skill",
                    "name": entry.name,
                    "description": entry.description,
                    "path": entry.path.display().to_string(),
                    "tags": entry.tags,
                    "source": format!("{:?}", entry.source),
                    "agents": entry.agents,
                    "installed_at": entry.installed_at.to_rfc3339()
                });
                Ok(serde_json::to_string_pretty(&details)?)
            } else if let Ok(entry) = registry.get_workflow(name_str) {
                let details = json!({
                    "type": "workflow",
                    "name": entry.name,
                    "description": entry.description,
                    "steps_count": entry.steps.len(),
                    "dependencies": entry.dependencies,
                    "installed_at": entry.installed_at.to_rfc3339()
                });
                Ok(serde_json::to_string_pretty(&details)?)
            } else {
                Err(anyhow::anyhow!(
                    "Capability '{}' not found in vault",
                    name_str
                ))
            }
        }
        "set_capability_env" => {
            let name_str = args
                .get("name")
                .and_then(|v| v.as_str())
                .context("Missing name parameter")?;
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .context("Missing key parameter")?;
            let value = args
                .get("value")
                .and_then(|v| v.as_str())
                .context("Missing value parameter")?;

            registry.update_mcp_env(name_str, key, value)?;
            Ok(format!(
                "Successfully set env var '{}' on '{}'",
                key, name_str
            ))
        }
        "search_registry" => {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .context("Missing query parameter")?;

            let search_engine = SearchEngine::new(registry.clone());
            let results = search_engine.search_npm(query).await?;
            Ok(serde_json::to_string_pretty(&results)?)
        }
        "doctor_check" => {
            let mcps = registry.list_mcps()?;
            let skills = registry.list_skills()?;
            let workflows = registry.list_workflows()?;

            let report = json!({
                "vault_version": env!("CARGO_PKG_VERSION"),
                "installed_mcps": mcps.len(),
                "installed_skills": skills.len(),
                "installed_workflows": workflows.len(),
                "status": "healthy"
            });
            Ok(serde_json::to_string_pretty(&report)?)
        }
        _ => Err(anyhow::anyhow!("Tool not found: {}", name)),
    }
}
