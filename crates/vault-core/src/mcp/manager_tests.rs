#[cfg(test)]
mod tests {
    use crate::mcp::manager::{DefaultMcpManager, McpManager};
    use crate::mcp::models::McpSource;
    use crate::registry::SqliteRegistry;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_mcp_manager_get_and_list_empty() {
        let temp_db_dir = tempdir().unwrap();
        let db_path = temp_db_dir.path().join("vault.db");
        let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());

        let temp_vault_dir = tempdir().unwrap();
        let manager = DefaultMcpManager::new(registry, temp_vault_dir.path().to_path_buf());

        let list = manager.list().unwrap();
        assert!(list.is_empty());

        let get_res = manager.get("nonexistent");
        assert!(get_res.is_err());
    }

    #[tokio::test]
    async fn test_mcp_manager_install_local() {
        let temp_db_dir = tempdir().unwrap();
        let db_path = temp_db_dir.path().join("vault.db");
        let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
        let temp_vault_dir = tempdir().unwrap();
        let manager = DefaultMcpManager::new(registry.clone(), temp_vault_dir.path().to_path_buf());

        // Create dummy local path
        let local_dir = tempdir().unwrap();
        let script_path = local_dir.path().join("mcp_server.sh");
        std::fs::write(&script_path, "#!/bin/sh\necho 'running'").unwrap();

        let source = McpSource::Local {
            path: local_dir.path().to_path_buf(),
        };
        let entry = manager
            .install(
                "my-local-mcp",
                source,
                "latest",
                vec![],
                std::collections::HashMap::new(),
                vec![],
                vec!["tag1".to_string()],
                Some("Local server description".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(entry.name, "my-local-mcp");
        assert!(temp_vault_dir
            .path()
            .join("mcps")
            .join("my-local-mcp")
            .exists());
    }
}
