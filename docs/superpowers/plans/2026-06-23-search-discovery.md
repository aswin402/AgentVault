# Search & Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement unified local fuzzy search and npm registry search capabilities to discover and rank MCP servers, skills, and workflows, along with the `vault search` CLI subcommand.

**Architecture:** Design a `SearchEngine` struct in `crates/vault-core/src/search.rs` that combines results from the local SQLite registry and the npm registry. Local search applies relevance scoring based on exact name match (1.0), partial name match (0.8), tag match (0.7), and description keyword match (0.5), filtering out scores below or equal to 0.3. Remote search queries npm's search API, filters for MCP-relevant packages using keyword heuristics, and reports results alongside local ones.

**Tech Stack:** Rust, SQLite (`rusqlite`), `serde`, `serde_json`, `reqwest`, `tokio`, `tabled`, `indicatif`.

## Global Constraints

- Code must compile warning-free with `cargo clippy --workspace --all-targets -- -D warnings`.
- Code formatting must fully comply with `cargo fmt --all -- --check`.
- Unit tests must be written for all non-trivial logic.

---

### Task 1: Local Search & Discovery Engine

**Files:**
- Create: `crates/vault-core/src/search.rs`
- Modify: `crates/vault-core/src/lib.rs`
- Test: Unit tests inside `crates/vault-core/src/search.rs`

**Interfaces:**
- Consumes: `Registry` trait, `CapabilityRecord`, `CapabilityKind`, `VaultError`
- Produces: `SearchEngine` struct, `SearchResult` struct, and local scoring capabilities

- [ ] **Step 1: Define types and skeleton for local search**
  Create `crates/vault-core/src/search.rs` with the `SearchResult` and `SearchEngine` declarations, and a draft scoring function:
  ```rust
  use crate::error::VaultError;
  use crate::registry::Registry;
  use crate::capability::models::{CapabilityRecord, CapabilityKind};
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  pub struct SearchResult {
      pub id: Option<String>,
      pub name: String,
      pub kind: CapabilityKind,
      pub description: Option<String>,
      pub tags: Vec<String>,
      pub source: String, // "local" or "npm"
      pub score: f64,
  }

  pub struct SearchEngine {
      registry: Arc<dyn Registry>,
  }

  impl SearchEngine {
      pub fn new(registry: Arc<dyn Registry>) -> Self {
          Self { registry }
      }

      pub fn calculate_score(query: &str, record: &CapabilityRecord) -> f64 {
          let query_lower = query.to_lowercase();
          let name_lower = record.name.to_lowercase();

          // 1. Exact name match (case-insensitive check)
          if name_lower == query_lower {
              return 1.0;
          }

          // 2. Partial name match (contains substring)
          if name_lower.contains(&query_lower) {
              return 0.8;
          }

          // 3. Tag match (any tag is an exact match to the query)
          let has_tag_match = record.tags.iter().any(|t| t.to_lowercase() == query_lower);
          if has_tag_match {
              return 0.7;
          }

          // 4. Description keyword match (description contains query as substring)
          if let Some(desc) = &record.description {
              if desc.to_lowercase().contains(&query_lower) {
                  return 0.5;
              }
          }

          0.0
      }

      pub fn search_local(&self, query: &str) -> Result<Vec<SearchResult>, VaultError> {
          let records = self.registry.search(query)?;
          let mut results = Vec::new();

          for record in records {
              let score = Self::calculate_score(query, &record);
              if score > 0.3 {
                  results.push(SearchResult {
                      id: Some(record.id),
                      name: record.name,
                      kind: record.kind,
                      description: record.description,
                      tags: record.tags,
                      source: "local".to_string(),
                      score,
                  });
              }
          }

          // Sort descending by score, then alphabetically by name as tie-breaker
          results.sort_by(|a, b| {
              b.score
                  .partial_cmp(&a.score)
                  .unwrap_or(std::cmp::Ordering::Equal)
                  .then_with(|| a.name.cmp(&b.name))
          });

          Ok(results)
      }
  }
  ```

- [ ] **Step 2: Add mod search to crates/vault-core/src/lib.rs**
  Modify `crates/vault-core/src/lib.rs` to register the new search module:
  ```rust
  pub mod search;
  ```

- [ ] **Step 3: Write tests for local search engine**
  Write tests in `crates/vault-core/src/search.rs` to test:
  - Exact name match score = 1.0
  - Partial name match score = 0.8
  - Tag match score = 0.7
  - Description match score = 0.5
  - Scores <= 0.3 are filtered out (score 0.0)
  - Sorting order (higher score first, then tie-breaking by name)

- [ ] **Step 4: Run cargo test to verify Task 1 works**
  Verify everything compiles and all tests pass:
  `cargo test`

---

### Task 2: Remote npm Registry Search

**Files:**
- Modify: `crates/vault-core/src/search.rs`
- Test: Add unit/mock tests to `crates/vault-core/src/search.rs`

- [ ] **Step 1: Implement search_npm method**
  Add `search_npm` using `reqwest` to query `https://registry.npmjs.org/-/v1/search` with the parameters:
  `text={query}&size=20`

  Parse response payload:
  ```json
  {
    "objects": [
      {
        "package": {
          "name": "package-name",
          "version": "1.0.0",
          "description": "package description",
          "keywords": ["mcp", "server"]
        }
      }
    ]
  }
  ```

  Filter objects to keep only packages that are relevant MCP servers using keyword heuristics (checks if name, description, or keywords contain "mcp", "mcp-server", or "model-context-protocol" case-insensitively).

  Assign relevance score:
  - If package name matches query exactly: 1.0
  - If name contains query: 0.8
  - If description contains query: 0.5
  - Else: 0.4

  Map to `SearchResult` with `source: "npm"`, `kind: CapabilityKind::Mcp`.

- [ ] **Step 2: Add unit/mock tests for npm search parsing**
  Write a test that mocks or stubs the HTTP client or directly parses mock JSON response data to verify the mapping, keyword filter heuristics, and scoring.

- [ ] **Step 3: Run cargo test to verify Task 2 works**
  Verify the project compiles and tests pass.

---

### Task 3: CLI Subcommand wiring

**Files:**
- Modify: `crates/vault-cli/src/commands/search.rs`
- Test: Run the binary manually to confirm search behaves properly

- [ ] **Step 1: Wiring command logic**
  Implement the subcommand handler:
  - Show a spinner while fetching results.
  - Run `search_local` and `search_npm` in parallel or sequence.
  - If `--local` is specified, skip npm. If `--npm` is specified, skip local.
  - Combine and sort all results. Deduplicate by name (if a local package and an npm package share the same name, mark the local one as installed/local and prioritize it).
  - Print results using `tabled::Table`. Show columns: `Installed`, `Name`, `Source`, `Relevance`, `Description`.
  - Apply `owo-colors` formatting.
