#[cfg(test)]
mod tests {
    use crate::mcp::manager::{DefaultMcpManager, McpManager};
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
}
