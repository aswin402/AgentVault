# MCP Management Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the centralized MCP installation, removal, update, and listing logic (`McpManager` trait and implementation) along with the corresponding CLI subcommands (`vault install`, `vault remove`, `vault list`, `vault update`).

**Architecture:** Create an `McpManager` trait in `vault-core` and implement it in `DefaultMcpManager` using `Arc<dyn Registry>` and the filesystem vault directory path. Installers utilize system process invocations (`npm`, `uv`/`pip`, `git`) to build isolated execution directories for each MCP server, storing references inside the SQLite registry.

**Tech Stack:** Rust, SQLite, `std::process::Command`, `std::os::unix::fs::symlink`, `semver`, `tokio`.

## Global Constraints

- Code must compile warning-free with `cargo clippy --workspace --all-targets -- -D warnings`.
- Code formatting must fully comply with `cargo fmt --all -- --check`.
- Unit tests must be written for all non-trivial logic.

---

### Task 1: Define McpManager Trait & Registry Operations

**Files:**
- Create: `crates/vault-core/src/mcp/manager.rs`
- Modify: `crates/vault-core/src/mcp/mod.rs`
- Test: Create `crates/vault-core/src/mcp/manager_tests.rs`

**Interfaces:**
- Consumes: `Registry` trait from `crate::registry::Registry`
- Produces: `McpManager` trait and `DefaultMcpManager` implementation

- [ ] **Step 1: Write the failing test for get and list**
  Write a test in `crates/vault-core/src/mcp/manager_tests.rs`:
  ```rust
  #[cfg(test)]
  mod tests {
      use crate::mcp::manager::{DefaultMcpManager, McpManager};
      use crate::registry::SqliteRegistry;
      use tempfile::tempdir;
      use std::sync::Arc;

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
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test`
  Expected: Compile error because `DefaultMcpManager` and `McpManager` do not exist yet.

- [ ] **Step 3: Define McpManager and DefaultMcpManager**
  Create `crates/vault-core/src/mcp/manager.rs`:
  ```rust
  use crate::error::VaultError;
  use crate::mcp::models::{McpEntry, McpSource};
  use crate::registry::Registry;
  use async_trait::async_trait;
  use std::path::PathBuf;
  use std::sync::Arc;

  #[async_trait]
  pub trait McpManager: Send + Sync {
      async fn install(
          &self,
          name: &str,
          source: McpSource,
          version_req: &str,
          args: Vec<String>,
          env_vars: std::collections::HashMap<String, String>,
          agents: Vec<String>,
          tags: Vec<String>,
          description: Option<String>,
      ) -> Result<McpEntry, VaultError>;
      async fn remove(&self, name: &str, keep_files: bool) -> Result<(), VaultError>;
      async fn update(&self, name: &str, force: bool) -> Result<McpEntry, VaultError>;
      fn get(&self, name: &str) -> Result<McpEntry, VaultError>;
      fn list(&self) -> Result<Vec<McpEntry>, VaultError>;
  }

  pub struct DefaultMcpManager {
      registry: Arc<dyn Registry>,
      vault_dir: PathBuf,
  }

  impl DefaultMcpManager {
      pub fn new(registry: Arc<dyn Registry>, vault_dir: PathBuf) -> Self {
          Self { registry, vault_dir }
      }
  }

  #[async_trait]
  impl McpManager for DefaultMcpManager {
      async fn install(
          &self,
          _name: &str,
          _source: McpSource,
          _version_req: &str,
          _args: Vec<String>,
          _env_vars: std::collections::HashMap<String, String>,
          _agents: Vec<String>,
          _tags: Vec<String>,
          _description: Option<String>,
      ) -> Result<McpEntry, VaultError> {
          Err(VaultError::NotFound("Not implemented yet".to_string()))
      }

      async fn remove(&self, _name: &str, _keep_files: bool) -> Result<(), VaultError> {
          Err(VaultError::NotFound("Not implemented yet".to_string()))
      }

      async fn update(&self, _name: &str, _force: bool) -> Result<McpEntry, VaultError> {
          Err(VaultError::NotFound("Not implemented yet".to_string()))
      }

      fn get(&self, name: &str) -> Result<McpEntry, VaultError> {
          self.registry.get_mcp(name)
      }

      fn list(&self) -> Result<Vec<McpEntry>, VaultError> {
          self.registry.list_mcps()
      }
  }
  ```

- [ ] **Step 4: Update module declarations**
  Modify `crates/vault-core/src/mcp/mod.rs` to expose the manager:
  ```rust
  pub mod manager;
  pub mod models;
  ```
  And modify `crates/vault-core/src/lib.rs` to ensure tests are running.

- [ ] **Step 5: Run tests and verify they pass**
  Run: `cargo test`
  Expected: PASS

- [ ] **Step 6: Commit**
  ```bash
  git add crates/vault-core/src/mcp/mod.rs crates/vault-core/src/mcp/manager.rs
  git commit -m "feat: define McpManager trait and DefaultMcpManager skeleton"
  ```

---

### Task 2: Implement Local Path Installer

**Files:**
- Modify: `crates/vault-core/src/mcp/manager.rs`
- Test: Add test case in `crates/vault-core/src/mcp/manager_tests.rs`

- [ ] **Step 1: Write the failing test for Local path install**
  Add test case to `crates/vault-core/src/mcp/manager_tests.rs`:
  ```rust
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

      let source = McpSource::Local { path: local_dir.path().to_path_buf() };
      let entry = manager.install(
          "my-local-mcp",
          source,
          "latest",
          vec![],
          std::collections::HashMap::new(),
          vec![],
          vec!["tag1".to_string()],
          Some("Local server description".to_string()),
      ).await.unwrap();

      assert_eq!(entry.name, "my-local-mcp");
      assert!(temp_vault_dir.path().join("mcps").join("my-local-mcp").exists());
  }
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test`
  Expected: FAIL with "Not implemented yet"

- [ ] **Step 3: Implement Local path installation**
  Modify `crates/vault-core/src/mcp/manager.rs` to support `McpSource::Local` inside `install` (creating symlink and registering in db):
  ```rust
  // Inside install method:
  if let McpSource::Local { ref path } = source {
      if !path.exists() {
          return Err(VaultError::NotFound(format!("Local path does not exist: {}", path.display())));
      }
      let target_link = self.vault_dir.join("mcps").join(name);
      if let Some(parent) = target_link.parent() {
          std::fs::create_dir_all(parent)?;
      }
      if target_link.exists() {
          std::fs::remove_dir_all(&target_link)?;
      }
      
      #[cfg(unix)]
      std::os::unix::fs::symlink(path, &target_link)?;
      #[cfg(windows)]
      std::os::windows::fs::symlink_dir(path, &target_link)?;

      let entry = McpEntry {
          id: uuid::Uuid::new_v4().to_string(),
          name: name.to_string(),
          display_name: Some(name.to_string()),
          version: "1.0.0".to_string(), // Local defaults to 1.0.0 or parses package file if available
          source: source.clone(),
          install_path: target_link,
          command: "node".to_string(), // Local entry could define custom script runner, placeholder for now
          args: args.clone(),
          env_vars: env_vars.clone(),
          transport: crate::mcp::models::McpTransport::Stdio,
          status: crate::mcp::models::McpStatus::Active,
          installed_at: chrono::Utc::now(),
          updated_at: chrono::Utc::now(),
          checksum: None,
          agents: agents.clone(),
          tags: tags.clone(),
          description: description.clone(),
      };
      
      self.registry.insert_mcp(&entry)?;
      return Ok(entry);
  }
  ```

- [ ] **Step 4: Run tests and verify they pass**
  Run: `cargo test`
  Expected: PASS

- [ ] **Step 5: Commit**
  ```bash
  git add crates/vault-core/src/mcp/manager.rs
  git commit -m "feat: implement local path symlinking install in McpManager"
  ```

---

### Task 3: Implement NPM/NPX Installer

**Files:**
- Modify: `crates/vault-core/src/mcp/manager.rs`
- Test: Add test case in `crates/vault-core/src/mcp/manager_tests.rs`

- [ ] **Step 1: Write the failing test for NPM install**
  Add test case to `crates/vault-core/src/mcp/manager_tests.rs`:
  ```rust
  #[tokio::test]
  async fn test_mcp_manager_install_npm() {
      let temp_db_dir = tempdir().unwrap();
      let db_path = temp_db_dir.path().join("vault.db");
      let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
      let temp_vault_dir = tempdir().unwrap();
      let manager = DefaultMcpManager::new(registry.clone(), temp_vault_dir.path().to_path_buf());

      // Use a known small package for testing, or mock the npm installation logic
      let source = McpSource::Npm { package: "express".to_string() };
      let entry = manager.install(
          "express-mcp",
          source,
          "latest",
          vec![],
          std::collections::HashMap::new(),
          vec![],
          vec![],
          None,
      ).await.unwrap();

      assert_eq!(entry.name, "express-mcp");
      assert!(temp_vault_dir.path().join("mcps").join("express-mcp").join("package.json").exists());
  }
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test`
  Expected: FAIL with "Not implemented yet"

- [ ] **Step 3: Implement NPM package installation**
  Modify `crates/vault-core/src/mcp/manager.rs` to support `McpSource::Npm`:
  - Run `npm install --prefix <vault_dir>/mcps/<name> <package>` via process execution.
  - Parse package command endpoints.
  - Insert entry in SQLite registry database.

- [ ] **Step 4: Run tests and verify they pass**
  Run: `cargo test`
  Expected: PASS

- [ ] **Step 5: Commit**
  ```bash
  git add crates/vault-core/src/mcp/manager.rs
  git commit -m "feat: implement npm/npx installation logic in McpManager"
  ```

---

### Task 4: Implement PyPI Installer

**Files:**
- Modify: `crates/vault-core/src/mcp/manager.rs`
- Test: Add test case in `crates/vault-core/src/mcp/manager_tests.rs`

- [ ] **Step 1: Write the failing test for PyPI install**
  Add test case to `crates/vault-core/src/mcp/manager_tests.rs`:
  ```rust
  #[tokio::test]
  async fn test_mcp_manager_install_pypi() {
      let temp_db_dir = tempdir().unwrap();
      let db_path = temp_db_dir.path().join("vault.db");
      let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
      let temp_vault_dir = tempdir().unwrap();
      let manager = DefaultMcpManager::new(registry.clone(), temp_vault_dir.path().to_path_buf());

      // Dummy PyPI package name, logic mocks pip command
      let source = McpSource::PyPi { package: "requests".to_string() };
      let entry = manager.install(
          "requests-mcp",
          source,
          "latest",
          vec![],
          std::collections::HashMap::new(),
          vec![],
          vec![],
          None,
      ).await.unwrap();

      assert_eq!(entry.name, "requests-mcp");
      assert!(temp_vault_dir.path().join("mcps").join("requests-mcp").join("venv").exists());
  }
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test`
  Expected: FAIL with "Not implemented yet"

- [ ] **Step 3: Implement PyPI virtualenv installation**
  Modify `crates/vault-core/src/mcp/manager.rs` to support `McpSource::PyPi`:
  - Run `python3 -m venv <vault_dir>/mcps/<name>/venv` via process execution.
  - Run `<vault_dir>/mcps/<name>/venv/bin/pip install <package>` via process execution.
  - Insert entry in SQLite registry database.

- [ ] **Step 4: Run tests and verify they pass**
  Run: `cargo test`
  Expected: PASS

- [ ] **Step 5: Commit**
  ```bash
  git add crates/vault-core/src/mcp/manager.rs
  git commit -m "feat: implement pypi virtualenv installation logic in McpManager"
  ```

---

### Task 5: Implement removal, updates, listing and wiring into CLI commands

**Files:**
- Modify: `crates/vault-cli/src/commands/install.rs`
- Modify: `crates/vault-cli/src/commands/remove.rs`
- Modify: `crates/vault-cli/src/commands/list.rs`
- Modify: `crates/vault-cli/src/commands/update.rs`

- [ ] **Step 1: Wire install.rs CLI handler**
  Implement parsing of input strings into `McpSource` variants and call `McpManager::install`.

- [ ] **Step 2: Wire remove.rs CLI handler**
  Call `McpManager::remove` and delete files if requested.

- [ ] **Step 3: Wire list.rs CLI handler**
  Format results using table formatters.

- [ ] **Step 4: Wire update.rs CLI handler**
  Implement re-installation updates.

- [ ] **Step 5: Run integration validation**
  Run full build and run manual test suite to verify CLI functionality.

- [ ] **Step 6: Commit**
  ```bash
  git add crates/vault-cli/src/commands/
  git commit -m "feat: wire install, remove, list, update subcommands in CLI binary"
  ```
