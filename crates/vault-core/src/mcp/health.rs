use crate::mcp::models::McpEntry;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

pub async fn ping_mcp_server(mcp: &McpEntry, handshake_timeout: Duration) -> Result<(), String> {
    let mut cmd = Command::new(&mcp.command);
    cmd.args(&mcp.args);
    cmd.envs(&mcp.env_vars);
    cmd.stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn process: {}", e))?;

    let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    // JSON-RPC initialize payload
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "AgentVault-Doctor",
                "version": "0.2.1"
            }
        }
    });

    let req_str = serde_json::to_string(&req).map_err(|e| format!("Serialization error: {}", e))? + "\n";

    // Write request and wait for response
    let handshake_fut = async {
        stdin.write_all(req_str.as_bytes()).await.map_err(|e| format!("Write error: {}", e))?;
        stdin.flush().await.map_err(|e| format!("Flush error: {}", e))?;

        if let Some(line) = reader.next_line().await.map_err(|e| format!("Read error: {}", e))? {
            let res: serde_json::Value = serde_json::from_str(&line).map_err(|e| format!("JSON parse error ({}): {}", line, e))?;
            if res.get("error").is_some() {
                return Err(format!("Server returned error: {:?}", res.get("error")));
            }
            if res.get("result").is_some() || res.get("id").is_some() {
                return Ok(());
            }
            Err(format!("Invalid response format: {}", line))
        } else {
            Err("Server closed stdout connection without responding".to_string())
        }
    };

    let res = timeout(handshake_timeout, handshake_fut).await;

    // Force terminate child
    let _ = child.kill().await;

    match res {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Connection timed out".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::models::{McpEntry, McpSource, McpTransport, McpStatus};
    use std::collections::HashMap;
    use std::time::Duration;

    #[tokio::test]
    async fn test_ping_mcp_server_success() {
        let mcp = McpEntry {
            id: "test".to_string(),
            name: "mock".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Local { path: std::path::PathBuf::new() },
            install_path: std::path::PathBuf::new(),
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "read line; echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}'".to_string()],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec![],
            description: None,
        };

        let result = ping_mcp_server(&mcp, Duration::from_millis(500)).await;
        assert!(result.is_ok(), "Mock ping failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_ping_mcp_server_timeout() {
        let mcp = McpEntry {
            id: "test".to_string(),
            name: "mock".to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Local { path: std::path::PathBuf::new() },
            install_path: std::path::PathBuf::new(),
            command: "sleep".to_string(),
            args: vec!["10".to_string()],
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec![],
            description: None,
        };

        let result = ping_mcp_server(&mcp, Duration::from_millis(100)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Connection timed out");
    }
}
