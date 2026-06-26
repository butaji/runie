//! runie-cli — Unified headless CLI for Runie
//!
//! Supports modes:
//! - `runie print <prompt>` — single-turn streaming to stdout
//! - `runie inspect` — print runtime configuration discovered for the current directory
//! - `runie json` — structured JSON stdin/stdout for scripting
//! - `runie server` — TCP/stdio JSON-RPC server for IDE integration
//! - `runie acp` — ACP (Agent Client Protocol) over stdio for programmatic control
//!
//! Dispatch is based on `argv[1]` (subcommand) or `--mode` flag.

use anyhow::Result;

mod print;
mod json;
mod server;
mod acp;
mod inspect;

fn print_usage() {
    eprintln!(
        "Usage: runie <command> [args]

Commands:
  print <prompt>    Stream LLM response to stdout
  inspect           Show runtime configuration for current directory
  json              JSON stdin/stdout for scripting
  server            TCP/stdio JSON-RPC server
  acp               ACP (Agent Client Protocol) over stdio

Options:
  --help, -h        Show this help
  --version         Show version"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }
    let result = match args[1].as_str() {
        "--help" | "-h" => { print_usage(); return; }
        "--version" => { println!("runie-cli {}", env!("CARGO_PKG_VERSION")); return; }
        "print" => run_print(&args[2..]),
        "inspect" => run_inspect(&args[2..]),
        "json" => block_on(run_json()),
        "server" => block_on(run_server(&args[2..])),
        "acp" => block_on(acp::run()),
        other => { eprintln!("Unknown command: {other}"); print_usage(); std::process::exit(1); }
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

fn run_inspect(args: &[String]) -> Result<()> {
    let json = args.iter().any(|a| a == "--json");
    inspect::run(json)
}

fn run_print(args: &[String]) -> Result<()> {
    let prompt = match args.first() {
        Some(p) => p.as_str(),
        None => {
            eprintln!("Usage: runie print <prompt>");
            std::process::exit(1);
        }
    };
    print::run(prompt)
}

async fn run_json() -> Result<()> {
    json::run().await
}

async fn run_server(args: &[String]) -> Result<()> {
    let use_stdio = args.iter().any(|a| a == "--stdio");
    let yolo = args.iter().any(|a| a == "--yolo");
    server::run(use_stdio, yolo).await
}
