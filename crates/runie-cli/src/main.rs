//! runie-cli — Unified headless CLI for Runie
//!
//! Supports modes:
//! - `runie print <prompt>` — single-turn streaming to stdout
//! - `runie inspect` — print runtime configuration discovered for the current directory
//! - `runie json` — structured JSON stdin/stdout for scripting
//! - `runie server` — TCP/stdio JSON-RPC server for IDE integration
//! - `runie mcp` — manage MCP servers (list, add, remove)
//! - `runie completion` — generate shell completions (requires --features completions)

use anyhow::Result;
use clap::Parser;

mod inspect;
mod json;
mod login;
mod mcp;
mod print;
mod scope; // Required for ConfigScope ValueEnum impl
mod server;
pub mod transport;

#[cfg(feature = "completions")]
mod completion;

/// Runie CLI — Terminal-native coding agent harness
#[derive(Parser, Debug)]
#[command(name = "runie")]
#[command(about = "Terminal-native coding agent harness", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Stream LLM response to stdout
    Print {
        /// The prompt to send to the LLM
        prompt: String,
        /// Enable OS-level sandboxing for bash tool execution
        #[arg(long)]
        sandbox: bool,
    },
    /// Show runtime configuration discovered for current directory
    Inspect {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Configure a provider with API key
    Login {
        /// Provider name (e.g., openai, anthropic, minimax)
        #[arg(short, long)]
        provider: Option<String>,
        /// API key (will prompt if not provided)
        #[arg(short, long)]
        api_key: Option<String>,
    },
    /// JSON stdin/stdout for scripting
    Json,
    /// TCP/stdio JSON-RPC server
    Server {
        /// Use stdio transport instead of TCP
        #[arg(long)]
        stdio: bool,
        /// Skip permission checks (for testing/automation)
        #[arg(long)]
        yolo: bool,
    },
    /// Manage MCP (Model Context Protocol) servers
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Generate shell completions
    #[cfg(feature = "completions")]
    Completion {
        /// Shell to generate completions for (bash, elvish, fish, powershell, zsh)
        #[arg(default_value = "bash", value_parser = clap::builder::PossibleValuesParser::new(["bash", "zsh", "fish", "powershell", "elvish"]))]
        shell: String,
    },
}

#[derive(Parser, Debug)]
enum McpCommand {
    /// List configured MCP servers
    List,
    /// Add an MCP server
    Add {
        /// Server name (e.g., "filesystem")
        name: String,
        /// Scope: global (default, ~/.runie/config.toml) or project (.runie/config.toml)
        #[arg(long, default_value = "global")]
        scope: scope::ConfigScopeValue,
        /// Command to run (e.g., "npx" "-y" "@modelcontextprotocol/server-filesystem")
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Remove an MCP server
    Remove {
        /// Server name
        name: String,
        /// Scope: global (default) or project
        #[arg(long, default_value = "global")]
        scope: scope::ConfigScopeValue,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // Install human-panic hook for crash reports.
    human_panic::setup_panic!();

    // Initialize tracing subscriber.
    runie_core::tracing_init::init();

    // Install color-eyre for better error chains.
    color_eyre::install().expect("Failed to install color-eyre hook");

    let cli = Cli::parse();

    let result = match cli.command {
        Command::Print { prompt, sandbox } => run_print(&prompt, sandbox).await,
        Command::Inspect { json } => run_inspect(json).await,
        Command::Login { provider, api_key } => run_login(provider, api_key).await,
        Command::Json => run_json().await,
        Command::Server { stdio, yolo } => run_server(stdio, yolo).await,
        Command::Mcp { command } => run_mcp(command).await,
        #[cfg(feature = "completions")]
        Command::Completion { shell } => crate::completion::run_completion(&shell),
    };

    if let Err(e) = result {
        // Log the full error chain with color-eyre formatting.
        tracing::error!("CLI command failed: {:#}", e);
        tracing::error!(
            "Hint: Set RUST_LOG=debug for more details, or RUST_LOG=trace for verbose output."
        );
        std::process::exit(1);
    }
}

async fn run_inspect(json: bool) -> Result<()> {
    inspect::run(json).await
}

async fn run_login(provider: Option<String>, api_key: Option<String>) -> Result<()> {
    login::run(provider, api_key).await
}

async fn run_print(prompt: &str, sandbox: bool) -> Result<()> {
    print::run(prompt, sandbox).await
}

async fn run_json() -> Result<()> {
    json::run().await
}

async fn run_server(use_stdio: bool, yolo: bool) -> Result<()> {
    server::run(use_stdio, yolo).await
}

async fn run_mcp(cmd: McpCommand) -> Result<()> {
    match cmd {
        McpCommand::List => mcp::list().await,
        McpCommand::Add {
            name,
            command,
            scope,
        } => mcp::add(name, command, scope.0).await,
        McpCommand::Remove { name, scope } => mcp::remove(name, scope.0).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // Layer 1: CLI parsing
    #[test]
    fn cli_parses_print() {
        let cli = Cli::try_parse_from(["runie", "print", "hello world"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Print {
                prompt: _,
                sandbox: false
            }
        ));
    }

    #[test]
    fn cli_parses_print_with_sandbox() {
        let cli = Cli::try_parse_from(["runie", "print", "--sandbox", "hello world"]).unwrap();
        match cli.command {
            Command::Print { prompt, sandbox } => {
                assert_eq!(prompt, "hello world");
                assert!(sandbox);
            }
            _ => panic!("Expected Print command"),
        }
    }

    #[test]
    fn cli_parses_inspect() {
        let cli = Cli::try_parse_from(["runie", "inspect"]).unwrap();
        assert!(matches!(cli.command, Command::Inspect { json: false }));
    }

    #[test]
    fn cli_parses_inspect_json() {
        let cli = Cli::try_parse_from(["runie", "inspect", "--json"]).unwrap();
        assert!(matches!(cli.command, Command::Inspect { json: true }));
    }

    #[test]
    fn cli_parses_json_mode() {
        let cli = Cli::try_parse_from(["runie", "json"]).unwrap();
        assert!(matches!(cli.command, Command::Json));
    }

    #[test]
    fn cli_parses_server() {
        let cli = Cli::try_parse_from(["runie", "server"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Server {
                stdio: false,
                yolo: false
            }
        ));
    }

    #[test]
    fn cli_parses_mcp_list() {
        let cli = Cli::try_parse_from(["runie", "mcp", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Mcp {
                command: McpCommand::List
            }
        ));
    }

    #[test]
    fn cli_parses_mcp_add() {
        let cli = Cli::try_parse_from([
            "runie",
            "mcp",
            "add",
            "my-server",
            "--",
            "npx",
            "-y",
            "@server",
        ])
        .unwrap();
        match cli.command {
            Command::Mcp {
                command:
                    McpCommand::Add {
                        name,
                        command,
                        scope,
                    },
            } => {
                assert_eq!(name, "my-server");
                assert_eq!(command, vec!["npx", "-y", "@server"]);
                assert_eq!(scope.0, runie_core::config::ConfigScope::Global);
            }
            _ => panic!("Expected Mcp::Add"),
        }
    }

    #[test]
    fn cli_parses_mcp_add_project_scope() {
        let cli = Cli::try_parse_from([
            "runie",
            "mcp",
            "add",
            "my-server",
            "--scope",
            "project",
            "--",
            "npx",
            "@server",
        ])
        .unwrap();
        match cli.command {
            Command::Mcp {
                command:
                    McpCommand::Add {
                        name,
                        scope,
                        command,
                    },
            } => {
                assert_eq!(name, "my-server");
                assert_eq!(scope.0, runie_core::config::ConfigScope::Project);
                assert_eq!(command, vec!["npx", "@server"]);
            }
            _ => panic!("Expected Mcp::Add"),
        }
    }

    #[test]
    fn cli_parses_mcp_remove() {
        let cli = Cli::try_parse_from(["runie", "mcp", "remove", "my-server"]).unwrap();
        match cli.command {
            Command::Mcp {
                command: McpCommand::Remove { name, scope },
            } => {
                assert_eq!(name, "my-server");
                assert_eq!(scope.0, runie_core::config::ConfigScope::Global);
            }
            _ => panic!("Expected Mcp::Remove"),
        }
    }

    #[test]
    fn cli_rejects_unknown_subcommand() {
        let result = Cli::try_parse_from(["runie", "unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_help_includes_all_commands() {
        let help = Cli::command().render_help().to_string();
        assert!(help.contains("print"), "help should include print");
        assert!(help.contains("inspect"), "help should include inspect");
        assert!(help.contains("login"), "help should include login");
        assert!(help.contains("json"), "help should include json");
        assert!(help.contains("server"), "help should include server");
        assert!(help.contains("mcp"), "help should include mcp");
    }

    #[test]
    fn cli_parses_login() {
        let cli = Cli::try_parse_from(["runie", "login"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Login {
                provider: None,
                api_key: None
            }
        ));
    }

    #[test]
    fn cli_parses_login_with_provider() {
        let cli = Cli::try_parse_from(["runie", "login", "--provider", "openai"]).unwrap();
        match cli.command {
            Command::Login { provider, api_key } => {
                assert_eq!(provider, Some("openai".to_string()));
                assert_eq!(api_key, None);
            }
            _ => panic!("Expected Login command"),
        }
    }

    #[test]
    fn cli_parses_login_with_short_flags() {
        let cli =
            Cli::try_parse_from(["runie", "login", "-p", "anthropic", "-a", "sk-test"]).unwrap();
        match cli.command {
            Command::Login { provider, api_key } => {
                assert_eq!(provider, Some("anthropic".to_string()));
                assert_eq!(api_key, Some("sk-test".to_string()));
            }
            _ => panic!("Expected Login command"),
        }
    }
}
