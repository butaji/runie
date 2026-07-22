//! `runie mcp` — Manage MCP servers via ConfigActor.
//!
//! MCP servers are stored in `~/.runie/config.toml` under `[mcp]`.
//! ConfigActor handles file persistence; this module provides CLI interaction.

use anyhow::Result;
use std::collections::HashMap;

use runie_core::bus::EventBus;
use runie_core::config::{ConfigScope, McpServer, McpTransport};
use runie_core::event::Event;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type McpHandle = runie_core::actors::RactorConfigHandle;

/// Spawn ConfigActor and return the handle after ConfigLoaded.
async fn spawn_config_actor() -> Result<McpHandle> {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell, _join) = runie_core::actors::RactorConfigActor::spawn_default(bus.clone())
        .await
        .unwrap();

    // Wait for ConfigLoaded
    let mut sub = bus.subscribe();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let timeout = deadline - tokio::time::Instant::now();
        if timeout.is_zero() {
            break;
        }
        match tokio::time::timeout(timeout, sub.recv()).await {
            Ok(Ok(evt)) => {
                if matches!(evt, Event::ConfigLoaded { .. }) {
                    break;
                }
            }
            _ => break,
        }
    }
    Ok(handle)
}

/// Print a formatted list of MCP servers.
fn print_servers(servers: &[(String, McpServer)], scope_name: &str) {
    if servers.is_empty() {
        println!("  (no MCP servers configured in {scope_name} config)");
        return;
    }
    for (name, server) in servers {
        let transport = match server.transport {
            McpTransport::Stdio => "stdio",
            McpTransport::Http => "http",
            McpTransport::Sse => "sse",
            McpTransport::WebSocket => "websocket",
        };
        println!("  {name} [{transport}]");
        if !server.command.is_empty() {
            println!("    command: {}", server.command.join(" "));
        }
        if let Some(ref url) = server.url {
            println!("    url: {url}");
        }
        if !server.headers.is_empty() {
            println!("    headers: {:?}", server.headers);
        }
        println!("    scope: {}", server.scope);
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Run the `mcp list` subcommand.
pub async fn list() -> Result<()> {
    let handle = spawn_config_actor().await?;

    println!("MCP Servers:");
    println!("\n  Global:");
    let global = handle.list_mcp_servers(ConfigScope::Global).await;
    print_servers(&global, "global");

    println!("\n  Project:");
    let project = handle.list_mcp_servers(ConfigScope::Project).await;
    print_servers(&project, "project");

    Ok(())
}

/// Run the `mcp add` subcommand.
pub async fn add(name: String, command: Vec<String>, scope: ConfigScope) -> Result<()> {
    let handle = spawn_config_actor().await?;

    let server = McpServer { transport: McpTransport::Stdio, command, url: None, headers: HashMap::new(), scope };
    handle.add_mcp_server(scope, name.clone(), server).await;
    println!("Added MCP server '{name}'.");
    Ok(())
}

/// Run the `mcp remove` subcommand.
pub async fn remove(name: String, scope: ConfigScope) -> Result<()> {
    let handle = spawn_config_actor().await?;
    handle.remove_mcp_server(scope, name.clone()).await;
    println!("Removed MCP server '{name}'.");
    Ok(())
}

/// Run the `mcp wizard` subcommand - interactive setup for all transport types.
#[allow(clippy::too_many_lines)]
pub async fn wizard(scope: ConfigScope) -> Result<()> {
    use std::io::{self, Write};

    println!("\n=== Runie MCP Server Wizard ===\n");

    // Step 1: Get server name
    print!("Server name: ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("Server name cannot be empty");
    }

    // Step 2: Select transport type
    println!("\nTransport types:");
    println!("  1. stdio     - Local command (default)");
    println!("  2. http      - HTTP endpoint");
    println!("  3. sse       - Server-Sent Events");
    println!("  4. websocket - WebSocket connection");
    print!("\nSelect transport type [1-4]: ");
    io::stdout().flush()?;
    let mut transport_choice = String::new();
    io::stdin().read_line(&mut transport_choice)?;
    let transport_choice = transport_choice.trim().to_string();

    let transport = match transport_choice.as_str() {
        "1" | "" => McpTransport::Stdio,
        "2" => McpTransport::Http,
        "3" => McpTransport::Sse,
        "4" => McpTransport::WebSocket,
        _ => {
            println!("Invalid choice, defaulting to stdio");
            McpTransport::Stdio
        }
    };

    // Step 3: Get transport-specific config
    let (command, url, headers) = match transport {
        McpTransport::Stdio => {
            println!("\nEnter command and arguments (e.g., 'npx -y @modelcontextprotocol/server-filesystem'):");
            print!("Command: ");
            io::stdout().flush()?;
            let mut cmd_line = String::new();
            io::stdin().read_line(&mut cmd_line)?;
            let cmd_line = cmd_line.trim().to_string();
            let command: Vec<String> = if cmd_line.is_empty() {
                Vec::new()
            } else {
                cmd_line.split_whitespace().map(|s| s.to_string()).collect()
            };
            (command, None, HashMap::new())
        }
        McpTransport::Http | McpTransport::Sse | McpTransport::WebSocket => {
            println!("\nEnter server URL (e.g., https://api.example.com/mcp or ws://localhost:8080/mcp):");
            print!("URL: ");
            io::stdout().flush()?;
            let mut url = String::new();
            io::stdin().read_line(&mut url)?;
            let url = url.trim().to_string();
            if url.is_empty() {
                anyhow::bail!("URL cannot be empty for HTTP/SSE/WebSocket transport");
            }

            // Optional headers for HTTP/SSE
            let mut headers = HashMap::new();
            if matches!(transport, McpTransport::Http | McpTransport::Sse) {
                println!("\nAdd HTTP headers? (Y/n)");
                print!("Add headers: ");
                io::stdout().flush()?;
                let mut add_headers = String::new();
                io::stdin().read_line(&mut add_headers)?;
                if add_headers.trim().to_lowercase() != "n" && add_headers.trim().to_lowercase() != "no" {
                    loop {
                        print!("Header name (or Enter to finish): ");
                        io::stdout().flush()?;
                        let mut key = String::new();
                        io::stdin().read_line(&mut key)?;
                        let key = key.trim().to_string();
                        if key.is_empty() {
                            break;
                        }
                        print!("Header value: ");
                        io::stdout().flush()?;
                        let mut value = String::new();
                        io::stdin().read_line(&mut value)?;
                        let value = value.trim().to_string();
                        headers.insert(key, value);
                    }
                }
            }

            (Vec::new(), Some(url), headers)
        }
    };

    // Create and save the server config
    let handle = spawn_config_actor().await?;
    let server = McpServer { transport, command, url, headers, scope };
    handle.add_mcp_server(scope, name.clone(), server).await;
    println!("\n✅ MCP server '{name}' added successfully!");
    println!("   Use `runie mcp list` to verify.");
    Ok(())
}

/// Run the `mcp status` subcommand - check connection status of all servers.
pub async fn status() -> Result<()> {
    let handle = spawn_config_actor().await?;

    println!("MCP Server Status:\n");

    println!("  Global:");
    let global = handle.list_mcp_servers(ConfigScope::Global).await;
    for (name, server) in global {
        let transport = match server.transport {
            McpTransport::Stdio => "stdio",
            McpTransport::Http => "http",
            McpTransport::Sse => "sse",
            McpTransport::WebSocket => "websocket",
        };
        println!("    {name} [{transport}] - configured");
    }

    println!("\n  Project:");
    let project = handle.list_mcp_servers(ConfigScope::Project).await;
    for (name, server) in project {
        let transport = match server.transport {
            McpTransport::Stdio => "stdio",
            McpTransport::Http => "http",
            McpTransport::Sse => "sse",
            McpTransport::WebSocket => "websocket",
        };
        println!("    {name} [{transport}] - configured");
    }

    println!("\nNote: Connection status will be checked when the agent starts.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use runie_core::config::{ConfigScope, McpServer, McpTransport};
    use std::collections::HashMap;

    // Layer 1: McpServer construction
    #[test]
    fn mcp_server_builder() {
        let server = McpServer {
            transport: McpTransport::Stdio,
            command: vec!["npx".to_string(), "-y".to_string(), "@server".to_string()],
            url: None,
            headers: HashMap::new(),
            scope: ConfigScope::Global,
        };
        assert!(matches!(server.transport, McpTransport::Stdio));
        assert_eq!(server.command.len(), 3);
    }

    #[test]
    fn mcp_server_http_with_headers() {
        let server = McpServer {
            transport: McpTransport::Http,
            command: Vec::new(),
            url: Some("https://api.example.com/mcp".to_string()),
            headers: [("Authorization".to_string(), "Bearer token".to_string())]
                .into_iter()
                .collect(),
            scope: ConfigScope::Global,
        };
        assert!(matches!(server.transport, McpTransport::Http));
        assert_eq!(server.headers.len(), 1);
    }

    #[test]
    fn mcp_server_websocket() {
        let server = McpServer {
            transport: McpTransport::WebSocket,
            command: Vec::new(),
            url: Some("ws://localhost:8080/mcp".to_string()),
            headers: HashMap::new(),
            scope: ConfigScope::Project,
        };
        assert!(matches!(server.transport, McpTransport::WebSocket));
        assert!(server.url.is_some());
    }

    // Layer 1: ConfigScope variants
    #[test]
    fn config_scope_variants() {
        assert!(matches!(ConfigScope::Global, ConfigScope::Global));
        assert!(matches!(ConfigScope::Project, ConfigScope::Project));
    }
}
