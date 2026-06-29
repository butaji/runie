//! `runie mcp` — MCP server management CLI
//!
//! Supports adding, listing, and removing MCP server configurations
//! for both user (`~/.runie/config.toml`) and project (`./.runie/config.toml`) scopes.

use anyhow::{Context, Result};
use runie_core::actors::{RactorConfigActor, RactorConfigHandle};
use runie_core::actors::config::messages::ConfigScope;
use runie_core::config::{Config, McpServer, McpTransport};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::McpCommand;

const PROJECT_CONFIG_DIR: &str = ".runie";
const PROJECT_CONFIG_FILE: &str = "config.toml";

/// Run the MCP CLI command (for clap-based CLI).
pub fn run_mcp(subcommand: Option<&McpCommand>) -> Result<()> {
    // Spawn a minimal runtime for the ConfigActor
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let (config_handle, _cell) = RactorConfigActor::spawn_default(
            runie_core::bus::EventBus::new(16),
        )
        .await;

        match subcommand {
            Some(McpCommand::List) => run_list_internal_async(&config_handle, ConfigScope::Global).await,
            Some(McpCommand::Add { name, command }) => {
                run_add_internal_async(&config_handle, name, command, ConfigScope::Global, McpTransport::Stdio).await
            }
            Some(McpCommand::Remove { name }) => {
                run_remove_internal_async(&config_handle, name, ConfigScope::Global).await
            }
            None => run_list_internal_async(&config_handle, ConfigScope::Global).await,
        }
    })
}

/// Run the MCP CLI command (legacy argv-based CLI).
#[allow(dead_code)]
pub fn run(args: &[String]) -> Result<()> {
    // Handle --help / -h before subcommand
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return Ok(());
    }

    let subcmd = args.first().map(|s| s.as_str()).unwrap_or("list");
    let rest = if args.len() > 1 { &args[1..] } else { &[] };

    match subcmd {
        "list" => run_list_internal_from_args(rest),
        "add" => run_add_from_args(rest),
        "remove" | "rm" => run_remove_from_args(rest),
        _ => {
            eprintln!("Unknown MCP command: {subcmd}");
            print_usage();
            std::process::exit(1);
        }
    }
}

#[allow(dead_code)]
fn print_usage() {
    eprintln!(
        "Usage: runie mcp <command> [args]\n\n\
Commands:\n\
  list                     List configured MCP servers\n\
  add <name> <command|url> Add an MCP server\n\
  remove <name>            Remove an MCP server\n\n\
Options:\n\
  --scope user|project       Config scope (default: user)\n\
  --transport stdio|http|sse Transport type (default: stdio)\n\
  -H, --header KEY=VALUE      Add HTTP header (for http/sse transports)"
    );
}

fn run_list_internal(scope: &str) -> Result<()> {
    let config = load_config_for_scope(scope)?;
    let servers = &config.mcp.servers;

    if servers.is_empty() {
        println!("No MCP servers configured for scope '{scope}'.");
        return Ok(());
    }

    println!("MCP servers [{scope}]:");
    println!("{:<20} {:<10} {}", "NAME", "TRANSPORT", "CONFIG");
    println!("{}", "-".repeat(60));

    for (name, server) in servers.iter() {
        let config_str = if server.command.is_empty() {
            server.url.clone().unwrap_or_default()
        } else {
            server.command.join(" ")
        };
        let transport_str = transport_name(&server.transport);
        println!("{:<20} {:<10} {}", name, transport_str, config_str);
    }

    Ok(())
}

/// Async version using ConfigActor.
async fn run_list_internal_async(config_handle: &RactorConfigHandle, scope: ConfigScope) -> Result<()> {
    let servers = config_handle.list_mcp_servers(scope).await;
    let scope_str = match scope {
        ConfigScope::Global => "user",
        ConfigScope::Project => "project",
    };

    if servers.is_empty() {
        println!("No MCP servers configured for scope '{}'.", scope_str);
        return Ok(());
    }

    println!("MCP servers [{}]:", scope_str);
    println!("{:<20} {:<10} {}", "NAME", "TRANSPORT", "CONFIG");
    println!("{}", "-".repeat(60));

    for (name, server) in servers.iter() {
        let config_str = if server.command.is_empty() {
            server.url.clone().unwrap_or_default()
        } else {
            server.command.join(" ")
        };
        let transport_str = transport_name(&server.transport);
        println!("{:<20} {:<10} {}", name, transport_str, config_str);
    }

    Ok(())
}

#[allow(dead_code)]
fn run_list_internal_from_args(args: &[String]) -> Result<()> {
    let scope = parse_scope(args).unwrap_or_else(|| "user".to_string());
    run_list_internal(&scope)
}

fn transport_name(t: &McpTransport) -> &'static str {
    match t {
        McpTransport::Stdio => "stdio",
        McpTransport::Http => "http",
        McpTransport::Sse => "sse",
    }
}

fn run_add_internal(name: &str, command: &str, scope: &str, transport: McpTransport) -> Result<()> {
    let server = build_server(transport, command, HashMap::new(), scope);
    let mut config = load_config_for_scope(scope)?;
    config.mcp.servers.insert(name.to_string(), server);
    save_config_for_scope(&config, scope)?;
    println!("Added MCP server '{}' to [{}] scope.", name, scope);
    Ok(())
}

/// Async version using ConfigActor.
async fn run_add_internal_async(
    config_handle: &RactorConfigHandle,
    name: &str,
    command: &str,
    scope: ConfigScope,
    transport: McpTransport,
) -> Result<()> {
    let scope_str = match scope {
        ConfigScope::Global => "user",
        ConfigScope::Project => "project",
    };
    let server = build_server(transport, command, HashMap::new(), scope_str);
    config_handle.add_mcp_server(scope, name.to_string(), server).await;
    println!("Added MCP server '{}' to [{}] scope.", name, scope_str);
    Ok(())
}

#[allow(dead_code)]
fn run_add_from_args(args: &[String]) -> Result<()> {
    let parsed = parse_add_args(args)?;
    run_add_internal(&parsed.name, &parsed.command, &parsed.scope, parsed.transport)
}

#[allow(dead_code)]
struct AddArgs {
    name: String,
    command: String,
    scope: String,
    transport: McpTransport,
}

#[allow(dead_code)]
fn parse_add_args(args: &[String]) -> Result<AddArgs> {
    let (name, scope, transport, _headers, positional) = parse_add_arg_pairs(args)?;
    let name = name.context("Server name required")?;
    let command = positional.join(" ");
    if command.is_empty() {
        anyhow::bail!("Command or URL required");
    }
    Ok(AddArgs { name, command, scope, transport })
}

#[allow(dead_code)]
fn parse_add_arg_pairs(args: &[String]) -> Result<(Option<String>, String, McpTransport, HashMap<String, String>, Vec<String>)> {
    let mut name = None;
    let mut scope = "user".to_string();
    let mut transport = McpTransport::Stdio;
    let mut headers = HashMap::new();
    let mut positional: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scope" => { ensure_arg(args, i + 1, "--scope")?; scope = args[i + 1].clone(); i += 2; }
            "--transport" | "-t" => { ensure_arg(args, i + 1, "--transport")?; transport = parse_transport(&args[i + 1])?; i += 2; }
            "-H" | "--header" => { ensure_arg(args, i + 1, "--header")?; add_header(&args[i + 1], &mut headers)?; i += 2; }
            _ => { if name.is_none() { name = Some(args[i].clone()); } else { positional.push(args[i].clone()); } i += 1; }
        }
    }

    Ok((name, scope, transport, headers, positional))
}

#[allow(dead_code)]
fn add_header(kv: &str, headers: &mut HashMap<String, String>) -> Result<()> {
    if let Some((k, v)) = kv.split_once('=') {
        headers.insert(k.to_string(), v.to_string());
        Ok(())
    } else {
        anyhow::bail!("Header must be in KEY=VALUE format")
    }
}

#[allow(dead_code)]
fn ensure_arg(args: &[String], idx: usize, flag: &str) -> Result<()> {
    if idx >= args.len() {
        anyhow::bail!("{flag} requires a value");
    }
    Ok(())
}

#[allow(dead_code)]
fn parse_transport(s: &str) -> Result<McpTransport> {
    match s.to_lowercase().as_str() {
        "stdio" => Ok(McpTransport::Stdio),
        "http" => Ok(McpTransport::Http),
        "sse" => Ok(McpTransport::Sse),
        _ => anyhow::bail!("Invalid transport: {s}"),
    }
}

fn build_server(
    transport: McpTransport,
    cmd_or_url: &str,
    headers: HashMap<String, String>,
    scope: &str,
) -> McpServer {
    match transport {
        McpTransport::Stdio => McpServer {
            transport,
            command: cmd_or_url.split_whitespace().map(String::from).collect(),
            url: None,
            headers: HashMap::new(),
            scope: scope.to_string(),
        },
        McpTransport::Http | McpTransport::Sse => McpServer {
            transport,
            command: Vec::new(),
            url: Some(cmd_or_url.to_string()),
            headers,
            scope: scope.to_string(),
        },
    }
}

fn run_remove_internal(name: &str, scope: &str) -> Result<()> {
    let mut config = load_config_for_scope(scope)?;

    if config.mcp.servers.remove(name).is_some() {
        save_config_for_scope(&config, scope)?;
        println!("Removed MCP server '{name}' from [{scope}] scope.");
    } else {
        anyhow::bail!("MCP server '{name}' not found in [{scope}] scope.");
    }

    Ok(())
}

/// Async version using ConfigActor.
async fn run_remove_internal_async(
    config_handle: &RactorConfigHandle,
    name: &str,
    scope: ConfigScope,
) -> Result<()> {
    let scope_str = match scope {
        ConfigScope::Global => "user",
        ConfigScope::Project => "project",
    };
    // First check if the server exists
    let servers = config_handle.list_mcp_servers(scope).await;
    if !servers.iter().any(|(n, _)| n == name) {
        anyhow::bail!("MCP server '{}' not found in [{}] scope.", name, scope_str);
    }
    config_handle.remove_mcp_server(scope, name.to_string()).await;
    println!("Removed MCP server '{}' from [{}] scope.", name, scope_str);
    Ok(())
}

#[allow(dead_code)]
fn run_remove_from_args(args: &[String]) -> Result<()> {
    let mut name = None;
    let mut scope = "user".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--scope" => {
                ensure_arg(args, i + 1, "--scope")?;
                scope = args[i + 1].clone();
                i += 2;
            }
            _ => {
                if name.is_none() {
                    name = Some(args[i].clone());
                }
                i += 1;
            }
        }
    }

    let name = name.context("Server name required")?;
    run_remove_internal(&name, &scope)
}

#[allow(dead_code)]
fn parse_scope(args: &[String]) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--scope" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

fn load_config_for_scope(scope: &str) -> Result<Config> {
    let path = config_path_for_scope(scope);
    if path.exists() {
        Ok(Config::load(Some(&path)))
    } else {
        Ok(Config::default())
    }
}

fn save_config_for_scope(config: &Config, scope: &str) -> Result<()> {
    let path = config_path_for_scope(scope);
    config.save_to(&path).with_context(|| {
        format!("Failed to save config to {}", path.to_string_lossy())
    })
}

fn config_path_for_scope(scope: &str) -> PathBuf {
    match scope {
        "project" => std::env::current_dir()
            .unwrap_or_default()
            .join(PROJECT_CONFIG_DIR)
            .join(PROJECT_CONFIG_FILE),
        _ => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".runie")
            .join("config.toml"),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::config::McpSection;

    #[test]
    fn transport_name_test() {
        assert_eq!(transport_name(&McpTransport::Stdio), "stdio");
        assert_eq!(transport_name(&McpTransport::Http), "http");
        assert_eq!(transport_name(&McpTransport::Sse), "sse");
    }

    #[test]
    fn parse_transport_test() {
        assert!(matches!(parse_transport("stdio"), Ok(McpTransport::Stdio)));
        assert!(matches!(parse_transport("HTTP"), Ok(McpTransport::Http)));
        assert!(matches!(parse_transport("sse"), Ok(McpTransport::Sse)));
        assert!(parse_transport("invalid").is_err());
    }

    #[test]
    fn add_header_test() {
        let mut headers = HashMap::new();
        add_header("Authorization=Bearer token", &mut headers).unwrap();
        assert_eq!(headers.get("Authorization"), Some(&"Bearer token".to_string()));
    }

    #[test]
    fn build_server_stdio() {
        let server = build_server(McpTransport::Stdio, "npx -y @server", HashMap::new(), "user");
        assert!(matches!(server.transport, McpTransport::Stdio));
        assert_eq!(server.command, vec!["npx", "-y", "@server"]);
        assert_eq!(server.scope, "user");
    }

    #[test]
    fn build_server_http() {
        let headers = [("X-Custom".to_string(), "value".to_string())]
            .into_iter()
            .collect();
        let server = build_server(McpTransport::Http, "https://api.example.com", headers, "project");
        assert!(matches!(server.transport, McpTransport::Http));
        assert!(server.command.is_empty());
        assert_eq!(server.url.as_deref(), Some("https://api.example.com"));
        assert_eq!(server.scope, "project");
    }

    #[test]
    fn mcp_transport_serialization() {
        assert_eq!(serde_json::to_string(&McpTransport::Stdio).unwrap(), "\"stdio\"");
        assert_eq!(serde_json::to_string(&McpTransport::Http).unwrap(), "\"http\"");
        assert_eq!(serde_json::to_string(&McpTransport::Sse).unwrap(), "\"sse\"");
    }

    #[test]
    fn mcp_server_serialization() {
        let server = McpServer {
            transport: McpTransport::Stdio,
            command: vec!["npx".to_string(), "-y".to_string(), "@server".to_string()],
            url: None,
            headers: HashMap::new(),
            scope: "user".to_string(),
        };
        let json = serde_json::to_string_pretty(&server).unwrap();
        assert!(json.contains("\"command\""));
        assert!(json.contains("npx"));
        let back: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.command, server.command);
        assert_eq!(back.scope, "user");
    }

    #[test]
    fn mcp_section_default() {
        let section = McpSection::default();
        assert!(section.servers.is_empty());
    }

    #[test]
    fn parse_scope_from_args() {
        assert_eq!(parse_scope(&["--scope".to_string(), "project".to_string()]), Some("project".to_string()));
        assert_eq!(parse_scope(&["--scope".to_string(), "user".to_string(), "extra".to_string()]), Some("user".to_string()));
        assert_eq!(parse_scope(&["foo".to_string()]), None);
    }

    #[test]
    fn mcp_server_http_with_headers() {
        let server = McpServer {
            transport: McpTransport::Http,
            command: Vec::new(),
            url: Some("https://api.example.com/mcp".to_string()),
            headers: [("Authorization".to_string(), "Bearer token123".to_string())].into_iter().collect(),
            scope: "project".to_string(),
        };
        let json = serde_json::to_string_pretty(&server).unwrap();
        assert!(json.contains("Authorization"));
        assert!(json.contains("Bearer token123"));
        let back: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.headers.get("Authorization"), Some(&"Bearer token123".to_string()));
    }

    #[test]
    fn parse_add_args_basic() {
        let args = &["my-server".to_string(), "npx".to_string(), "-y".to_string(), "@server".to_string()];
        let result = parse_add_args(args).unwrap();
        assert_eq!(result.name, "my-server");
        assert_eq!(result.scope, "user");
    }

    #[test]
    fn parse_add_args_with_options() {
        let args = &["--scope".to_string(), "project".to_string(), "--transport".to_string(), "http".to_string(), "my-server".to_string(), "https://api.example.com".to_string()];
        let result = parse_add_args(args).unwrap();
        assert_eq!(result.name, "my-server");
        assert_eq!(result.scope, "project");
        assert!(matches!(result.transport, McpTransport::Http));
    }
}
