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
