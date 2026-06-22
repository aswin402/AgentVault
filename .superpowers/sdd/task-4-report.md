# Task 4 Report: Implement PyPI Installer

## What was implemented
- Implemented `McpSource::PyPi` package installation handling inside `DefaultMcpManager::install` in [manager.rs](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-core/src/mcp/manager.rs).
- **Executable Discovery**: Added dynamic checks to detect `uv`, `python3`, or `python` in PATH, returning `VaultError::McpInstall` if none of these are available.
- **Virtualenv Scaffolding**:
  - If `uv` is found, runs `uv venv <vault_dir>/mcps/<name>/venv` followed by `uv pip install --python <venv_python> <package_spec>`.
  - If only `python3`/`python` is found, runs `python -m venv <vault_dir>/mcps/<name>/venv` followed by `<pip_path> install <package_spec>`.
- **Target Dir Cleanup**: Ensures `<vault_dir>/mcps/<name>` is cleaned using `clean_target_dir` before starting, and recursively removed via `std::fs::remove_dir_all` if the venv creation or installation fails.
- **Asynchronous Execution**: Used `tokio::process::Command` to run all command-line operations (scaffolding, installing, checking version) asynchronously.
- **Command Resolution**: Implemented the `resolve_pypi_cmd` helper to locate an executable matching the package name (with underscores replaced by hyphens) in the virtualenv's binary folder (`venv/bin/` or `venv/Scripts/`, checking standard executable extensions on Windows). Falls back to the virtualenv's `python` path with `["-m", package_name]` args if no binary is found.
- **Version Query**: Resolves the exact package version securely by running the virtual environment's Python to execute `sys.argv[1]` metadata extraction. It passes the package name as a command-line argument rather than embedding/formatting it directly into the Python code string (preventing script injection vulnerabilities):
  - Runs `"import sys, importlib.metadata; print(importlib.metadata.version(sys.argv[1]))"`
  - Falls back to `"import sys, pkg_resources; print(pkg_resources.get_distribution(sys.argv[1]).version)"`
  - Defaults to `version_req` if the python query fails.
- **Formatting**: Ran `cargo fmt --all` to format all source files dynamically according to the project rules.
- **Unit Test**: Added `test_mcp_manager_install_pypi` in [manager_tests.rs](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-core/src/mcp/manager_tests.rs) using the zero-dependency `"six"` package. The test detects if Python or `uv` is available, runs the installation, asserts directory existence, and validates registry database serialization and retrieval.

## TDD Evidence

### RED Phase
- **Command run**: `cargo test test_mcp_manager_install_pypi`
- **Why failure was expected**: Prior to implementation, `DefaultMcpManager::install` for `McpSource::PyPi` fell through to return `VaultError::NotFound`, causing the test to panic on `unwrap()`.
- **Output**:
  ```
  test mcp::manager_tests::tests::test_mcp_manager_install_pypi ... FAILED
  failures:
  ---- mcp::manager_tests::tests::test_mcp_manager_install_pypi stdout ----
  thread 'mcp::manager_tests::tests::test_mcp_manager_install_pypi' panicked at crates/vault-core/src/mcp/manager_tests.rs:158:14:
  called `Result::unwrap()` on an `Err` value: NotFound { kind: "mcp", name: "six-mcp" }
  ```

### GREEN Phase
- **Command run**: `cargo test`
- **Passing Output**:
  ```
  running 7 tests
  test config::tests::test_default_config ... ok
  test config::tests::test_save_and_load_config ... ok
  test mcp::manager_tests::tests::test_mcp_manager_get_and_list_empty ... ok
  test mcp::manager_tests::tests::test_mcp_manager_install_local ... ok
  test registry::tests::test_sqlite_registry_mcp_ops ... ok
  test mcp::manager_tests::tests::test_mcp_manager_install_pypi ... ok
  test mcp::manager_tests::tests::test_mcp_manager_install_npm ... ok

  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
  ```

## Files Changed
- Modify: [manager.rs](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-core/src/mcp/manager.rs)
- Test: [manager_tests.rs](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/AgentVault/crates/vault-core/src/mcp/manager_tests.rs)

## Self-Review Findings
- **Completeness**: Successfully implements all 7 steps outlined in the PyPI installation task brief and security mitigation reviews.
- **Correctness**: Venv creation, package installation, launch command resolution, secure version querying, and formatting conform fully with the specifications.
- **Security**: Mitigated potential python code injection by using `sys.argv` instead of string interpolation inside python scripts.
- **Robustness**: Properly handles command failure, cleanup of temporary directories, path resolution across Unix/Windows, and fails gracefully to fallback values or errors.
