# Manifest & Declarative Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the declarative state management layer using a `vault.toml` file, enabling users to export current registry state to a manifest and import state back to synchronize capabilities across environments.

**Architecture:** 
1. Define a `VaultManifest` struct and parser in `crates/vault-core/src/manifest.rs` utilizing `toml` and `serde`.
2. Implement `vault export` which reads the database registry state (MCPs, skills, workflows, and registered agent connectors) and formats it into a pretty-printed `vault.toml` file.
3. Implement `vault import` which compares a `vault.toml` file against the database state, computing a diff of what to install, what to update, and what to prune (if `--prune` is passed), executing the required installs/updates/removals sequentially.

**Tech Stack:** Rust, `serde`, `toml`, `semver`, `tokio`.

## Global Constraints

- Code must compile warning-free with `cargo clippy --workspace --all-targets -- -D warnings`.
- Code formatting must fully comply with `cargo fmt --all -- --check`.
- Unit tests must be written for all non-trivial logic.

---

### Task 1: Define vault.toml Manifest Specification & Parser

**Files:**
- Create: `crates/vault-core/src/manifest.rs`
- Modify: `crates/vault-core/src/lib.rs`
- Test: Unit tests inside `crates/vault-core/src/manifest.rs`

- [ ] **Step 1: Declare the `VaultManifest` structs**
  Define `VaultManifest` and its supporting metadata structs in `crates/vault-core/src/manifest.rs`. Make sure fields deserialize correctly with TOML formats.
  
- [ ] **Step 2: Add validation rules**
  Add helper method `pub fn validate(&self) -> Result<(), crate::error::VaultError>` on `VaultManifest` to verify:
  - TOML semantic constraints.
  - Source strings are parseable as valid `McpSource` (e.g. `npm:package`, `local:/path`).
  - Semver constraints are valid formatting.

- [ ] **Step 3: Register mod manifest in crates/vault-core/src/lib.rs**
  Add `pub mod manifest;` to `crates/vault-core/src/lib.rs`.

- [ ] **Step 4: Write unit tests**
  Write tests for deserialization and validation of:
  - A valid complete `vault.toml` manifest.
  - Invalid manifests (e.g. invalid semver constraints, missing required metadata, invalid source format).

- [ ] **Step 5: Run tests**
  Verify everything compiles and all tests pass: `cargo test`

---

### Task 2: Implement vault export Subcommand

**Files:**
- Modify: `crates/vault-cli/src/commands/export.rs`
- Modify: `crates/vault-core/src/manifest.rs`

- [ ] **Step 1: Write export generator method**
  Add a method `pub fn from_registry(registry: &dyn Registry) -> Result<Self, VaultError>` on `VaultManifest` that queries installed MCPs, skills, workflows, and registered agent connectors and formats them into a `VaultManifest`.

- [ ] **Step 2: Wire CLI handler**
  In `crates/vault-cli/src/commands/export.rs`:
  - Open registry database.
  - Generate the manifest.
  - Serialize to string using `toml::to_string_pretty`.
  - Write output to the destination path specified by `--output` (defaults to `./vault.toml`).
  - Print a success message detailing what was exported.

- [ ] **Step 3: Test export functionality**
  Run export manually and verify it generates a valid, beautifully formatted `vault.toml` file matching the current state.

---

### Task 3: Implement vault import Subcommand

**Files:**
- Modify: `crates/vault-cli/src/commands/import.rs`

- [ ] **Step 1: Write import diff calculator**
  Implement reconciliation logic that parses a manifest and compares it against the local registry, returning three lists:
  - Capabilities to install.
  - Capabilities to update.
  - Capabilities to remove (prune).

- [ ] **Step 2: Wire CLI handler**
  In `crates/vault-cli/src/commands/import.rs`:
  - Read and validate the `vault.toml` manifest from `--file` (defaults to `./vault.toml`).
  - Compute the reconciliation diff.
  - If `--dry-run` is passed, print the diff details in a clean table and terminate.
  - Otherwise, ask for confirmation (if not running in non-interactive mode).
  - Run the install, update, and remove commands sequentially with spinners, logging the outcome.
  - Run sync for connectors specified under `[agents]`.

- [ ] **Step 3: Write tests and verify**
  Run full integration verification to verify that:
  - Importing on an empty vault sets up everything declared in the manifest.
  - Importing twice is idempotent.
  - Pruning removes unlisted capabilities correctly.
