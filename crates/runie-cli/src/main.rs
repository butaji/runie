mod agent_spawn;
mod context_loader;
mod event_logger;
mod event_stream;
mod git;
mod logging;
mod provider_factory;
mod settings;
mod tui_run;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use settings::{CliSettings, Settings};

use crate::event_stream::EventStreamLogger;
use crate::provider_factory::create_provider;
use runie_ai::providers::MockProvider;
use runie_ai::Provider;

/// Tidy coding harness — AI agent toolkit for the terminal.
///
/// USAGE:
///   runie                  Start interactive TUI (default)
///   runie --mock           TUI with mock provider (no API key)
///   runie --mock-setup     Run onboarding wizard with mock provider
///   runie run "prompt"     CLI one-shot mode
#[derive(Parser)]
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
    #[arg(short, long)]
    session: Option<String>,

    /// Use mock provider for testing (no API key needed)
    #[arg(long)]
    mock: bool,

    /// Run onboarding wizard with mock provider
    #[arg(long)]
    mock_setup: bool,
}

impl From<&Cli> for CliSettings {
    fn from(_cli: &Cli) -> Self {
        Self::default()
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Run a single prompt without entering TUI (CLI)
    Run { prompt: String },
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    init_environment(&cli);
    let event_logger = init_event_logging();
    let mut settings = Settings::load();

    match cli.command {
        Some(Commands::Run { prompt }) => {
            run_cli_one_shot(&prompt, &cli.workspace, cli.mock, &settings).await?;
        }
        None => {
            #[cfg(not(windows))]
            {
                tui_run::run_tui(cli.workspace, cli.mock, &mut settings, cli.mock_setup, event_logger.as_ref()).await?;
            }
            #[cfg(windows)]
            {
                eprintln!("TUI mode not yet supported on Windows.");
                eprintln!("Use: runie run --prompt \"your prompt\"");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

/// CLI: One-shot execution without TUI.
async fn run_cli_one_shot(
    prompt: &str,
    _workspace: &PathBuf,
    mock: bool,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("❯ {}", prompt);
    println!();

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Mock error: {}", e),
        }
    } else {
        let provider = match create_provider(mock, settings) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        };
        println!("Model: {} ({})", settings.model, settings.provider);
        println!("Processing...");

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Ok(())
}
