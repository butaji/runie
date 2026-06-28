//! runie-cli — Unified headless CLI for Runie
//!
//! Supports modes:
//! - `runie print <prompt>` — single-turn streaming to stdout
//! - `runie inspect` — print runtime configuration discovered for the current directory
//! - `runie json` — structured JSON stdin/stdout for scripting
//! - `runie server` — TCP/stdio JSON-RPC server for IDE integration
//! - `runie acp` — ACP (Agent Client Protocol) over stdio for programmatic control

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod print;
mod json;
mod server;
mod acp;
mod inspect;
mod mcp;

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
    },
    /// Show runtime configuration discovered for current directory
    Inspect {
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    /// ACP (Agent Client Protocol) over stdio
    Acp,
    /// Manage MCP servers
    Mcp {
        /// MCP subcommand
        #[command(subcommand)]
        subcommand: Option<McpCommand>,
    },
}

#[derive(Parser, Debug)]
enum McpCommand {
    /// List configured MCP servers
    List,
    /// Add an MCP server
    Add {
        /// Server name
        name: String,
        /// Command to run
        command: String,
    },
    /// Remove an MCP server
    Remove {
        /// Server name
        name: String,
    },
}

fn main() {
    // Initialize tracing subscriber with EnvFilter from RUST_LOG (defaults to info).
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Command::Print { prompt } => run_print(&prompt),
        Command::Inspect { json } => run_inspect(json),
        Command::Json => block_on(run_json()),
        Command::Server { stdio, yolo } => block_on(run_server(stdio, yolo)),
        Command::Acp => block_on(acp::run()),
        Command::Mcp { subcommand } => mcp::run_mcp(subcommand.as_ref()),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn run_inspect(json: bool) -> Result<()> {
    inspect::run(json)
}

fn run_print(prompt: &str) -> Result<()> {
    print::run(prompt)
}

async fn run_json() -> Result<()> {
    json::run().await
}

async fn run_server(use_stdio: bool, yolo: bool) -> Result<()> {
    server::run(use_stdio, yolo).await
}
