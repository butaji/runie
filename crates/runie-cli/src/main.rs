mod acp;
mod agent_spawn;
mod context_loader;
mod event_logger;
mod event_stream;
mod git;
mod headless;
mod logging;
mod provider_factory;
mod settings;
mod tui_run;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use settings::{CliSettings, Settings};

use crate::acp::run_acp_stdio;
use crate::event_stream::EventStreamLogger;
use crate::headless::{is_headless, run_headless, build_headless_settings, run_cli_one_shot};

#[derive(Parser, Clone)]
#[command(name = "runie")]
#[command(about = "Tidy coding harness — AI agent toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Workspace directory
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Custom config/data directory (default: ~/.runie)
    #[arg(long)]
    dev_folder: Option<PathBuf>,

    /// Session ID to resume
    #[arg(short = 'S', long)]
    session: Option<String>,

    /// Use mock provider for testing (no API key needed)
    #[arg(long)]
    mock: bool,

    /// Run onboarding wizard with mock provider
    #[arg(long)]
    mock_setup: bool,

    // === Headless mode flags ===

    /// Send one prompt (alias for `run` subcommand as a flag)
    #[arg(short = 'p', long, help_heading = "Headless Mode")]
    single: Option<String>,

    /// Choose model
    #[arg(short = 'm', long, help_heading = "Headless Mode")]
    model: Option<String>,

    /// Create or resume named session
    #[arg(short = 's', long, help_heading = "Headless Mode")]
    session_id: Option<String>,

    /// Resume existing session by ID
    #[arg(short = 'r', long, help_heading = "Headless Mode")]
    resume: Option<String>,

    /// Continue most recent session
    #[arg(short = 'c', long, help_heading = "Headless Mode")]
    continue_: bool,

    /// Set working directory
    #[arg(long, help_heading = "Headless Mode")]
    cwd: Option<PathBuf>,

    /// Output format: plain, json, or streaming-json
    #[arg(long, help_heading = "Headless Mode", default_value = "plain")]
    output_format: headless::OutputFormat,

    /// Auto-approve tool executions
    #[arg(long, help_heading = "Headless Mode")]
    always_approve: bool,

    /// Run inline without TUI (no alternate screen)
    #[arg(long, help_heading = "Headless Mode")]
    no_alt_screen: bool,

    /// Suppress auto-updates
    #[arg(long, help_heading = "Headless Mode")]
    no_auto_update: bool,
}

impl From<&Cli> for CliSettings {
    fn from(_cli: &Cli) -> Self {
        Self::default()
    }
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Run a single prompt without entering TUI (CLI)
    Run { prompt: String },

    /// ACP (Agent Protocol) mode
    Agent {
        #[command(subcommand)]
        subcommand: AgentCommands,
    },
}

#[derive(Subcommand, Clone)]
enum AgentCommands {
    /// Run as ACP agent over JSON-RPC on stdin/stdout
    Stdio,
}

fn init_environment(cli: &Cli) {
    if let Some(dev) = &cli.dev_folder {
        std::env::set_var("RUNIE_HOME", dev.as_os_str());
    }
    settings::ensure_dirs();
    if let Some(path) = settings::config_path() {
        settings::create_default_config(&path);
    }
}

fn init_event_logging() -> Option<EventStreamLogger> {
    let runie_dir = settings::runie_dir()?;
    logging::init_logging(&runie_dir);
    let logs_dir = runie_dir.join("logs");
    event_logger::init_event_logger(&logs_dir);
    Some(EventStreamLogger::new(&runie_dir))
}

fn parse_cli_args() -> Cli {
    Cli::parse()
}

async fn init_and_run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = parse_cli_args();
    init_environment(&cli);
    let event_logger = init_event_logging();
    let mut settings = Settings::load();

    // Handle ACP agent mode
    if let Some(Commands::Agent { subcommand: AgentCommands::Stdio }) = &cli.command {
        run_acp_stdio().await?;
        return Ok(());
    }

    // Handle headless mode (flags present or single prompt)
    if is_headless(&cli) || cli.single.is_some() {
        return run_headless_mode(&cli, &mut settings).await;
    }

    // Handle run command
    if let Some(Commands::Run { prompt }) = &cli.command {
        run_cli_one_shot(prompt, &cli.workspace, cli.mock, &settings).await?;
        return Ok(());
    }

    // Run TUI mode
    run_tui_mode(cli, &mut settings, event_logger).await
}

async fn run_headless_mode(cli: &Cli, settings: &mut Settings) -> Result<(), Box<dyn std::error::Error>> {
    let headless_settings = build_headless_settings(cli, settings);
    run_headless(cli, &headless_settings).await?;
    Ok(())
}

async fn run_tui_mode(cli: Cli, settings: &mut Settings, event_logger: Option<EventStreamLogger>) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(windows))]
    {
        tui_run::run_tui(cli.workspace, cli.mock, settings, cli.mock_setup, event_logger.as_ref()).await?;
    }
    #[cfg(windows)]
    {
        eprintln!("TUI mode not yet supported on Windows.");
        eprintln!("Use: runie run --prompt \"your prompt\"");
        std::process::exit(1);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_and_run().await
}
