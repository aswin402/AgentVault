use std::process::Command;
use tempfile::tempdir;

fn get_bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // pop e2e_tests-xxxx
    if path.file_name().unwrap() == "deps" {
        path.pop(); // pop deps
    }
    path.join(if cfg!(windows) { "vault.exe" } else { "vault" })
}

#[test]
fn test_e2e_lifecycle() {
    let bin = get_bin_path();
    let temp_vault = tempdir().unwrap();
    let vault_dir = temp_vault.path().to_path_buf();

    // 1. vault init
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("init")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Create a local directory for mock MCP
    let local_mcp_dir = tempdir().unwrap();
    let mcp_script = local_mcp_dir.path().join("mcp.sh");
    std::fs::write(&mcp_script, "#!/bin/sh\necho 'hello'").unwrap();

    // 2. vault install local
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("install")
        .arg(format!("local:{}", local_mcp_dir.path().display()))
        .arg("--name")
        .arg("my-local-mcp")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "install failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. vault list
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("list")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("my-local-mcp"),
        "list doesn't contain installed MCP: {}",
        stdout
    );

    // 4. vault export
    let export_toml = vault_dir.join("export.toml");
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("export")
        .arg("--output")
        .arg(&export_toml)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(export_toml.exists());

    // 5. vault import on a fresh vault
    let temp_vault_new = tempdir().unwrap();
    let vault_dir_new = temp_vault_new.path().to_path_buf();

    // Init the new one
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir_new)
        .arg("init")
        .output()
        .unwrap();
    assert!(output.status.success());

    // Import the exported config
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir_new)
        .arg("import")
        .arg(&export_toml)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "import failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // List the new one and check that it contains my-local-mcp
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir_new)
        .arg("list")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my-local-mcp"));
}

#[test]
fn test_e2e_remove() {
    let bin = get_bin_path();
    let temp_vault = tempdir().unwrap();
    let vault_dir = temp_vault.path().to_path_buf();

    // init
    Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("init")
        .output()
        .unwrap();

    let local_mcp_dir = tempdir().unwrap();
    // install
    Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("install")
        .arg(format!("local:{}", local_mcp_dir.path().display()))
        .arg("--name")
        .arg("my-mcp")
        .output()
        .unwrap();

    // list to verify it's there
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("list")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("my-mcp"));

    // remove
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("remove")
        .arg("my-mcp")
        .arg("--force")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "remove failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // list to verify it's gone
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("list")
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stdout).contains("my-mcp"));
}

#[test]
fn test_e2e_edge_cases() {
    let bin = get_bin_path();
    let temp_vault = tempdir().unwrap();
    let vault_dir = temp_vault.path().to_path_buf();

    // init
    Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("init")
        .output()
        .unwrap();

    // 1. List on empty vault
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("list")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No capabilities installed") || stdout.contains("registered"));

    // 2. Remove non-existent
    let output = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("remove")
        .arg("nonexistent")
        .arg("--force")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found in vault") || stderr.contains("Error:"),
        "Unexpected stderr: {}",
        stderr
    );

    // 3. Install same MCP twice
    let local_mcp_dir = tempdir().unwrap();
    let output1 = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("install")
        .arg(format!("local:{}", local_mcp_dir.path().display()))
        .arg("--name")
        .arg("duplicate-mcp")
        .output()
        .unwrap();
    assert!(output1.status.success());

    let output2 = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("install")
        .arg(format!("local:{}", local_mcp_dir.path().display()))
        .arg("--name")
        .arg("duplicate-mcp")
        .output()
        .unwrap();
    assert!(!output2.status.success());
    let stderr = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr.contains("already installed"),
        "Unexpected stderr: {}",
        stderr
    );
}

#[test]
fn test_e2e_serve_mcp() {
    use std::io::{BufRead, Write};
    let bin = get_bin_path();
    let temp_vault = tempdir().unwrap();
    let vault_dir = temp_vault.path().to_path_buf();

    // Initialize the vault first
    Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("init")
        .output()
        .unwrap();

    // Start serve command
    let mut child = Command::new(&bin)
        .arg("--vault-dir")
        .arg(&vault_dir)
        .arg("serve")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());

    // 1. Send initialize request
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    stdin
        .write_all(format!("{}\n", init_req).as_bytes())
        .unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    assert!(line.contains("protocolVersion"), "Response: {}", line);
    assert!(line.contains("agentvault"), "Response: {}", line);

    // 2. Send tools/list request
    line.clear();
    let list_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    stdin
        .write_all(format!("{}\n", list_req).as_bytes())
        .unwrap();
    stdin.flush().unwrap();

    stdout.read_line(&mut line).unwrap();
    assert!(line.contains("list_capabilities"), "Response: {}", line);
    assert!(line.contains("install_capability"), "Response: {}", line);

    // 3. Send tools/call for list_capabilities
    line.clear();
    let call_req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_capabilities","arguments":{}}}"#;
    stdin
        .write_all(format!("{}\n", call_req).as_bytes())
        .unwrap();
    stdin.flush().unwrap();

    stdout.read_line(&mut line).unwrap();
    assert!(line.contains("content"), "Response: {}", line);
    assert!(line.contains("mcps"), "Response: {}", line);

    // Shutdown by dropping stdin (EOF) or killing the child
    std::mem::drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
