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

        let fetched = manager.get("my-local-mcp").unwrap();
        assert_eq!(fetched.name, "my-local-mcp");
        assert_eq!(fetched.tags, vec!["tag1".to_string()]);
    }

    #[tokio::test]
    async fn test_mcp_manager_install_npm() {
        let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };
        if std::process::Command::new(npm_cmd)
            .arg("--version")
            .output()
            .is_err()
        {
            println!("npm is not installed, skipping test");
            return;
        }

        let temp_db_dir = tempdir().unwrap();
        let db_path = temp_db_dir.path().join("vault.db");
        let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
        let temp_vault_dir = tempdir().unwrap();
        let manager = DefaultMcpManager::new(registry.clone(), temp_vault_dir.path().to_path_buf());

        // Use a known small package for testing
        let source = McpSource::Npm {
            package: "is-number".to_string(),
        };
        let entry = manager
            .install(
                "is-number-mcp",
                source,
                "latest",
                vec![],
                std::collections::HashMap::new(),
                vec![],
                vec!["npm-tag".to_string()],
                None,
            )
            .await
            .unwrap();

        assert_eq!(entry.name, "is-number-mcp");
        assert!(temp_vault_dir
            .path()
            .join("mcps")
            .join("is-number-mcp")
            .join("package.json")
            .exists());

        let fetched = manager.get("is-number-mcp").unwrap();
        assert_eq!(fetched.name, "is-number-mcp");
        assert_eq!(fetched.tags, vec!["npm-tag".to_string()]);
    }

    #[tokio::test]
    async fn test_mcp_manager_install_pypi() {
        let has_uv = std::process::Command::new("uv")
            .arg("--version")
            .output()
            .is_ok();
        let has_python3 = std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_ok();
        let has_python = std::process::Command::new("python")
            .arg("--version")
            .output()
            .is_ok();

        if !has_uv && !has_python3 && !has_python {
            println!("Neither uv nor python is installed, skipping test");
            return;
        }

        let temp_db_dir = tempdir().unwrap();
        let db_path = temp_db_dir.path().join("vault.db");
        let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
        let temp_vault_dir = tempdir().unwrap();
        let manager = DefaultMcpManager::new(registry.clone(), temp_vault_dir.path().to_path_buf());

        // Use a known small package for testing
        let source = McpSource::PyPi {
            package: "six".to_string(),
        };
        let entry = manager
            .install(
                "six-mcp",
                source,
                "latest",
                vec![],
                std::collections::HashMap::new(),
                vec![],
                vec!["pypi-tag".to_string()],
                None,
            )
            .await
            .unwrap();

        assert_eq!(entry.name, "six-mcp");
        assert!(temp_vault_dir
            .path()
            .join("mcps")
            .join("six-mcp")
            .join("venv")
            .exists());

        let fetched = manager.get("six-mcp").unwrap();
        assert_eq!(fetched.name, "six-mcp");
        assert_eq!(fetched.tags, vec!["pypi-tag".to_string()]);
    }
}
