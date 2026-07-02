//! # rmcp Client Spike
//!
//! Time-boxed investigation: can the rmcp client connect to an MCP server via stdio?
//!
//! ## Findings (2026-07-02)
//!
//! **Decision: Migrate** — rmcp client works for stdio transport.
//!
//! ### API Surface
//!
//! - `TokioChildProcess::new(command.configure(...))` — spawn subprocess with piped stdin/stdout.
//!   `configure()` takes a closure: `|cmd| { cmd.arg("x"); cmd.stdin(Stdio::piped()); ... }`.
//! - `serve_client((), transport)` — serves client with empty handler (`()` implements `ClientHandler`).
//!   - With `local` feature: returns `Result<RunningService, ClientInitializeError>` (sync)
//!   - Without `local`: returns `impl Future<Output = Result<RunningService, ClientInitializeError>>` (async, needs `.await`)
//! - `client.list_all_tools()` — sends JSON-RPC tools/list request → returns `Vec<Tool>`
//! - `client.cancel()` — graceful shutdown
//!
//! ### Feature Notes
//!
//! The `local` feature may or may not be enabled depending on rmcp-macros transitive features.
//! In this workspace, `local` is NOT enabled, so `serve_client()` returns `impl Future`
//! and needs `.await`.
//!
//! ### Next Steps
//!
//! - Replace the placeholder stdio protocol code in `connection.rs` with rmcp client calls.
//! - The existing `SchemaCache` can store tool schemas keyed by server config fingerprint.
//! - The existing `McpConnectionManager` spawns one `TokioChildProcess` per server config.

use std::process::Stdio;

use anyhow::Result;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use tempfile::TempDir;

#[tokio::test]
async fn rmcp_client_connects_to_echo_server() -> Result<()> {
    // Minimal echo Python MCP server for offline testing.
    let python_script = r#"
import sys, json
def read():
    l = sys.stdin.readline()
    return json.loads(l) if l else None
def send(cid, result=None, error=None):
    r = {"jsonrpc": "2.0", "id": cid}
    if error: r["error"] = error
    else: r["result"] = result
    sys.stdout.write(json.dumps(r)+"\n"); sys.stdout.flush()

# handshake
m = read()
if m and m.get("method") == "initialize":
    send(m["id"], {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": "echo", "version": "0.1"}})

# serve requests
for _ in range(5):
    m = read()
    if not m: break
    if m.get("method") == "tools/list":
        send(m["id"], {"tools": [{"name": "echo_test", "description": "Echo test", "inputSchema": {"type": "object"}}]})
    elif m.get("method") == "ping":
        send(m["id"], {})
"#;

    let temp = TempDir::new()?;
    let script_path = temp.path().join("echo.py");
    std::fs::write(&script_path, python_script)?;

    let transport = TokioChildProcess::new(
        tokio::process::Command::new("python3")
            .configure(|c| {
                c.arg(script_path);
                c.stdout(Stdio::piped());
                c.stdin(Stdio::piped());
                c.stderr(Stdio::piped());
            }),
    )?;

    // `()` implements `ClientHandler` and therefore `Service<RoleClient>`.
    // `serve_client((), transport)` connects and performs MCP handshake.
    let client = rmcp::serve_client((), transport).await?;

    // list_all_tools() sends JSON-RPC tools/list request.
    let tools = client.list_all_tools().await?;
    assert!(!tools.is_empty(), "Expected at least one tool from echo server");
    let tool_name: String = tools[0].name.clone().into_owned();
    assert_eq!(tool_name.as_str(), "echo_test");

    client.cancel().await?;
    Ok(())
}
