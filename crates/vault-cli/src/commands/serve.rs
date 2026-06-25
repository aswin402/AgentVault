use crate::cli::ServeArgs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vault_core::config::resolve_vault_dir;
use vault_core::gateway::child::ChildMcpServer;
use vault_core::gateway::registry::GatewayRegistry;
use vault_core::mcp::manager::{DefaultMcpManager, McpManager};
use vault_core::mcp::models::{McpSource, McpStatus, McpTransport};
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

pub async fn handle(args: ServeArgs, vault_dir_override: Option<&str>) -> Result<()> {
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

    // In gateway mode, spawn all installed Active/Stdio MCP servers.
    let gateway = if args.gateway {
        let gw = Arc::new(GatewayRegistry::new());
        let mcps = registry.list_mcps()?;

        for mcp in &mcps {
            if mcp.status != McpStatus::Active {
                continue;
            }
            if !matches!(mcp.transport, McpTransport::Stdio) {
                continue;
            }
            if mcp.command.is_empty() {
                continue;
            }

            match ChildMcpServer::spawn(&mcp.name, &mcp.command, &mcp.args, &mcp.env_vars).await {
                Ok(mut child) => {
                    if let Err(e) = child.initialize().await {
                        eprintln!("[gateway] Failed to initialize '{}': {}", mcp.name, e);
                        let _ = child.shutdown().await;
                        continue;
                    }
                    if let Err(e) = child.list_tools().await {
                        eprintln!("[gateway] Failed to list tools for '{}': {}", mcp.name, e);
                        let _ = child.shutdown().await;
                        continue;
                    }

                    match gw.register_child(&mcp.name, child).await {
                        Ok(names) => {
                            eprintln!(
                                "[gateway] Registered '{}' with {} tools",
                                mcp.name,
                                names.len()
                            );
                        }
                        Err(e) => {
                            eprintln!("[gateway] Failed to register '{}': {}", mcp.name, e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[gateway] Failed to spawn '{}': {}", mcp.name, e);
                }
            }
        }

        Some(gw)
    } else {
        None
    };

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
            gateway.clone(),
            &mut writer,
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

    // Graceful shutdown: kill all child servers on stdin EOF.
    if let Some(gw) = &gateway {
        eprintln!("[gateway] Shutting down all child servers...");
        let _ = gw.shutdown_all().await;
    }

    Ok(())
}

/// Send a JSON-RPC notification to the client (fire-and-forget).
async fn send_notification(writer: &mut tokio::io::Stdout, method: &str) -> Result<()> {
    let notification = json!({
        "jsonrpc": "2.0",
        "method": method
    });
    let msg = serde_json::to_string(&notification)? + "\n";
    writer.write_all(msg.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_request(
    req: JsonRpcRequest,
    registry: Arc<SqliteRegistry>,
    mcp_manager: Arc<DefaultMcpManager>,
    skill_manager: Arc<DefaultSkillManager>,
    workflow_manager: Arc<DefaultWorkflowManager>,
    gateway: Option<Arc<GatewayRegistry>>,
    writer: &mut tokio::io::Stdout,
) -> Option<JsonRpcResponse> {
    let method = req.method.as_str();
    let is_gateway = gateway.is_some();

    match method {
        "initialize" => {
            let server_name = if is_gateway {
                "agentvault-gateway"
            } else {
                "agentvault"
            };
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": server_name,
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
            let mut tools = build_management_tools();

            // In gateway mode, merge in all child server tools.
            if let Some(gw) = &gateway {
                let child_tools = gw.get_merged_tools().await;
                tools.extend(child_tools);
            }

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

            // In gateway mode, check if this is a child server tool call.
            if let Some(gw) = &gateway {
                // Check if the tool belongs to a child server (has __ separator
                // and is NOT a vault management tool).
                if vault_core::gateway::registry::parse_namespaced(name).is_some()
                    && !name.starts_with("vault__")
                {
                    let call_res = gw.route_tool_call(name, arguments).await;
                    return match call_res {
                        Ok(result) => Some(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(result),
                            error: None,
                        }),
                        Err(e) => Some(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: None,
                            error: Some(json!({
                                "code": -32603,
                                "message": format!("Tool execution failed: {}", e)
                            })),
                        }),
                    };
                }
            }

            let call_res = execute_tool_call(
                name,
                arguments,
                registry.clone(),
                mcp_manager.clone(),
                skill_manager.clone(),
                workflow_manager.clone(),
                gateway.clone(),
                writer,
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

/// Build the list of vault management tool definitions.
fn build_management_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "list_capabilities",
            "description": "List all installed capabilities in AgentVault (MCPs, Skills, and Workflows).",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "install_capability",
            "description": "Install a new capability (MCP, Skill, or Workflow) into the vault. In gateway mode, newly installed MCP servers are automatically spawned and their tools become available immediately.",
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
        }),
        json!({
            "name": "remove_capability",
            "description": "Remove a capability from the vault. In gateway mode, the MCP server is shut down and its tools are removed immediately.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "The name of the capability to remove" }
                },
                "required": ["name"]
            }
        }),
        json!({
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
        }),
        json!({
            "name": "get_capability_details",
            "description": "Get full metadata for a specific installed capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "The name of the capability to get details for" }
                },
                "required": ["name"]
            }
        }),
        json!({
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
        }),
        json!({
            "name": "search_registry",
            "description": "Search for MCP servers available to install.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query string" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "doctor_check",
            "description": "Run diagnostic health checks on the vault.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
    ]
}

#[allow(clippy::too_many_arguments)]
async fn execute_tool_call(
    name: &str,
    args: Value,
    registry: Arc<SqliteRegistry>,
    mcp_manager: Arc<DefaultMcpManager>,
    skill_manager: Arc<DefaultSkillManager>,
    workflow_manager: Arc<DefaultWorkflowManager>,
    gateway: Option<Arc<GatewayRegistry>>,
    writer: &mut tokio::io::Stdout,
) -> Result<String> {
    match name {
        "list_capabilities" => {
            let mcps = registry.list_mcps()?;
            let skills = registry.list_skills()?;
            let workflows = registry.list_workflows()?;

            let mut summary = json!({
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

            // In gateway mode, also show which child servers are running.
            if let Some(gw) = &gateway {
                let children = gw.get_child_names().await;
                summary
                    .as_object_mut()
                    .unwrap()
                    .insert("gateway_active_servers".to_string(), json!(children));
            }

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
                // Notify client that tool list has changed.
                let _ = send_notification(writer, "notifications/tools/list_changed").await;
                Ok(format!("Successfully installed skill '{}'", entry.name))
            } else if is_workflow {
                let path = std::path::PathBuf::from(
                    source_str.strip_prefix("local:").unwrap_or(source_str),
                );
                let entry = workflow_manager
                    .install(vault_core::workflow::manager::WorkflowSource::Local { path })
                    .await?;
                let _ = send_notification(writer, "notifications/tools/list_changed").await;
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

                // In gateway mode, spawn the newly installed MCP and register it.
                if let Some(gw) = &gateway {
                    if !entry.command.is_empty() && matches!(entry.transport, McpTransport::Stdio) {
                        match ChildMcpServer::spawn(
                            &entry.name,
                            &entry.command,
                            &entry.args,
                            &entry.env_vars,
                        )
                        .await
                        {
                            Ok(mut child) => {
                                if child.initialize().await.is_ok()
                                    && child.list_tools().await.is_ok()
                                {
                                    let _ = gw.register_child(&entry.name, child).await;
                                }
                            }
                            Err(e) => {
                                eprintln!("[gateway] Could not auto-spawn '{}': {}", entry.name, e);
                            }
                        }
                    }
                }

                // Notify client that tool list has changed.
                let _ = send_notification(writer, "notifications/tools/list_changed").await;

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

            // In gateway mode, unregister the child server first.
            if let Some(gw) = &gateway {
                let _ = gw.unregister_child(name_str).await;
            }

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
                // Notify client that tool list has changed.
                let _ = send_notification(writer, "notifications/tools/list_changed").await;
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

            // In gateway mode, restart the child server with the updated version.
            if let Some(gw) = &gateway {
                // Unregister old instance (if running).
                let _ = gw.unregister_child(name_str).await;

                // Spawn the updated version.
                if !entry.command.is_empty() && matches!(entry.transport, McpTransport::Stdio) {
                    if let Ok(mut child) = ChildMcpServer::spawn(
                        &entry.name,
                        &entry.command,
                        &entry.args,
                        &entry.env_vars,
                    )
                    .await
                    {
                        if child.initialize().await.is_ok() && child.list_tools().await.is_ok() {
                            let _ = gw.register_child(&entry.name, child).await;
                        }
                    }
                }

                let _ = send_notification(writer, "notifications/tools/list_changed").await;
            }

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

            let mut report = json!({
                "vault_version": env!("CARGO_PKG_VERSION"),
                "installed_mcps": mcps.len(),
                "installed_skills": skills.len(),
                "installed_workflows": workflows.len(),
                "status": "healthy"
            });

            // In gateway mode, include child server status.
            if let Some(gw) = &gateway {
                let children = gw.get_child_names().await;
                report
                    .as_object_mut()
                    .unwrap()
                    .insert("gateway_active_servers".to_string(), json!(children));
            }

            Ok(serde_json::to_string_pretty(&report)?)
        }
        _ => Err(anyhow::anyhow!("Tool not found: {}", name)),
    }
}
