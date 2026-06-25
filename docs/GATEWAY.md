# MCP Gateway Mode

AgentVault can act as a unified **MCP-to-MCP gateway**, aggregating all your installed MCP servers behind a single stdio endpoint. Instead of configuring each AI agent with every MCP server individually, you point your agent at AgentVault's gateway and get access to everything.

## Quick Start

```bash
vault serve --gateway
```

This will:

1. Read all installed MCP servers from the vault database
2. Spawn every **Active** + **Stdio** MCP server as a child process
3. Perform the JSON-RPC `initialize` handshake with each child
4. Aggregate all their tools under namespaced names
5. Listen on stdin for incoming JSON-RPC requests

## How It Works

### Architecture

```
┌──────────────┐    stdio     ┌───────────────────────┐
│   AI Agent   │◄────────────►│  vault serve          │
│ (Claude/etc) │              │  --gateway            │
└──────────────┘              └──────┬────────────────┘
                                     │ spawns & manages
                    ┌────────────────┼──────────────────┐
                    │                │                   │
              ┌─────▼──────┐  ┌──────▼───────┐  ┌───────▼───────┐
              │ brave-mcp   │  │ filesystem   │  │ memory-mcp    │
              │ (child)     │  │ (child)      │  │ (child)       │
              └────────────┘  └──────────────┘  └───────────────┘
```

### Tool Namespacing

To prevent tool name collisions, all child server tools are prefixed with the server name:

| Server Name | Original Tool | Gateway Tool Name |
|-------------|---------------|-------------------|
| `brave-search` | `web_search` | `brave-search__web_search` |
| `filesystem` | `read_file` | `filesystem__read_file` |
| `memory` | `store` | `memory__store` |

### Request Routing

When a `tools/call` request arrives:

1. If the tool name contains `__` and doesn't start with `vault__`, it's routed to the appropriate child server
2. Otherwise, it's handled as a vault management tool (e.g., `install_capability`, `list_capabilities`)

### Concurrency Safety

- Each child server's stdin/stdout is protected by a `tokio::sync::Mutex`
- This prevents JSON-RPC message interleaving when multiple tool calls are in flight
- Request IDs are managed via `AtomicU64` to ensure uniqueness

## Dynamic Lifecycle

The gateway automatically manages child server lifecycle:

| Event | Action |
|-------|--------|
| `install_capability` (MCP) | Spawns the new MCP, registers its tools, sends `notifications/tools/list_changed` |
| `remove_capability` (MCP) | Shuts down the child process, unregisters tools, sends notification |
| `update_capability` (MCP) | Shuts down old instance, spawns updated version, sends notification |
| stdin EOF | Gracefully shuts down all child processes |

## Management Tools

When running in gateway mode, you get 8 management tools:

| Tool | Description |
|------|-------------|
| `list_capabilities` | List all installed MCPs, skills, workflows + active gateway servers |
| `install_capability` | Install from npm/pypi/github/local, auto-spawn in gateway |
| `remove_capability` | Remove and auto-shutdown child |
| `update_capability` | Update and auto-restart child |
| `get_capability_details` | Full metadata with masked env vars |
| `set_capability_env` | Set environment variables |
| `search_registry` | Search npm for available MCPs |
| `doctor_check` | Health diagnostics with gateway status |

## Agent Configuration

### Claude Code

Add to your Claude Code MCP config:

```json
{
  "mcpServers": {
    "agentvault": {
      "command": "vault",
      "args": ["serve", "--gateway"]
    }
  }
}
```

### Gemini CLI

Add to your Gemini CLI settings:

```json
{
  "mcpServers": {
    "agentvault": {
      "command": "vault",
      "args": ["serve", "--gateway"]
    }
  }
}
```

## Resource Considerations

- Each child MCP server runs as a separate OS process
- Idle servers consume minimal resources (blocked on stdin read)
- The gateway itself adds ~5-10MB RSS overhead
- All child processes are killed on gateway shutdown (no orphans)

## Non-Gateway Mode

Without `--gateway`, `vault serve` still works as a standard MCP server exposing only the vault management tools. No child processes are spawned.

```bash
vault serve  # Management tools only, no child servers
```
