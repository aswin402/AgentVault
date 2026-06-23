# Task 4 Report: Implement SyncEngine & SQLite Log Integration

## What Was Implemented

1. **`SyncEngine` struct** inside `crates/vault-connectors/src/sync.rs`. It manages:
   - Synchronizing AI agent capability configs with the sqlite registry.
   - Performing dry-run diffs (`dry_run`).
   - Logging audit logs to the SQLite database via the registry (`log_sync`).
   - Pruning/preserving connector configurations depending on the `prune` parameter.
2. **`update_agent_config` Registry integration**:
   - Modified `Registry` trait and `SqliteRegistry` implementation in `crates/vault-core/src/registry.rs` to support `update_agent_config`.
   - Used this update method inside `SyncEngine` to update the `last_synced` timestamp for the synchronized agent without using the delete-and-reinsert pattern, which would have triggered `ON DELETE CASCADE` and deleted the entire sync history.
3. **Lib Registration**:
   - Registered `pub mod sync;` in `crates/vault-connectors/src/lib.rs`.
4. **Dependencies**:
   - Added `uuid` to `crates/vault-connectors/Cargo.toml` as it's required for generating unique sync history log entries.

## What Was Tested & Test Results

Added comprehensive integration tests under the `integration_tests` module in `crates/vault-connectors/src/tests.rs`:
- `test_sync_engine_initialization`: Verifies that `SyncEngine` initializes correctly.
- `test_sync_engine_dry_run`: Verifies that `dry_run` successfully generates the correct difference.
- `test_sync_engine_sync_agent_prune_true`: Verifies that `sync_agent` with `prune = true` installs new servers, prunes servers that aren't in the registry, saves history logs to SQLite, and updates the agent's `last_synced` config timestamp.
- `test_sync_engine_sync_agent_prune_false`: Verifies that `sync_agent` with `prune = false` installs new servers and leaves non-registry-backed connector configurations intact.

### TDD Evidence

- **RED**: Running tests initially failed to compile as expected. First, `SyncEngine` was missing, then we had missing trait imports:
  - Command run: `cargo test -p vault-connectors`
  - Output:
    ```
    error[E0599]: no method named `insert_agent_config` found for struct `Arc<SqliteRegistry>` in the current scope
       --> crates/vault-connectors/src/tests.rs:491:18
        |
    491 |         registry.insert_agent_config(&agent_config).unwrap();
        |                  ^^^^^^^^^^^^^^^^^^^
        |
        = help: items from traits can only be used if the trait is in scope
    ```
- **GREEN**: All tests compile and pass successfully after bringing `Registry` into scope in tests and cleaning up warnings:
  - Command run: `cargo test -p vault-connectors`
  - Output:
    ```
    running 22 tests
    test tests::integration_tests::test_codex_connector_paths ... ok
    ...
    test tests::integration_tests::test_sync_engine_initialization ... ok
    test tests::integration_tests::test_sync_engine_dry_run ... ok
    test tests::integration_tests::test_sync_engine_sync_agent_prune_true ... ok
    test tests::integration_tests::test_sync_engine_sync_agent_prune_false ... ok

    test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
    ```

## Files Changed

- `crates/vault-connectors/Cargo.toml`
- `crates/vault-connectors/src/lib.rs`
- `crates/vault-connectors/src/tests.rs`
- `crates/vault-connectors/src/sync.rs` (new)
- `crates/vault-core/src/registry.rs`

## Self-Review Findings

- **Architecture Integrity**: Realized that the delete-then-insert pattern to update the agent's `last_synced` timestamp would trigger the SQLite `ON DELETE CASCADE` constraint on the `sync_history` foreign key (which references `agent_configs(agent_type)`). This would have deleted all history logs every time a successful sync completed. Resolved this by introducing an explicit `update_agent_config` method to the `Registry` trait and implementing it for `SqliteRegistry`.
- **YAGNI**: Standardized configuration pruning exactly as requested, preserving files and registry interfaces cleanly.
