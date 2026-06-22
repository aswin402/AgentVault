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
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use vault_core::mcp::models::{McpEntry, McpSource, McpStatus, McpTransport};

    fn create_test_mcp_entry(name: &str, command: &str, args: Vec<String>) -> McpEntry {
        McpEntry {
            id: "test-id".to_string(),
            name: name.to_string(),
            display_name: None,
            version: "1.0.0".to_string(),
            source: McpSource::Npm {
                package: "test-package".to_string(),
            },
            install_path: PathBuf::from("/tmp"),
            command: command.to_string(),
            args,
            env_vars: HashMap::new(),
            transport: McpTransport::Stdio,
            status: McpStatus::Active,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            checksum: None,
            agents: vec![],
            tags: vec![],
            description: None,
        }
    }

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
        config
            .mcp_servers
            .insert("test-server".to_string(), server_config);

        connector.write_config(&config).await.unwrap();

        let reloaded = connector.read_config().await.unwrap();
        assert_eq!(reloaded.mcp_servers.len(), 1);
        assert_eq!(
            reloaded.mcp_servers.get("test-server").unwrap().command,
            "node"
        );
    }

    #[tokio::test]
    async fn test_claude_connector_diff() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("claude_desktop_config.json");
        let backup_dir = temp.path().join("backups");
        let connector = ClaudeConnector::new_with_paths(config_path, backup_dir);

        // 1. Diff against empty config: Addition
        let entry = create_test_mcp_entry("server-a", "node", vec!["a.js".to_string()]);
        let diff = connector.diff(std::slice::from_ref(&entry)).await.unwrap();
        assert_eq!(diff.additions.len(), 1);
        assert_eq!(diff.additions[0].name, "server-a");
        assert!(diff.updates.is_empty());
        assert!(diff.removals.is_empty());

        // Write the entry to config
        connector.sync(std::slice::from_ref(&entry)).await.unwrap();

        // 2. Diff with no changes: should be empty
        let diff_empty = connector.diff(std::slice::from_ref(&entry)).await.unwrap();
        assert!(diff_empty.is_empty());

        // 3. Diff with changed fields: Update
        let updated_entry = create_test_mcp_entry("server-a", "deno", vec!["b.js".to_string()]);
        let diff_update = connector.diff(&[updated_entry]).await.unwrap();
        assert!(diff_update.additions.is_empty());
        assert_eq!(diff_update.updates.len(), 1);
        assert_eq!(diff_update.updates[0].name, "server-a");
        assert_eq!(diff_update.updates[0].changed_fields.len(), 2);
        assert!(diff_update.removals.is_empty());

        // 4. Diff with missing entry: Removal
        let diff_removal = connector.diff(&[]).await.unwrap();
        assert!(diff_removal.additions.is_empty());
        assert!(diff_removal.updates.is_empty());
        assert_eq!(diff_removal.removals.len(), 1);
        assert_eq!(diff_removal.removals[0].name, "server-a");
    }

    #[tokio::test]
    async fn test_claude_connector_sync_success() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("claude_desktop_config.json");
        let backup_dir = temp.path().join("backups");
        let connector = ClaudeConnector::new_with_paths(config_path.clone(), backup_dir);

        let entry_a = create_test_mcp_entry("server-a", "node", vec![]);
        let entry_b = create_test_mcp_entry("server-b", "python", vec![]);

        let res = connector
            .sync(&[entry_a.clone(), entry_b.clone()])
            .await
            .unwrap();
        assert!(res.success);
        assert_eq!(res.diff.additions.len(), 2);

        let config = connector.read_config().await.unwrap();
        assert_eq!(config.mcp_servers.len(), 2);
        assert!(config.mcp_servers.contains_key("server-a"));
        assert!(config.mcp_servers.contains_key("server-b"));

        let entry_a_updated = create_test_mcp_entry("server-a", "node-new", vec![]);
        let res2 = connector.sync(&[entry_a_updated]).await.unwrap();
        assert!(res2.success);
        assert_eq!(res2.diff.updates.len(), 1);
        assert_eq!(res2.diff.removals.len(), 1);

        let config2 = connector.read_config().await.unwrap();
        assert_eq!(config2.mcp_servers.len(), 1);
        assert_eq!(
            config2.mcp_servers.get("server-a").unwrap().command,
            "node-new"
        );
    }

    #[tokio::test]
    async fn test_claude_connector_backup() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("claude_desktop_config.json");
        let backup_dir = temp.path().join("backups");
        let connector = ClaudeConnector::new_with_paths(config_path.clone(), backup_dir.clone());

        let bp_empty = connector.backup().unwrap();
        assert_eq!(bp_empty, PathBuf::new());

        let entry = create_test_mcp_entry("server-a", "node", vec![]);
        connector.sync(&[entry]).await.unwrap();

        let bp = connector.backup().unwrap();
        assert!(bp.exists());
        assert!(bp.starts_with(&backup_dir));

        let orig_content = std::fs::read_to_string(&config_path).unwrap();
        let backup_content = std::fs::read_to_string(&bp).unwrap();
        assert_eq!(orig_content, backup_content);
    }

    #[tokio::test]
    async fn test_claude_connector_verify_and_rollback() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("claude_desktop_config.json");
        let backup_dir = temp.path().join("backups");
        let connector = ClaudeConnector::new_with_paths(config_path.clone(), backup_dir);

        assert!(!connector.verify().unwrap());

        let entry = create_test_mcp_entry("server-a", "node", vec![]);
        connector.sync(&[entry]).await.unwrap();
        assert!(connector.verify().unwrap());

        let bp = connector.backup().unwrap();

        std::fs::write(&config_path, "invalid json content").unwrap();
        assert!(!connector.verify().unwrap());

        std::fs::copy(&bp, &config_path).unwrap();
        assert!(connector.verify().unwrap());
        let config = connector.read_config().await.unwrap();
        assert!(config.mcp_servers.contains_key("server-a"));
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
        config
            .mcp_servers
            .insert("test-server".to_string(), server_config);

        connector.write_config(&config).await.unwrap();

        let reloaded = connector.read_config().await.unwrap();
        assert_eq!(reloaded.mcp_servers.len(), 1);
        assert_eq!(
            reloaded.mcp_servers.get("test-server").unwrap().command,
            "node"
        );
    }

    #[tokio::test]
    async fn test_gemini_connector_diff() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let backup_dir = temp.path().join("backups");
        let connector = GeminiConnector::new_with_paths(config_path, backup_dir);

        // 1. Diff against empty config: Addition
        let entry = create_test_mcp_entry("server-a", "node", vec!["a.js".to_string()]);
        let diff = connector.diff(std::slice::from_ref(&entry)).await.unwrap();
        assert_eq!(diff.additions.len(), 1);
        assert_eq!(diff.additions[0].name, "server-a");
        assert!(diff.updates.is_empty());
        assert!(diff.removals.is_empty());

        // Write the entry to config
        connector.sync(std::slice::from_ref(&entry)).await.unwrap();

        // 2. Diff with no changes: should be empty
        let diff_empty = connector.diff(std::slice::from_ref(&entry)).await.unwrap();
        assert!(diff_empty.is_empty());

        // 3. Diff with changed fields: Update
        let updated_entry = create_test_mcp_entry("server-a", "deno", vec!["b.js".to_string()]);
        let diff_update = connector.diff(&[updated_entry]).await.unwrap();
        assert!(diff_update.additions.is_empty());
        assert_eq!(diff_update.updates.len(), 1);
        assert_eq!(diff_update.updates[0].name, "server-a");
        assert_eq!(diff_update.updates[0].changed_fields.len(), 2);
        assert!(diff_update.removals.is_empty());

        // 4. Diff with missing entry: Removal
        let diff_removal = connector.diff(&[]).await.unwrap();
        assert!(diff_removal.additions.is_empty());
        assert!(diff_removal.updates.is_empty());
        assert_eq!(diff_removal.removals.len(), 1);
        assert_eq!(diff_removal.removals[0].name, "server-a");
    }

    #[tokio::test]
    async fn test_gemini_connector_sync_success() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let backup_dir = temp.path().join("backups");
        let connector = GeminiConnector::new_with_paths(config_path.clone(), backup_dir);

        let entry_a = create_test_mcp_entry("server-a", "node", vec![]);
        let entry_b = create_test_mcp_entry("server-b", "python", vec![]);

        let res = connector
            .sync(&[entry_a.clone(), entry_b.clone()])
            .await
            .unwrap();
        assert!(res.success);
        assert_eq!(res.diff.additions.len(), 2);

        let config = connector.read_config().await.unwrap();
        assert_eq!(config.mcp_servers.len(), 2);
        assert!(config.mcp_servers.contains_key("server-a"));
        assert!(config.mcp_servers.contains_key("server-b"));

        let entry_a_updated = create_test_mcp_entry("server-a", "node-new", vec![]);
        let res2 = connector.sync(&[entry_a_updated]).await.unwrap();
        assert!(res2.success);
        assert_eq!(res2.diff.updates.len(), 1);
        assert_eq!(res2.diff.removals.len(), 1);

        let config2 = connector.read_config().await.unwrap();
        assert_eq!(config2.mcp_servers.len(), 1);
        assert_eq!(
            config2.mcp_servers.get("server-a").unwrap().command,
            "node-new"
        );
    }

    #[tokio::test]
    async fn test_gemini_connector_backup() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let backup_dir = temp.path().join("backups");
        let connector = GeminiConnector::new_with_paths(config_path.clone(), backup_dir.clone());

        let bp_empty = connector.backup().unwrap();
        assert_eq!(bp_empty, PathBuf::new());

        let entry = create_test_mcp_entry("server-a", "node", vec![]);
        connector.sync(&[entry]).await.unwrap();

        let bp = connector.backup().unwrap();
        assert!(bp.exists());
        assert!(bp.starts_with(&backup_dir));

        let orig_content = std::fs::read_to_string(&config_path).unwrap();
        let backup_content = std::fs::read_to_string(&bp).unwrap();
        assert_eq!(orig_content, backup_content);
    }

    #[tokio::test]
    async fn test_gemini_connector_verify_and_rollback() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("settings.json");
        let backup_dir = temp.path().join("backups");
        let connector = GeminiConnector::new_with_paths(config_path.clone(), backup_dir);

        assert!(!connector.verify().unwrap());

        let entry = create_test_mcp_entry("server-a", "node", vec![]);
        connector.sync(&[entry]).await.unwrap();
        assert!(connector.verify().unwrap());

        let bp = connector.backup().unwrap();

        std::fs::write(&config_path, "invalid json content").unwrap();
        assert!(!connector.verify().unwrap());

        std::fs::copy(&bp, &config_path).unwrap();
        assert!(connector.verify().unwrap());
        let config = connector.read_config().await.unwrap();
        assert!(config.mcp_servers.contains_key("server-a"));
    }
}
