use crate::types::{AgentMcpConfig, SyncDiff, SyncEntry};
use serde_json::json;

#[test]
fn test_agent_mcp_config_serialization() {
    let json_val = json!({
        "command": "node",
        "args": ["index.js"],
        "env": { "PORT": "3000" }
    });
    let config: AgentMcpConfig = serde_json::from_value(json_val).unwrap();
    assert_eq!(config.command, "node");
    assert_eq!(config.args, vec!["index.js"]);
    assert_eq!(config.env.get("PORT").unwrap(), "3000");
}

#[test]
fn test_sync_diff_methods() {
    let mut diff = SyncDiff::default();
    assert!(diff.is_empty());
    assert_eq!(diff.change_count(), 0);

    diff.additions.push(SyncEntry {
        name: "test-mcp".to_string(),
        source: "registry".to_string(),
        version: "1.0.0".to_string(),
    });
    assert!(!diff.is_empty());
    assert_eq!(diff.change_count(), 1);
}

#[cfg(test)]
mod integration_tests {
    use crate::claude::ClaudeConnector;
    use crate::gemini::GeminiConnector;
    use crate::traits::AgentConnector;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_claude_connector_read_empty_config() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("claude_desktop_config.json");
        let backup_dir = temp.path().join("backups");
        
        let connector = ClaudeConnector::new_with_paths(config_path, backup_dir);
        let config = connector.read_config().await.unwrap();
        assert!(config.mcp_servers.is_empty());
    }

    #[tokio::test]
    async fn test_claude_connector_write_and_read() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("claude_desktop_config.json");
        let backup_dir = temp.path().join("backups");
        
        let connector = ClaudeConnector::new_with_paths(config_path, backup_dir);
        let mut config = connector.read_config().await.unwrap();
        
        let server_config = crate::types::AgentMcpConfig {
            command: "node".to_string(),
            args: vec!["app.js".to_string()],
            env: std::collections::HashMap::new(),
        };
        config.mcp_servers.insert("test-server".to_string(), server_config);
        
        connector.write_config(&config).await.unwrap();
        
        let reloaded = connector.read_config().await.unwrap();
        assert_eq!(reloaded.mcp_servers.len(), 1);
        assert_eq!(reloaded.mcp_servers.get("test-server").unwrap().command, "node");
    }

    #[tokio::test]
    async fn test_gemini_connector_read_empty_config() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let backup_dir = temp.path().join("backups");
        
        let connector = GeminiConnector::new_with_paths(config_path, backup_dir);
        let config = connector.read_config().await.unwrap();
        assert!(config.mcp_servers.is_empty());
    }

    #[tokio::test]
    async fn test_gemini_connector_write_and_read() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let backup_dir = temp.path().join("backups");
        
        let connector = GeminiConnector::new_with_paths(config_path, backup_dir);
        let mut config = connector.read_config().await.unwrap();
        
        let server_config = crate::types::AgentMcpConfig {
            command: "node".to_string(),
            args: vec!["app.js".to_string()],
            env: std::collections::HashMap::new(),
        };
        config.mcp_servers.insert("test-server".to_string(), server_config);
        
        connector.write_config(&config).await.unwrap();
        
        let reloaded = connector.read_config().await.unwrap();
        assert_eq!(reloaded.mcp_servers.len(), 1);
        assert_eq!(reloaded.mcp_servers.get("test-server").unwrap().command, "node");
    }
}
