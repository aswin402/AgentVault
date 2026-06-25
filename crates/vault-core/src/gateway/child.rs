use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

/// Status of a child MCP server process.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildStatus {
    Initializing,
    Ready,
    Crashed { error: String },
    Stopped,
}

/// Manages a single child MCP server process.
///
/// Communicates with the child over JSON-RPC via stdio (stdin/stdout).
/// Uses mutex-protected pipes for thread-safe serialized access and an
/// atomic counter for auto-incrementing request IDs.
pub struct ChildMcpServer {
    /// Human-readable name for this server.
    pub name: String,
    /// The child process handle.
    child: Child,
    /// Stdin pipe to the child process (mutex for serialized writes).
    stdin: Mutex<ChildStdin>,
    /// Stdout reader from the child process (mutex for serialized reads).
    stdout: Mutex<BufReader<ChildStdout>>,
    /// Cached tool definitions from this child.
    pub tools: Vec<Value>,
    /// Current status.
    pub status: ChildStatus,
    /// Last time a tool call was forwarded to this child.
    pub last_used: Mutex<Instant>,
    /// Auto-incrementing JSON-RPC request ID.
    next_request_id: AtomicU64,
}

impl ChildMcpServer {
    /// Spawn a new child MCP server process.
    ///
    /// The child is started with piped stdin/stdout for JSON-RPC communication,
    /// stderr redirected to null, and `kill_on_drop` enabled so the process is
    /// cleaned up automatically when the handle is dropped.
    pub async fn spawn(
        name: &str,
        command: &str,
        args: &[String],
        env_vars: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut child = tokio::process::Command::new(command)
            .args(args)
            .envs(env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("Failed to spawn child MCP server '{name}': {command}"))?;

        let stdin = child.stdin.take().expect("stdin should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");

        Ok(Self {
            name: name.to_string(),
            child,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            tools: vec![],
            status: ChildStatus::Initializing,
            last_used: Mutex::new(Instant::now()),
            next_request_id: AtomicU64::new(1),
        })
    }

    /// Perform the MCP initialize handshake with the child server.
    ///
    /// Sends an `initialize` request followed by a `notifications/initialized`
    /// notification. Sets the server status to `Ready` on success.
    pub async fn initialize(&mut self) -> Result<Value> {
        let result = self
            .send_request(
                "initialize",
                Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "agentvault-gateway",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
            )
            .await
            .context("MCP initialize handshake failed")?;

        self.send_notification("notifications/initialized")
            .await
            .context("Failed to send initialized notification")?;

        self.status = ChildStatus::Ready;
        Ok(result)
    }

    /// Request the list of available tools from the child server.
    ///
    /// Caches the result in `self.tools` and returns a clone.
    pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
        let result = self
            .send_request("tools/list", None)
            .await
            .context("Failed to list tools")?;

        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        self.tools = tools.clone();
        Ok(tools)
    }

    /// Forward a tool call to the child server.
    ///
    /// Updates `last_used` and sends a `tools/call` JSON-RPC request.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        {
            let mut last = self.last_used.lock().await;
            *last = Instant::now();
        }

        self.send_request(
            "tools/call",
            Some(json!({
                "name": tool_name,
                "arguments": arguments
            })),
        )
        .await
    }

    /// Shut down the child server process.
    ///
    /// Sets the status to `Stopped`, kills the process, and waits for it to exit.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.status = ChildStatus::Stopped;
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        Ok(())
    }

    /// Send a JSON-RPC request to the child and read the response.
    ///
    /// Generates an auto-incrementing ID, writes the request as a single line
    /// to stdin, and reads one line from stdout as the response.
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_request_id.fetch_add(1, Ordering::SeqCst);

        let mut request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method
        });

        if let Some(p) = params {
            request
                .as_object_mut()
                .expect("request is an object")
                .insert("params".to_string(), p);
        }

        let msg = serde_json::to_string(&request)? + "\n";

        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(msg.as_bytes())
                .await
                .context("Failed to write to child stdin")?;
            stdin.flush().await.context("Failed to flush child stdin")?;
        }

        loop {
            let mut line = String::new();
            {
                let mut stdout = self.stdout.lock().await;
                stdout
                    .read_line(&mut line)
                    .await
                    .context("Failed to read from child stdout")?;
            }

            if line.is_empty() {
                anyhow::bail!("Child server '{}' closed stdout connection", self.name);
            }

            let response: Value =
                serde_json::from_str(&line).context("Failed to parse JSON-RPC response")?;

            let is_response = response.get("method").is_none();
            if is_response {
                if let Some(resp_id) = response.get("id") {
                    if resp_id.as_u64() == Some(id) {
                        if let Some(error) = response.get("error") {
                            let message = error
                                .get("message")
                                .and_then(|m| m.as_str())
                                .unwrap_or("Unknown error");
                            anyhow::bail!("JSON-RPC error: {message}");
                        }
                        return Ok(response.get("result").cloned().unwrap_or(Value::Null));
                    } else {
                        eprintln!(
                            "[gateway] Ignored JSON-RPC response with mismatched ID (expected {id}, got {resp_id:?})"
                        );
                    }
                }
            } else {
                eprintln!(
                    "[gateway] Received notification/request from '{}': {}",
                    self.name, response
                );
            }
        }
    }

    /// Send a JSON-RPC notification to the child (fire-and-forget, no response).
    async fn send_notification(&self, method: &str) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method
        });

        let msg = serde_json::to_string(&notification)? + "\n";

        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(msg.as_bytes())
            .await
            .context("Failed to write notification to child stdin")?;
        stdin.flush().await.context("Failed to flush child stdin")?;

        Ok(())
    }
}
