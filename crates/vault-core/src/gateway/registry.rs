use super::child::ChildMcpServer;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Separator used for tool namespacing: `server_name__tool_name`.
const NAMESPACE_SEPARATOR: &str = "__";

/// Maps a namespaced tool name to its origin server and original tool name.
#[derive(Debug, Clone)]
pub struct ToolRoute {
    pub server_name: String,
    pub original_tool_name: String,
}

/// Central registry for the MCP gateway.
///
/// Tracks active child MCP servers and provides namespaced tool routing
/// so that tools from different servers never collide.
pub struct GatewayRegistry {
    /// Active child MCP servers, keyed by server name.
    children: Arc<RwLock<HashMap<String, Arc<Mutex<ChildMcpServer>>>>>,
    /// Tool routing table: namespaced_name -> route info.
    tool_routes: Arc<RwLock<HashMap<String, ToolRoute>>>,
}

impl Default for GatewayRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            children: Arc::new(RwLock::new(HashMap::new())),
            tool_routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a child MCP server and its tools.
    ///
    /// Each tool is namespaced as `server_name__tool_name`. Returns the list
    /// of namespaced tool names. Fails if any namespaced name would collide
    /// with an already-registered tool.
    pub async fn register_child(&self, name: &str, child: ChildMcpServer) -> Result<Vec<String>> {
        let tools = child.tools.clone();

        // Build the set of new routes, checking for collisions.
        let mut new_routes: Vec<(String, ToolRoute)> = Vec::new();
        let routes_read = self.tool_routes.read().await;

        for tool in &tools {
            let tool_name = tool
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow!("Tool missing 'name' field"))?;

            let namespaced = namespace_tool(name, tool_name);

            if routes_read.contains_key(&namespaced) {
                return Err(anyhow!(
                    "Tool name collision: '{namespaced}' is already registered"
                ));
            }

            new_routes.push((
                namespaced,
                ToolRoute {
                    server_name: name.to_string(),
                    original_tool_name: tool_name.to_string(),
                },
            ));
        }
        drop(routes_read);

        // No collisions — commit all routes.
        let mut routes_write = self.tool_routes.write().await;
        let namespaced_names: Vec<String> = new_routes.iter().map(|(n, _)| n.clone()).collect();

        for (namespaced, route) in new_routes {
            routes_write.insert(namespaced, route);
        }
        drop(routes_write);

        // Store the child.
        let mut children_write = self.children.write().await;
        children_write.insert(name.to_string(), Arc::new(Mutex::new(child)));

        Ok(namespaced_names)
    }

    /// Unregister a child server, removing all its tool routes and shutting it down.
    pub async fn unregister_child(&self, name: &str) -> Result<()> {
        let child = {
            let mut children_write = self.children.write().await;
            children_write
                .remove(name)
                .ok_or_else(|| anyhow!("No child server named '{name}'"))?
        };

        // Remove all routes belonging to this server.
        {
            let mut routes_write = self.tool_routes.write().await;
            routes_write.retain(|_, route| route.server_name != name);
        }

        // Shut down the child.
        let mut child_guard = child.lock().await;
        child_guard.shutdown().await?;

        Ok(())
    }

    /// Route a tool call to the appropriate child server by namespaced name.
    pub async fn route_tool_call(&self, namespaced_name: &str, arguments: Value) -> Result<Value> {
        let (server_name, original_tool_name) = {
            let routes = self.tool_routes.read().await;
            let route = routes
                .get(namespaced_name)
                .ok_or_else(|| anyhow!("Unknown tool: {namespaced_name}"))?;
            (route.server_name.clone(), route.original_tool_name.clone())
        };

        let child = {
            let children = self.children.read().await;
            children
                .get(&server_name)
                .ok_or_else(|| anyhow!("Child server '{server_name}' not found"))?
                .clone()
        };

        let child_guard = child.lock().await;
        child_guard.call_tool(&original_tool_name, arguments).await
    }

    /// Return the merged list of all tools across all children, with namespaced names.
    pub async fn get_merged_tools(&self) -> Vec<Value> {
        let children = self.children.read().await;
        let mut merged = Vec::new();

        for (name, child_arc) in children.iter() {
            let child = child_arc.lock().await;
            for tool in &child.tools {
                let mut tool = tool.clone();
                if let Some(obj) = tool.as_object_mut() {
                    if let Some(tool_name) = obj.get("name").and_then(|n| n.as_str()) {
                        let namespaced = namespace_tool(name, tool_name);
                        obj.insert("name".to_string(), Value::String(namespaced));
                    }
                }
                merged.push(tool);
            }
        }

        merged
    }

    /// Return the names of all registered child servers.
    pub async fn get_child_names(&self) -> Vec<String> {
        let children = self.children.read().await;
        children.keys().cloned().collect()
    }

    /// Shut down all child servers and clear the registry.
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut children = self.children.write().await;

        for (_, child_arc) in children.drain() {
            let mut child = child_arc.lock().await;
            child.shutdown().await?;
        }

        let mut routes = self.tool_routes.write().await;
        routes.clear();

        Ok(())
    }
}

/// Create a namespaced tool name: `server_name__tool_name`.
pub fn namespace_tool(server_name: &str, tool_name: &str) -> String {
    format!("{}{}{}", server_name, NAMESPACE_SEPARATOR, tool_name)
}

/// Parse a namespaced tool name into `(server_name, tool_name)`.
///
/// Splits on the first occurrence of `__`, so tool names containing `__`
/// are preserved correctly.
pub fn parse_namespaced(namespaced: &str) -> Option<(String, String)> {
    namespaced
        .split_once(NAMESPACE_SEPARATOR)
        .map(|(s, t)| (s.to_string(), t.to_string()))
}
