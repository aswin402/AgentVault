# Agent Connectors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement AgentVault's capability synchronization layer with Claude Code, Gemini CLI, OpenCode, and Codex CLI connectors, backup/verification safeguards, history auditing, and connector CLI management commands.

**Architecture:** Create an `AgentConnector` trait and unified types (`AgentConfig`, `AgentMcpConfig`, `SyncDiff`, `SyncResult`) under `vault-connectors`. Connectors read, parse, diff, merge, atomically write, and verify agent-specific configuration files. Create a `SyncEngine` to run diff, backup, write, verify, and log history to the SQLite database.

**Tech Stack:** Rust, SQLite (`rusqlite`), `serde_json`, `chrono`, `tokio`, `tempfile`.

## Global Constraints

- Code must compile warning-free with `cargo clippy --workspace --all-targets -- -D warnings`.
- Code formatting must fully comply with `cargo fmt --all -- --check`.
- Unit tests must be written for all non-trivial logic.

---

### Task 1: Define Connector Types & Traits

**Files:**
- Create: `crates/vault-connectors/src/types.rs`
- Create: `crates/vault-connectors/src/traits.rs`
- Modify: `crates/vault-connectors/src/lib.rs`
- Create: `crates/vault-connectors/src/tests.rs`

**Interfaces:**
- Consumes: `vault_core::agent::AgentType`, `vault_core::mcp::models::McpEntry`, `vault_core::error::VaultError`
- Produces: `AgentMcpConfig`, `AgentConfig`, `SyncDiff`, `SyncResult`, `SyncEntry`, `SyncUpdate`, `FieldChange` structures, and the `AgentConnector` trait.

- [ ] **Step 1: Write the failing tests for types serialization**
  Create `crates/vault-connectors/src/tests.rs` with:
  ```rust
  #[cfg(test)]
  mod tests {
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
  }
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test -p vault-connectors`
  Expected: Compile error because `types` module does not exist.

- [ ] **Step 3: Implement types and trait**
  Create `crates/vault-connectors/src/types.rs`:
  ```rust
  use serde::{Deserialize, Serialize};
  use serde_json::Value;
  use std::collections::HashMap;
  use chrono::{DateTime, Utc};

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct AgentMcpConfig {
      pub command: String,
      pub args: Vec<String>,
      #[serde(default)]
      pub env: HashMap<String, String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct AgentConfig {
      pub raw: Value,
      #[serde(default)]
      pub mcp_servers: HashMap<String, AgentMcpConfig>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct SyncEntry {
      pub name: String,
      pub source: String,
      pub version: String,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct FieldChange {
      pub field: String,
      pub old_value: String,
      pub new_value: String,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct SyncUpdate {
      pub name: String,
      pub changed_fields: Vec<FieldChange>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
  pub struct SyncDiff {
      pub additions: Vec<SyncEntry>,
      pub removals: Vec<SyncEntry>,
      pub updates: Vec<SyncUpdate>,
  }

  impl SyncDiff {
      pub fn is_empty(&self) -> bool {
          self.additions.is_empty() && self.removals.is_empty() && self.updates.is_empty()
      }

      pub fn change_count(&self) -> usize {
          self.additions.len() + self.removals.len() + self.updates.len()
      }
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct SyncResult {
      pub agent_type: String,
      pub timestamp: DateTime<Utc>,
      pub diff: SyncDiff,
      pub success: bool,
      pub backup_path: Option<String>,
      pub error: Option<String>,
  }
  ```

  Create `crates/vault-connectors/src/traits.rs`:
  ```rust
  use crate::types::{AgentConfig, SyncDiff, SyncResult};
  use async_trait::async_trait;
  use std::path::{Path, PathBuf};
  use vault_core::agent::AgentType;
  use vault_core::error::VaultError;
  use vault_core::mcp::models::McpEntry;

  #[async_trait]
  pub trait AgentConnector: Send + Sync {
      fn agent_type(&self) -> AgentType;
      fn config_path(&self) -> &Path;
      async fn read_config(&self) -> Result<AgentConfig, VaultError>;
      async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError>;
      async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError>;
      async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError>;
      fn backup(&self) -> Result<PathBuf, VaultError>;
      fn verify(&self) -> Result<bool, VaultError>;
  }
  ```

- [ ] **Step 4: Update lib.rs to re-export modules**
  Modify `crates/vault-connectors/src/lib.rs` to expose the types and traits:
  ```rust
  pub mod traits;
  pub mod types;

  #[cfg(test)]
  mod tests;
  ```

- [ ] **Step 5: Run tests to verify they pass**
  Run: `cargo test -p vault-connectors`
  Expected: PASS

- [ ] **Step 6: Commit**
  Run:
  ```bash
  git add crates/vault-connectors/src/types.rs crates/vault-connectors/src/traits.rs crates/vault-connectors/src/lib.rs crates/vault-connectors/src/tests.rs
  git commit -m "feat: define AgentConnector trait and sync structures"
  ```

---

### Task 2: Implement Claude Code & Gemini CLI Connectors

**Files:**
- Create: `crates/vault-connectors/src/claude.rs`
- Create: `crates/vault-connectors/src/gemini.rs`
- Modify: `crates/vault-connectors/src/lib.rs`
- Modify: `crates/vault-connectors/src/tests.rs`

**Interfaces:**
- Consumes: `AgentConnector` trait and serialization types from Task 1.
- Produces: `ClaudeConnector` and `GeminiConnector` structs implementing `AgentConnector`.

- [ ] **Step 1: Write failing tests for Claude Code reading and writing**
  Add tests inside `crates/vault-connectors/src/tests.rs`:
  ```rust
  use crate::claude::ClaudeConnector;
  use crate::traits::AgentConnector;
  use tempfile::tempdir;
  use std::fs;

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
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test -p vault-connectors`
  Expected: Compile error because `ClaudeConnector` is not defined.

- [ ] **Step 3: Implement ClaudeConnector**
  Create `crates/vault-connectors/src/claude.rs`:
  ```rust
  use crate::traits::AgentConnector;
  use crate::types::{AgentConfig, AgentMcpConfig, SyncDiff, SyncEntry, SyncResult, SyncUpdate, FieldChange};
  use async_trait::async_trait;
  use std::collections::{HashMap, HashSet};
  use std::path::{Path, PathBuf};
  use vault_core::agent::AgentType;
  use vault_core::error::VaultError;
  use vault_core::mcp::models::McpEntry;

  pub struct ClaudeConnector {
      config_path: PathBuf,
      backup_dir: PathBuf,
  }

  impl ClaudeConnector {
      pub fn new() -> Self {
          let home = dirs::home_dir().expect("Could not determine home directory");
          Self {
              config_path: home.join(".claude").join("claude_desktop_config.json"),
              backup_dir: home.join(".agentvault").join("backups").join("claude"),
          }
      }

      pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
          Self { config_path, backup_dir }
      }

      fn mcp_to_agent_config(entry: &McpEntry) -> AgentMcpConfig {
          AgentMcpConfig {
              command: entry.command.clone(),
              args: entry.args.clone(),
              env: entry.env_vars.clone(),
          }
      }
  }

  #[async_trait]
  impl AgentConnector for ClaudeConnector {
      fn agent_type(&self) -> AgentType {
          AgentType::ClaudeCode
      }

      fn config_path(&self) -> &Path {
          &self.config_path
      }

      async fn read_config(&self) -> Result<AgentConfig, VaultError> {
          if !self.config_path.exists() {
              return Ok(AgentConfig {
                  raw: serde_json::json!({}),
                  mcp_servers: HashMap::new(),
              });
          }

          let content = tokio::fs::read_to_string(&self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          let raw: serde_json::Value = serde_json::from_str(&content)
              .map_err(|e| VaultError::Config {
                  path: self.config_path.clone(),
                  message: format!("Invalid JSON: {}", e),
              })?;

          let mcp_servers = raw
              .get("mcpServers")
              .and_then(|v| v.as_object())
              .map(|obj| {
                  obj.iter()
                      .filter_map(|(name, value)| {
                          serde_json::from_value::<AgentMcpConfig>(value.clone())
                              .ok()
                              .map(|config| (name.clone(), config))
                      })
                      .collect()
              })
              .unwrap_or_default();

          Ok(AgentConfig { raw, mcp_servers })
      }

      async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError> {
          let mut raw = config.raw.clone();
          let mcp_obj = serde_json::to_value(&config.mcp_servers)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;
          raw["mcpServers"] = mcp_obj;

          let temp_path = self.config_path.with_extension("vault-tmp");
          let content = serde_json::to_string_pretty(&raw)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;

          if let Some(parent) = self.config_path.parent() {
              tokio::fs::create_dir_all(parent).await.map_err(|e| VaultError::Io {
                  path: parent.to_path_buf(),
                  source: e,
              })?;
          }

          tokio::fs::write(&temp_path, &content)
              .await
              .map_err(|e| VaultError::Io {
                  path: temp_path.clone(),
                  source: e,
              })?;

          tokio::fs::rename(&temp_path, &self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          Ok(())
      }

      async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError> {
          let config = self.read_config().await?;
          let mut diff = SyncDiff::default();

          // Additions & Updates
          for entry in entries {
              match config.mcp_servers.get(&entry.name) {
                  None => {
                      diff.additions.push(SyncEntry {
                          name: entry.name.clone(),
                          source: entry.source.to_string(),
                          version: entry.version.clone(),
                      });
                  }
                  Some(existing) => {
                      let mut changes = Vec::new();
                      if existing.command != entry.command {
                          changes.push(FieldChange {
                              field: "command".to_string(),
                              old_value: existing.command.clone(),
                              new_value: entry.command.clone(),
                          });
                      }
                      if existing.args != entry.args {
                          changes.push(FieldChange {
                              field: "args".to_string(),
                              old_value: format!("{:?}", existing.args),
                              new_value: format!("{:?}", entry.args),
                          });
                      }
                      if existing.env != entry.env_vars {
                          changes.push(FieldChange {
                              field: "env".to_string(),
                              old_value: format!("{:?}", existing.env),
                              new_value: format!("{:?}", entry.env_vars),
                          });
                      }
                      if !changes.is_empty() {
                          diff.updates.push(SyncUpdate {
                              name: entry.name.clone(),
                              changed_fields: changes,
                          });
                      }
                  }
              }
          }

          // Removals (vault-managed entries in agent but not in registry)
          // For now, if they are not in the entries list, and prune is active, we mark for removal.
          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for (name, _) in &config.mcp_servers {
              if !vault_names.contains(name.as_str()) {
                  diff.removals.push(SyncEntry {
                      name: name.clone(),
                      source: "vault-managed".to_string(),
                      version: "".to_string(),
                  });
              }
          }

          Ok(diff)
      }

      async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError> {
          let diff = self.diff(entries).await?;
          let timestamp = chrono::Utc::now();

          if diff.is_empty() {
              return Ok(SyncResult {
                  agent_type: self.agent_type().to_string(),
                  timestamp,
                  diff,
                  success: true,
                  backup_path: None,
                  error: None,
              });
          }

          let backup_path = if self.config_path.exists() {
              Some(self.backup()?)
          } else {
              None
          };

          let mut config = self.read_config().await?;

          // Apply additions and updates
          for entry in entries {
              config
                  .mcp_servers
                  .insert(entry.name.clone(), Self::mcp_to_agent_config(entry));
          }

          // Apply removals
          for removal in &diff.removals {
              config.mcp_servers.remove(&removal.name);
          }

          self.write_config(&config).await?;

          let valid = self.verify()?;
          if !valid {
              if let Some(ref bp) = backup_path {
                  std::fs::copy(bp, &self.config_path).map_err(|e| VaultError::Io {
                      path: self.config_path.clone(),
                      source: e,
                  })?;
              }
              return Err(VaultError::McpInstall {
                  source_type: self.agent_type().to_string(),
                  message: "Sync verification failed after writing config".to_string(),
              });
          }

          Ok(SyncResult {
              agent_type: self.agent_type().to_string(),
              timestamp,
              diff,
              success: true,
              backup_path: backup_path.map(|p| p.display().to_string()),
              error: None,
          })
      }

      fn backup(&self) -> Result<PathBuf, VaultError> {
          if !self.config_path.exists() {
              return Ok(PathBuf::new());
          }

          std::fs::create_dir_all(&self.backup_dir).map_err(|e| VaultError::Io {
              path: self.backup_dir.clone(),
              source: e,
          })?;

          let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
          let backup_path = self.backup_dir.join(format!("{}.json", timestamp));

          std::fs::copy(&self.config_path, &backup_path).map_err(|e| VaultError::Io {
              path: backup_path.clone(),
              source: e,
          })?;

          Ok(backup_path)
      }

      fn verify(&self) -> Result<bool, VaultError> {
          if !self.config_path.exists() {
              return Ok(false);
          }

          let content = std::fs::read_to_string(&self.config_path).map_err(|e| VaultError::Io {
              path: self.config_path.clone(),
              source: e,
          })?;

          match serde_json::from_str::<serde_json::Value>(&content) {
              Ok(v) => Ok(v.is_object()),
              Err(_) => Ok(false),
          }
      }
  }
  ```

- [ ] **Step 4: Implement GeminiConnector**
  Create `crates/vault-connectors/src/gemini.rs` (identical logic but target paths are `~/.gemini/config/settings.json` and backups go to `~/.agentvault/backups/gemini/`):
  ```rust
  use crate::traits::AgentConnector;
  use crate::types::{AgentConfig, AgentMcpConfig, SyncDiff, SyncEntry, SyncResult, SyncUpdate, FieldChange};
  use async_trait::async_trait;
  use std::collections::{HashMap, HashSet};
  use std::path::{Path, PathBuf};
  use vault_core::agent::AgentType;
  use vault_core::error::VaultError;
  use vault_core::mcp::models::McpEntry;

  pub struct GeminiConnector {
      config_path: PathBuf,
      backup_dir: PathBuf,
  }

  impl GeminiConnector {
      pub fn new() -> Self {
          let home = dirs::home_dir().expect("Could not determine home directory");
          Self {
              config_path: home.join(".gemini").join("config").join("settings.json"),
              backup_dir: home.join(".agentvault").join("backups").join("gemini"),
          }
      }

      pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
          Self { config_path, backup_dir }
      }

      fn mcp_to_agent_config(entry: &McpEntry) -> AgentMcpConfig {
          AgentMcpConfig {
              command: entry.command.clone(),
              args: entry.args.clone(),
              env: entry.env_vars.clone(),
          }
      }
  }

  #[async_trait]
  impl AgentConnector for GeminiConnector {
      fn agent_type(&self) -> AgentType {
          AgentType::GeminiCli
      }

      fn config_path(&self) -> &Path {
          &self.config_path
      }

      async fn read_config(&self) -> Result<AgentConfig, VaultError> {
          if !self.config_path.exists() {
              return Ok(AgentConfig {
                  raw: serde_json::json!({}),
                  mcp_servers: HashMap::new(),
              });
          }

          let content = tokio::fs::read_to_string(&self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          let raw: serde_json::Value = serde_json::from_str(&content)
              .map_err(|e| VaultError::Config {
                  path: self.config_path.clone(),
                  message: format!("Invalid JSON: {}", e),
              })?;

          let mcp_servers = raw
              .get("mcpServers")
              .and_then(|v| v.as_object())
              .map(|obj| {
                  obj.iter()
                      .filter_map(|(name, value)| {
                          serde_json::from_value::<AgentMcpConfig>(value.clone())
                              .ok()
                              .map(|config| (name.clone(), config))
                      })
                      .collect()
              })
              .unwrap_or_default();

          Ok(AgentConfig { raw, mcp_servers })
      }

      async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError> {
          let mut raw = config.raw.clone();
          let mcp_obj = serde_json::to_value(&config.mcp_servers)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;
          raw["mcpServers"] = mcp_obj;

          let temp_path = self.config_path.with_extension("vault-tmp");
          let content = serde_json::to_string_pretty(&raw)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;

          if let Some(parent) = self.config_path.parent() {
              tokio::fs::create_dir_all(parent).await.map_err(|e| VaultError::Io {
                  path: parent.to_path_buf(),
                  source: e,
              })?;
          }

          tokio::fs::write(&temp_path, &content)
              .await
              .map_err(|e| VaultError::Io {
                  path: temp_path.clone(),
                  source: e,
              })?;

          tokio::fs::rename(&temp_path, &self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          Ok(())
      }

      async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError> {
          let config = self.read_config().await?;
          let mut diff = SyncDiff::default();

          for entry in entries {
              match config.mcp_servers.get(&entry.name) {
                  None => {
                      diff.additions.push(SyncEntry {
                          name: entry.name.clone(),
                          source: entry.source.to_string(),
                          version: entry.version.clone(),
                      });
                  }
                  Some(existing) => {
                      let mut changes = Vec::new();
                      if existing.command != entry.command {
                          changes.push(FieldChange {
                              field: "command".to_string(),
                              old_value: existing.command.clone(),
                              new_value: entry.command.clone(),
                          });
                      }
                      if existing.args != entry.args {
                          changes.push(FieldChange {
                              field: "args".to_string(),
                              old_value: format!("{:?}", existing.args),
                              new_value: format!("{:?}", entry.args),
                          });
                      }
                      if existing.env != entry.env_vars {
                          changes.push(FieldChange {
                              field: "env".to_string(),
                              old_value: format!("{:?}", existing.env),
                              new_value: format!("{:?}", entry.env_vars),
                          });
                      }
                      if !changes.is_empty() {
                          diff.updates.push(SyncUpdate {
                              name: entry.name.clone(),
                              changed_fields: changes,
                          });
                      }
                  }
              }
          }

          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for (name, _) in &config.mcp_servers {
              if !vault_names.contains(name.as_str()) {
                  diff.removals.push(SyncEntry {
                      name: name.clone(),
                      source: "vault-managed".to_string(),
                      version: "".to_string(),
                  });
              }
          }

          Ok(diff)
      }

      async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError> {
          let diff = self.diff(entries).await?;
          let timestamp = chrono::Utc::now();

          if diff.is_empty() {
              return Ok(SyncResult {
                  agent_type: self.agent_type().to_string(),
                  timestamp,
                  diff,
                  success: true,
                  backup_path: None,
                  error: None,
              });
          }

          let backup_path = if self.config_path.exists() {
              Some(self.backup()?)
          } else {
              None
          };

          let mut config = self.read_config().await?;

          for entry in entries {
              config
                  .mcp_servers
                  .insert(entry.name.clone(), Self::mcp_to_agent_config(entry));
          }

          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for removal in &diff.removals {
              config.mcp_servers.remove(&removal.name);
          }

          self.write_config(&config).await?;

          let valid = self.verify()?;
          if !valid {
              if let Some(ref bp) = backup_path {
                  std::fs::copy(bp, &self.config_path).map_err(|e| VaultError::Io {
                      path: self.config_path.clone(),
                      source: e,
                  })?;
              }
              return Err(VaultError::McpInstall {
                  source_type: self.agent_type().to_string(),
                  message: "Sync verification failed after writing config".to_string(),
              });
          }

          Ok(SyncResult {
              agent_type: self.agent_type().to_string(),
              timestamp,
              diff,
              success: true,
              backup_path: backup_path.map(|p| p.display().to_string()),
              error: None,
          })
      }

      fn backup(&self) -> Result<PathBuf, VaultError> {
          if !self.config_path.exists() {
              return Ok(PathBuf::new());
          }

          std::fs::create_dir_all(&self.backup_dir).map_err(|e| VaultError::Io {
              path: self.backup_dir.clone(),
              source: e,
          })?;

          let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
          let backup_path = self.backup_dir.join(format!("{}.json", timestamp));

          std::fs::copy(&self.config_path, &backup_path).map_err(|e| VaultError::Io {
              path: backup_path.clone(),
              source: e,
          })?;

          Ok(backup_path)
      }

      fn verify(&self) -> Result<bool, VaultError> {
          if !self.config_path.exists() {
              return Ok(false);
          }

          let content = std::fs::read_to_string(&self.config_path).map_err(|e| VaultError::Io {
              path: self.config_path.clone(),
              source: e,
          })?;

          match serde_json::from_str::<serde_json::Value>(&content) {
              Ok(v) => Ok(v.is_object()),
              Err(_) => Ok(false),
          }
      }
  }
  ```

- [ ] **Step 5: Register modules in lib.rs**
  Modify `crates/vault-connectors/src/lib.rs`:
  ```rust
  pub mod traits;
  pub mod types;
  pub mod claude;
  pub mod gemini;

  #[cfg(test)]
  mod tests;
  ```

- [ ] **Step 6: Run tests to verify they pass**
  Run: `cargo test -p vault-connectors`
  Expected: PASS

- [ ] **Step 7: Commit**
  Run:
  ```bash
  git add crates/vault-connectors/src/claude.rs crates/vault-connectors/src/gemini.rs crates/vault-connectors/src/lib.rs crates/vault-connectors/src/tests.rs
  git commit -m "feat: implement Claude Code and Gemini CLI connectors"
  ```

---

### Task 3: Implement OpenCode & Codex CLI Connectors

**Files:**
- Create: `crates/vault-connectors/src/opencode.rs`
- Create: `crates/vault-connectors/src/codex.rs`
- Modify: `crates/vault-connectors/src/lib.rs`
- Modify: `crates/vault-connectors/src/tests.rs`

**Interfaces:**
- Consumes: `AgentConnector` trait and types from Task 1.
- Produces: `OpenCodeConnector` and `CodexConnector` structs implementing `AgentConnector`.

- [ ] **Step 1: Write failing tests for OpenCode & Codex paths**
  Add tests inside `crates/vault-connectors/src/tests.rs`:
  ```rust
  use crate::opencode::OpenCodeConnector;
  use crate::codex::CodexConnector;

  #[tokio::test]
  async fn test_opencode_connector_paths() {
      let temp = tempdir().unwrap();
      let config_path = temp.path().join("config.json");
      let backup_dir = temp.path().join("backups");
      
      let connector = OpenCodeConnector::new_with_paths(config_path.clone(), backup_dir);
      assert_eq!(connector.config_path(), config_path.as_path());
  }
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test -p vault-connectors`
  Expected: Compile error because `OpenCodeConnector` is not defined.

- [ ] **Step 3: Implement OpenCodeConnector**
  Create `crates/vault-connectors/src/opencode.rs` (identical schema and traits logic as Claude/Gemini, defaults to `~/.config/opencode/config.json` but checks `$XDG_CONFIG_HOME` on Unix):
  ```rust
  use crate::traits::AgentConnector;
  use crate::types::{AgentConfig, AgentMcpConfig, SyncDiff, SyncEntry, SyncResult, SyncUpdate, FieldChange};
  use async_trait::async_trait;
  use std::collections::{HashMap, HashSet};
  use std::path::{Path, PathBuf};
  use vault_core::agent::AgentType;
  use vault_core::error::VaultError;
  use vault_core::mcp::models::McpEntry;

  pub struct OpenCodeConnector {
      config_path: PathBuf,
      backup_dir: PathBuf,
  }

  impl OpenCodeConnector {
      pub fn new() -> Self {
          let home = dirs::home_dir().expect("Could not determine home directory");
          let config_dir = std::env::var("XDG_CONFIG_HOME")
              .map(PathBuf::from)
              .unwrap_or_else(|_| home.join(".config"));
          Self {
              config_path: config_dir.join("opencode").join("config.json"),
              backup_dir: home.join(".agentvault").join("backups").join("opencode"),
          }
      }

      pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
          Self { config_path, backup_dir }
      }

      fn mcp_to_agent_config(entry: &McpEntry) -> AgentMcpConfig {
          AgentMcpConfig {
              command: entry.command.clone(),
              args: entry.args.clone(),
              env: entry.env_vars.clone(),
          }
      }
  }

  #[async_trait]
  impl AgentConnector for OpenCodeConnector {
      fn agent_type(&self) -> AgentType {
          AgentType::OpenCode
      }

      fn config_path(&self) -> &Path {
          &self.config_path
      }

      async fn read_config(&self) -> Result<AgentConfig, VaultError> {
          if !self.config_path.exists() {
              return Ok(AgentConfig {
                  raw: serde_json::json!({}),
                  mcp_servers: HashMap::new(),
              });
          }

          let content = tokio::fs::read_to_string(&self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          let raw: serde_json::Value = serde_json::from_str(&content)
              .map_err(|e| VaultError::Config {
                  path: self.config_path.clone(),
                  message: format!("Invalid JSON: {}", e),
              })?;

          let mcp_servers = raw
              .get("mcpServers")
              .and_then(|v| v.as_object())
              .map(|obj| {
                  obj.iter()
                      .filter_map(|(name, value)| {
                          serde_json::from_value::<AgentMcpConfig>(value.clone())
                              .ok()
                              .map(|config| (name.clone(), config))
                      })
                      .collect()
              })
              .unwrap_or_default();

          Ok(AgentConfig { raw, mcp_servers })
      }

      async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError> {
          let mut raw = config.raw.clone();
          let mcp_obj = serde_json::to_value(&config.mcp_servers)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;
          raw["mcpServers"] = mcp_obj;

          let temp_path = self.config_path.with_extension("vault-tmp");
          let content = serde_json::to_string_pretty(&raw)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;

          if let Some(parent) = self.config_path.parent() {
              tokio::fs::create_dir_all(parent).await.map_err(|e| VaultError::Io {
                  path: parent.to_path_buf(),
                  source: e,
              })?;
          }

          tokio::fs::write(&temp_path, &content)
              .await
              .map_err(|e| VaultError::Io {
                  path: temp_path.clone(),
                  source: e,
              })?;

          tokio::fs::rename(&temp_path, &self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          Ok(())
      }

      async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError> {
          let config = self.read_config().await?;
          let mut diff = SyncDiff::default();

          for entry in entries {
              match config.mcp_servers.get(&entry.name) {
                  None => {
                      diff.additions.push(SyncEntry {
                          name: entry.name.clone(),
                          source: entry.source.to_string(),
                          version: entry.version.clone(),
                      });
                  }
                  Some(existing) => {
                      let mut changes = Vec::new();
                      if existing.command != entry.command {
                          changes.push(FieldChange {
                              field: "command".to_string(),
                              old_value: existing.command.clone(),
                              new_value: entry.command.clone(),
                          });
                      }
                      if existing.args != entry.args {
                          changes.push(FieldChange {
                              field: "args".to_string(),
                              old_value: format!("{:?}", existing.args),
                              new_value: format!("{:?}", entry.args),
                          });
                      }
                      if existing.env != entry.env_vars {
                          changes.push(FieldChange {
                              field: "env".to_string(),
                              old_value: format!("{:?}", existing.env),
                              new_value: format!("{:?}", entry.env_vars),
                          });
                      }
                      if !changes.is_empty() {
                          diff.updates.push(SyncUpdate {
                              name: entry.name.clone(),
                              changed_fields: changes,
                          });
                      }
                  }
              }
          }

          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for (name, _) in &config.mcp_servers {
              if !vault_names.contains(name.as_str()) {
                  diff.removals.push(SyncEntry {
                      name: name.clone(),
                      source: "vault-managed".to_string(),
                      version: "".to_string(),
                  });
              }
          }

          Ok(diff)
      }

      async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError> {
          let diff = self.diff(entries).await?;
          let timestamp = chrono::Utc::now();

          if diff.is_empty() {
              return Ok(SyncResult {
                  agent_type: self.agent_type().to_string(),
                  timestamp,
                  diff,
                  success: true,
                  backup_path: None,
                  error: None,
              });
          }

          let backup_path = if self.config_path.exists() {
              Some(self.backup()?)
          } else {
              None
          };

          let mut config = self.read_config().await?;

          for entry in entries {
              config
                  .mcp_servers
                  .insert(entry.name.clone(), Self::mcp_to_agent_config(entry));
          }

          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for removal in &diff.removals {
              config.mcp_servers.remove(&removal.name);
          }

          self.write_config(&config).await?;

          let valid = self.verify()?;
          if !valid {
              if let Some(ref bp) = backup_path {
                  std::fs::copy(bp, &self.config_path).map_err(|e| VaultError::Io {
                      path: self.config_path.clone(),
                      source: e,
                  })?;
              }
              return Err(VaultError::McpInstall {
                  source_type: self.agent_type().to_string(),
                  message: "Sync verification failed after writing config".to_string(),
              });
          }

          Ok(SyncResult {
              agent_type: self.agent_type().to_string(),
              timestamp,
              diff,
              success: true,
              backup_path: backup_path.map(|p| p.display().to_string()),
              error: None,
          })
      }

      fn backup(&self) -> Result<PathBuf, VaultError> {
          if !self.config_path.exists() {
              return Ok(PathBuf::new());
          }

          std::fs::create_dir_all(&self.backup_dir).map_err(|e| VaultError::Io {
              path: self.backup_dir.clone(),
              source: e,
          })?;

          let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
          let backup_path = self.backup_dir.join(format!("{}.json", timestamp));

          std::fs::copy(&self.config_path, &backup_path).map_err(|e| VaultError::Io {
              path: backup_path.clone(),
              source: e,
          })?;

          Ok(backup_path)
      }

      fn verify(&self) -> Result<bool, VaultError> {
          if !self.config_path.exists() {
              return Ok(false);
          }

          let content = std::fs::read_to_string(&self.config_path).map_err(|e| VaultError::Io {
              path: self.config_path.clone(),
              source: e,
          })?;

          match serde_json::from_str::<serde_json::Value>(&content) {
              Ok(v) => Ok(v.is_object()),
              Err(_) => Ok(false),
          }
      }
  }
  ```

- [ ] **Step 4: Implement CodexConnector**
  Create `crates/vault-connectors/src/codex.rs` (identical schema and traits logic as Claude/Gemini, defaults to `~/.codex/config.json` on Unix and `%USERPROFILE%\.codex\config.json` on Windows):
  ```rust
  use crate::traits::AgentConnector;
  use crate::types::{AgentConfig, AgentMcpConfig, SyncDiff, SyncEntry, SyncResult, SyncUpdate, FieldChange};
  use async_trait::async_trait;
  use std::collections::{HashMap, HashSet};
  use std::path::{Path, PathBuf};
  use vault_core::agent::AgentType;
  use vault_core::error::VaultError;
  use vault_core::mcp::models::McpEntry;

  pub struct CodexConnector {
      config_path: PathBuf,
      backup_dir: PathBuf,
  }

  impl CodexConnector {
      pub fn new() -> Self {
          let home = dirs::home_dir().expect("Could not determine home directory");
          Self {
              config_path: home.join(".codex").join("config.json"),
              backup_dir: home.join(".agentvault").join("backups").join("codex"),
          }
      }

      pub fn new_with_paths(config_path: PathBuf, backup_dir: PathBuf) -> Self {
          Self { config_path, backup_dir }
      }

      fn mcp_to_agent_config(entry: &McpEntry) -> AgentMcpConfig {
          AgentMcpConfig {
              command: entry.command.clone(),
              args: entry.args.clone(),
              env: entry.env_vars.clone(),
          }
      }
  }

  #[async_trait]
  impl AgentConnector for CodexConnector {
      fn agent_type(&self) -> AgentType {
          AgentType::CodexCli
      }

      fn config_path(&self) -> &Path {
          &self.config_path
      }

      async fn read_config(&self) -> Result<AgentConfig, VaultError> {
          if !self.config_path.exists() {
              return Ok(AgentConfig {
                  raw: serde_json::json!({}),
                  mcp_servers: HashMap::new(),
              });
          }

          let content = tokio::fs::read_to_string(&self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          let raw: serde_json::Value = serde_json::from_str(&content)
              .map_err(|e| VaultError::Config {
                  path: self.config_path.clone(),
                  message: format!("Invalid JSON: {}", e),
              })?;

          let mcp_servers = raw
              .get("mcpServers")
              .and_then(|v| v.as_object())
              .map(|obj| {
                  obj.iter()
                      .filter_map(|(name, value)| {
                          serde_json::from_value::<AgentMcpConfig>(value.clone())
                              .ok()
                              .map(|config| (name.clone(), config))
                      })
                      .collect()
              })
              .unwrap_or_default();

          Ok(AgentConfig { raw, mcp_servers })
      }

      async fn write_config(&self, config: &AgentConfig) -> Result<(), VaultError> {
          let mut raw = config.raw.clone();
          let mcp_obj = serde_json::to_value(&config.mcp_servers)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;
          raw["mcpServers"] = mcp_obj;

          let temp_path = self.config_path.with_extension("vault-tmp");
          let content = serde_json::to_string_pretty(&raw)
              .map_err(|e| VaultError::Serialization(e.to_string()))?;

          if let Some(parent) = self.config_path.parent() {
              tokio::fs::create_dir_all(parent).await.map_err(|e| VaultError::Io {
                  path: parent.to_path_buf(),
                  source: e,
              })?;
          }

          tokio::fs::write(&temp_path, &content)
              .await
              .map_err(|e| VaultError::Io {
                  path: temp_path.clone(),
                  source: e,
              })?;

          tokio::fs::rename(&temp_path, &self.config_path)
              .await
              .map_err(|e| VaultError::Io {
                  path: self.config_path.clone(),
                  source: e,
              })?;

          Ok(())
      }

      async fn diff(&self, entries: &[McpEntry]) -> Result<SyncDiff, VaultError> {
          let config = self.read_config().await?;
          let mut diff = SyncDiff::default();

          for entry in entries {
              match config.mcp_servers.get(&entry.name) {
                  None => {
                      diff.additions.push(SyncEntry {
                          name: entry.name.clone(),
                          source: entry.source.to_string(),
                          version: entry.version.clone(),
                      });
                  }
                  Some(existing) => {
                      let mut changes = Vec::new();
                      if existing.command != entry.command {
                          changes.push(FieldChange {
                              field: "command".to_string(),
                              old_value: existing.command.clone(),
                              new_value: entry.command.clone(),
                          });
                      }
                      if existing.args != entry.args {
                          changes.push(FieldChange {
                              field: "args".to_string(),
                              old_value: format!("{:?}", existing.args),
                              new_value: format!("{:?}", entry.args),
                          });
                      }
                      if existing.env != entry.env_vars {
                          changes.push(FieldChange {
                              field: "env".to_string(),
                              old_value: format!("{:?}", existing.env),
                              new_value: format!("{:?}", entry.env_vars),
                          });
                      }
                      if !changes.is_empty() {
                          diff.updates.push(SyncUpdate {
                              name: entry.name.clone(),
                              changed_fields: changes,
                          });
                      }
                  }
              }
          }

          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for (name, _) in &config.mcp_servers {
              if !vault_names.contains(name.as_str()) {
                  diff.removals.push(SyncEntry {
                      name: name.clone(),
                      source: "vault-managed".to_string(),
                      version: "".to_string(),
                  });
              }
          }

          Ok(diff)
      }

      async fn sync(&self, entries: &[McpEntry]) -> Result<SyncResult, VaultError> {
          let diff = self.diff(entries).await?;
          let timestamp = chrono::Utc::now();

          if diff.is_empty() {
              return Ok(SyncResult {
                  agent_type: self.agent_type().to_string(),
                  timestamp,
                  diff,
                  success: true,
                  backup_path: None,
                  error: None,
              });
          }

          let backup_path = if self.config_path.exists() {
              Some(self.backup()?)
          } else {
              None
          };

          let mut config = self.read_config().await?;

          for entry in entries {
              config
                  .mcp_servers
                  .insert(entry.name.clone(), Self::mcp_to_agent_config(entry));
          }

          let vault_names: HashSet<&str> = entries.iter().map(|e| e.name.as_str()).collect();
          for removal in &diff.removals {
              config.mcp_servers.remove(&removal.name);
          }

          self.write_config(&config).await?;

          let valid = self.verify()?;
          if !valid {
              if let Some(ref bp) = backup_path {
                  std::fs::copy(bp, &self.config_path).map_err(|e| VaultError::Io {
                      path: self.config_path.clone(),
                      source: e,
                  })?;
              }
              return Err(VaultError::McpInstall {
                  source_type: self.agent_type().to_string(),
                  message: "Sync verification failed after writing config".to_string(),
              });
          }

          Ok(SyncResult {
              agent_type: self.agent_type().to_string(),
              timestamp,
              diff,
              success: true,
              backup_path: backup_path.map(|p| p.display().to_string()),
              error: None,
          })
      }

      fn backup(&self) -> Result<PathBuf, VaultError> {
          if !self.config_path.exists() {
              return Ok(PathBuf::new());
          }

          std::fs::create_dir_all(&self.backup_dir).map_err(|e| VaultError::Io {
              path: self.backup_dir.clone(),
              source: e,
          })?;

          let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
          let backup_path = self.backup_dir.join(format!("{}.json", timestamp));

          std::fs::copy(&self.config_path, &backup_path).map_err(|e| VaultError::Io {
              path: backup_path.clone(),
              source: e,
          })?;

          Ok(backup_path)
      }

      fn verify(&self) -> Result<bool, VaultError> {
          if !self.config_path.exists() {
              return Ok(false);
          }

          let content = std::fs::read_to_string(&self.config_path).map_err(|e| VaultError::Io {
              path: self.config_path.clone(),
              source: e,
          })?;

          match serde_json::from_str::<serde_json::Value>(&content) {
              Ok(v) => Ok(v.is_object()),
              Err(_) => Ok(false),
          }
      }
  }
  ```

- [ ] **Step 5: Register modules in lib.rs**
  Modify `crates/vault-connectors/src/lib.rs`:
  ```rust
  pub mod traits;
  pub mod types;
  pub mod claude;
  pub mod gemini;
  pub mod opencode;
  pub mod codex;

  #[cfg(test)]
  mod tests;
  ```

- [ ] **Step 6: Run tests to verify they pass**
  Run: `cargo test -p vault-connectors`
  Expected: PASS

- [ ] **Step 7: Commit**
  Run:
  ```bash
  git add crates/vault-connectors/src/opencode.rs crates/vault-connectors/src/codex.rs crates/vault-connectors/src/lib.rs crates/vault-connectors/src/tests.rs
  git commit -m "feat: implement OpenCode and Codex CLI connectors"
  ```

---

### Task 4: Implement SyncEngine & SQLite Log Integration

**Files:**
- Create: `crates/vault-connectors/src/sync.rs`
- Modify: `crates/vault-connectors/src/lib.rs`
- Modify: `crates/vault-connectors/src/tests.rs`

**Interfaces:**
- Consumes: `Registry` from `vault_core::registry::Registry`, connector list from Tasks 2-3, and SQLite logging queries.
- Produces: `SyncEngine` struct managing capability sync, diff preview, and audit logging.

- [ ] **Step 1: Write failing test for SyncEngine**
  Add tests inside `crates/vault-connectors/src/tests.rs`:
  ```rust
  use crate::sync::SyncEngine;
  use vault_core::registry::SqliteRegistry;
  use std::sync::Arc;

  #[tokio::test]
  async fn test_sync_engine_initialization() {
      let temp = tempdir().unwrap();
      let db_path = temp.path().join("vault.db");
      let registry = Arc::new(SqliteRegistry::new(&db_path).unwrap());
      let backup_dir = temp.path().join("backups");
      
      let _engine = SyncEngine::new(registry, backup_dir);
  }
  ```

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test -p vault-connectors`
  Expected: Compile error because `SyncEngine` is not defined.

- [ ] **Step 3: Implement SyncEngine**
  Create `crates/vault-connectors/src/sync.rs`:
  ```rust
  use crate::traits::AgentConnector;
  use crate::types::{SyncDiff, SyncResult};
  use std::path::PathBuf;
  use std::sync::Arc;
  use vault_core::agent::{AgentConnectorConfig, SyncHistoryEntry};
  use vault_core::error::VaultError;
  use vault_core::mcp::models::McpEntry;
  use vault_core::registry::Registry;

  pub struct SyncEngine {
      registry: Arc<dyn Registry>,
      backup_dir: PathBuf,
  }

  impl SyncEngine {
      pub fn new(registry: Arc<dyn Registry>, backup_dir: PathBuf) -> Self {
          Self { registry, backup_dir }
      }

      pub async fn sync_agent(&self, connector: &dyn AgentConnector, prune: bool) -> Result<SyncResult, VaultError> {
          let all_mcps = self.registry.list_mcps()?;
          let agent_str = connector.agent_type().to_string();
          let filtered_mcps: Vec<McpEntry> = all_mcps
              .into_iter()
              .filter(|mcp| mcp.agents.contains(&agent_str))
              .collect();

          let mut diff = connector.diff(&filtered_mcps).await?;
          if !prune {
              diff.removals.clear();
          }

          let mut result = connector.sync(&filtered_mcps).await?;
          result.diff = diff; 

          let history_entry = SyncHistoryEntry {
              id: uuid::Uuid::new_v4().to_string(),
              agent_type: agent_str.clone(),
              action: "sync".to_string(),
              diff_json: serde_json::to_string(&result.diff).unwrap_or_default(),
              synced_at: result.timestamp,
              success: result.success,
              error: result.error.clone(),
          };
          self.registry.log_sync(&history_entry)?;

          if result.success {
              if let Ok(mut config) = self.registry.get_agent_config(&agent_str) {
                  config.last_synced = Some(result.timestamp);
                  let _ = self.registry.delete_agent_config(&agent_str);
                  let _ = self.registry.insert_agent_config(&config);
              }
          }

          Ok(result)
      }

      pub async fn dry_run(&self, connector: &dyn AgentConnector) -> Result<SyncDiff, VaultError> {
          let all_mcps = self.registry.list_mcps()?;
          let agent_str = connector.agent_type().to_string();
          let filtered_mcps: Vec<McpEntry> = all_mcps
              .into_iter()
              .filter(|mcp| mcp.agents.contains(&agent_str))
              .collect();

          connector.diff(&filtered_mcps).await
      }
  }
  ```

- [ ] **Step 4: Register sync in lib.rs**
  Modify `crates/vault-connectors/src/lib.rs`:
  ```rust
  pub mod traits;
  pub mod types;
  pub mod claude;
  pub mod gemini;
  pub mod opencode;
  pub mod codex;
  pub mod sync;

  #[cfg(test)]
  mod tests;
  ```

- [ ] **Step 5: Run tests to verify they pass**
  Run: `cargo test -p vault-connectors`
  Expected: PASS

- [ ] **Step 6: Commit**
  Run:
  ```bash
  git add crates/vault-connectors/src/sync.rs crates/vault-connectors/src/lib.rs crates/vault-connectors/src/tests.rs
  git commit -m "feat: implement SyncEngine and logging integration"
  ```

---

### Task 5: Wire sync and connector CLI commands

**Files:**
- Modify: `crates/vault-cli/src/commands/sync.rs`
- Modify: `crates/vault-cli/src/commands/connector.rs`
- Modify: `crates/vault-cli/src/commands/status.rs`

**Interfaces:**
- Consumes: `SyncEngine` and connectors from `vault-connectors`
- Produces: Command line handling for `vault sync` and `vault connector` commands.

- [ ] **Step 1: Wire config path and SQLite registry connection**
  Modify `crates/vault-cli/src/commands/sync.rs`:
  ```rust
  use crate::cli::SyncArgs;
  use anyhow::{anyhow, Result};
  use std::path::PathBuf;
  use std::sync::Arc;
  use vault_connectors::traits::AgentConnector;
  use vault_connectors::sync::SyncEngine;
  use vault_connectors::claude::ClaudeConnector;
  use vault_connectors::gemini::GeminiConnector;
  use vault_connectors::opencode::OpenCodeConnector;
  use vault_connectors::codex::CodexConnector;
  use vault_core::agent::AgentType;
  use vault_core::registry::SqliteRegistry;
  use colored::Colorize;

  pub async fn handle(args: SyncArgs, vault_dir_override: Option<&str>) -> Result<()> {
      let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
      let vault_dir = vault_dir_override
          .map(PathBuf::from)
          .unwrap_or_else(|| home.join(".agentvault"));

      let db_path = vault_dir.join("vault.db");
      let registry = Arc::new(SqliteRegistry::new(&db_path)?);
      let backup_dir = vault_dir.join("backups");
      let engine = SyncEngine::new(registry.clone(), backup_dir);

      let connectors: Vec<Box<dyn AgentConnector>> = vec![
          Box::new(ClaudeConnector::new()),
          Box::new(GeminiConnector::new()),
          Box::new(OpenCodeConnector::new()),
          Box::new(CodexConnector::new()),
      ];

      let target_agents = if args.all {
          let registered = registry.list_agent_configs()?;
          if registered.is_empty() {
              println!("{}", "No agents registered. Add one using `vault connector add <agent>`.".yellow());
              return Ok(());
          }
          registered.into_iter().map(|c| c.agent_type).collect::<Vec<_>>()
      } else if let Some(ref agent_str) = args.agent {
          let agent_type: AgentType = agent_str.parse().map_err(|e: String| anyhow!(e))?;
          vec![agent_type]
      } else {
          return Err(anyhow!("Must specify an agent to sync or use --all"));
      };

      for agent_type in target_agents {
          let conn = connectors.iter().find(|c| c.agent_type() == agent_type);
          if let Some(connector) = conn {
              if args.dry_run {
                  let diff = engine.dry_run(connector.as_ref()).await?;
                  println!("Dry run diff for {}:", agent_type.to_string().bold());
                  if diff.is_empty() {
                      println!("  No changes.");
                  } else {
                      for add in &diff.additions {
                          println!("  + {} (add)", add.name.green());
                      }
                      for upd in &diff.updates {
                          println!("  ~ {} (update)", upd.name.yellow());
                      }
                      if args.prune {
                          for rem in &diff.removals {
                              println!("  - {} (remove)", rem.name.red());
                          }
                      }
                  }
              } else {
                  println!("Syncing to {}...", agent_type.to_string().bold());
                  let result = engine.sync_agent(connector.as_ref(), args.prune).await?;
                  if result.success {
                      println!("{} Successfully synced {}", "✓".green(), agent_type.to_string().bold());
                      if let Some(ref backup) = result.backup_path {
                          println!("  Backup created: {}", backup);
                      }
                  } else {
                      println!("{} Failed to sync {}: {:?}", "✗".red(), agent_type.to_string().bold(), result.error);
                  }
              }
          } else {
              println!("{} Unknown or unsupported agent: {}", "✗".red(), agent_type.to_string());
          }
      }

      Ok(())
  }
  ```

- [ ] **Step 2: Wire handle logic for connector commands**
  Modify `crates/vault-cli/src/commands/connector.rs`:
  ```rust
  use crate::cli::{ConnectorArgs, ConnectorCommands, ConnectorAddArgs, ConnectorListArgs, ConnectorRemoveArgs};
  use anyhow::{anyhow, Result};
  use std::path::PathBuf;
  use std::sync::Arc;
  use vault_core::agent::{AgentConnectorConfig, AgentType};
  use vault_core::registry::SqliteRegistry;
  use tabled::{Table, Tabled};
  use colored::Colorize;

  #[derive(Tabled)]
  struct ConnectorTableEntry {
      #[tabled(rename = "Agent Type")]
      agent_type: String,
      #[tabled(rename = "Config Path")]
      config_path: String,
      #[tabled(rename = "Enabled")]
      enabled: bool,
      #[tabled(rename = "Auto Sync")]
      auto_sync: bool,
      #[tabled(rename = "Last Synced")]
      last_synced: String,
  }

  pub async fn handle(args: ConnectorArgs, vault_dir_override: Option<&str>) -> Result<()> {
      let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not resolve home directory"))?;
      let vault_dir = vault_dir_override
          .map(PathBuf::from)
          .unwrap_or_else(|| home.join(".agentvault"));

      let db_path = vault_dir.join("vault.db");
      let registry = Arc::new(SqliteRegistry::new(&db_path)?);

      match args.command {
          ConnectorCommands::Add(subargs) => {
              let agent_type: AgentType = subargs.agent_type.parse().map_err(|e: String| anyhow!(e))?;
              
              let config_path = if let Some(path) = subargs.config_path {
                  PathBuf::from(path)
              } else {
                  match agent_type {
                      AgentType::ClaudeCode => home.join(".claude").join("claude_desktop_config.json"),
                      AgentType::GeminiCli => home.join(".gemini").join("config").join("settings.json"),
                      AgentType::OpenCode => {
                          let config_dir = std::env::var("XDG_CONFIG_HOME")
                              .map(PathBuf::from)
                              .unwrap_or_else(|_| home.join(".config"));
                          config_dir.join("opencode").join("config.json")
                      }
                      AgentType::CodexCli => home.join(".codex").join("config.json"),
                      _ => return Err(anyhow!("Cannot resolve default config path for custom agent, please provide --config-path")),
                  }
              };

              let config = AgentConnectorConfig {
                  id: uuid::Uuid::new_v4().to_string(),
                  agent_type,
                  config_path,
                  enabled: true,
                  last_synced: None,
                  auto_sync: subargs.auto_sync,
              };

              registry.insert_agent_config(&config)?;
              println!("{} Successfully added agent connector: {}", "✓".green(), subargs.agent_type.bold());
          }
          ConnectorCommands::List(subargs) => {
              let configs = registry.list_agent_configs()?;
              if subargs.json {
                  println!("{}", serde_json::to_string_pretty(&configs)?);
              } else if configs.is_empty() {
                  println!("No agent connectors registered. Add one with `vault connector add`.");
              } else {
                  let table_entries: Vec<ConnectorTableEntry> = configs
                      .into_iter()
                      .map(|c| ConnectorTableEntry {
                          agent_type: c.agent_type.to_string(),
                          config_path: c.config_path.to_string_lossy().to_string(),
                          enabled: c.enabled,
                          auto_sync: c.auto_sync,
                          last_synced: c.last_synced
                              .map(|t| t.to_rfc3339())
                              .unwrap_or_else(|| "Never".to_string()),
                      })
                      .collect();
                  println!("{}", Table::new(table_entries));
              }
          }
          ConnectorCommands::Remove(subargs) => {
              registry.delete_agent_config(&subargs.agent_type)?;
              println!("{} Successfully removed agent connector: {}", "✓".green(), subargs.agent_type.bold());
          }
      }
      Ok(())
  }
  ```

- [ ] **Step 3: Update vault status to display registered connectors and sync info**
  Modify `crates/vault-cli/src/commands/status.rs` to query `registry.list_agent_configs()` and display:
  ```rust
  // Inside crates/vault-cli/src/commands/status.rs handle()
  // Add list_agent_configs rendering if not in json mode
  let agents = registry.list_agent_configs()?;
  println!("\nRegistered Connectors:");
  if agents.is_empty() {
      println!("  None");
  } else {
      for agent in agents {
          let sync_time = agent.last_synced
              .map(|t| t.to_rfc3339())
              .unwrap_or_else(|| "Never".to_string());
          println!("  - {}: path={}, last_synced={}", agent.agent_type.to_string().bold(), agent.config_path.display(), sync_time);
      }
  }
  ```

- [ ] **Step 4: Run CLI compilation**
  Run: `cargo build --workspace`
  Expected: PASS

- [ ] **Step 5: Run full tests to verify everything compiles and passes**
  Run: `cargo test`
  Expected: PASS

- [ ] **Step 6: Commit**
  Run:
  ```bash
  git add crates/vault-cli/src/commands/sync.rs crates/vault-cli/src/commands/connector.rs crates/vault-cli/src/commands/status.rs
  git commit -m "feat: wire sync and connector commands to CLI"
  ```
