mod agent_spawn;
mod context_loader;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Set RUNIE_HOME if --dev-folder is provided
    if let Some(dev) = &cli.dev_folder {
        std::env::set_var("RUNIE_HOME", dev.as_os_str());
    }

    // Ensure runie directories exist
    settings::ensure_dirs();

    // Create default config if none exists
    if let Some(path) = settings::config_path() {
        settings::create_default_config(&path);
    }

    // Initialize logging and event stream
    let event_logger = if let Some(runie_dir) = settings::runie_dir() {
        logging::init_logging(&runie_dir);
        Some(EventStreamLogger::new(&runie_dir))
    } else {
        None
    };

    // Load settings with layered resolution, then apply CLI overrides
    let mut settings = Settings::load();

    match cli.command {
        // CLI: One-shot mode
        Some(Commands::Run { prompt }) => {
            run_cli_one_shot(&prompt, &cli.workspace, cli.mock, &settings).await?;
        }
        // TUI: Interactive terminal interface (default)
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
