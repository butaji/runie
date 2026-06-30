//! `runie mcp` — Manage MCP servers via ConfigActor.
//!
//! MCP servers are stored in `~/.runie/config.toml` under `[mcp]`.
//! ConfigActor handles file persistence; this module provides CLI interaction.

use anyhow::Result;
use std::collections::HashMap;

use runie_core::actors::config::messages::ConfigScope;
use runie_core::config::{McpServer, McpTransport};
use runie_core::event::Event;
use runie_core::bus::EventBus;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type McpHandle = runie_core::actors::RactorConfigHandle;

/// Spawn ConfigActor and return the handle after ConfigLoaded.
async fn spawn_config_actor() -> Result<McpHandle> {
    let bus = EventBus::<Event>::new(16);
    let (handle, _cell) = runie_core::actors::RactorConfigActor::spawn_default(bus.clone()).await;

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
        };
        println!("  {name} [{transport}]");
        println!("    command: {}", server.command.join(" "));
        if let Some(ref url) = server.url {
            println!("    url: {url}");
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
pub async fn add(
    name: String,
    command: Vec<String>,
    scope: ConfigScope,
) -> Result<()> {
    let handle = spawn_config_actor().await?;

    let server = McpServer {
        transport: McpTransport::Stdio,
        command,
        url: None,
        headers: HashMap::new(),
        scope: match scope {
            ConfigScope::Global => "global",
            ConfigScope::Project => "project",
        }.to_string(),
    };
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

#[cfg(test)]
mod tests {
    use runie_core::actors::config::messages::ConfigScope;
    use runie_core::config::{McpServer, McpTransport};
    use std::collections::HashMap;

    // Layer 1: McpServer construction
    #[test]
    fn mcp_server_builder() {
        let server = McpServer {
            transport: McpTransport::Stdio,
            command: vec!["npx".to_string(), "-y".to_string(), "@server".to_string()],
            url: None,
            headers: HashMap::new(),
            scope: "user".to_string(),
        };
        assert!(matches!(server.transport, McpTransport::Stdio));
        assert_eq!(server.command.len(), 3);
    }

    // Layer 1: ConfigScope variants
    #[test]
    fn config_scope_variants() {
        assert!(matches!(ConfigScope::Global, ConfigScope::Global));
        assert!(matches!(ConfigScope::Project, ConfigScope::Project));
    }
}
