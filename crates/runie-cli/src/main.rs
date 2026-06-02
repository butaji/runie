mod acp;
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

use crate::acp::run_acp_stdio;
use crate::event_stream::EventStreamLogger;
use crate::provider_factory::create_provider;
use runie_ai::providers::MockProvider;
use runie_ai::Provider;

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
    #[arg(short, long)]
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
    output_format: OutputFormat,

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

#[derive(Clone, Default, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Plain,
    Json,
    StreamingJson,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::StreamingJson => write!(f, "streaming-json"),
        }
    }
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

fn is_headless(cli: &Cli) -> bool {
    cli.single.is_some()
        || cli.model.is_some()
        || cli.session_id.is_some()
        || cli.resume.is_some()
        || cli.continue_
        || cli.cwd.is_some()
        || cli.output_format != OutputFormat::Plain
        || cli.always_approve
        || cli.no_alt_screen
        || cli.no_auto_update
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
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
        let headless_settings = build_headless_settings(&cli, &mut settings);
        run_headless(&cli, &headless_settings).await?;
        return Ok(());
    }

    match &cli.command {
        Some(Commands::Run { prompt }) => {
            run_cli_one_shot(prompt, &cli.workspace, cli.mock, &settings).await?;
        }
        Some(Commands::Agent { .. }) => {
            // ACP mode is handled earlier; this branch is unreachable
            unreachable!("ACP mode should be handled before this match");
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

fn build_headless_settings(cli: &Cli, settings: &mut Settings) -> CliSettings {
    // Apply CLI flags to settings
    if let Some(ref model) = cli.model {
        settings.model = model.clone();
    }
    if cli.single.is_some() || is_headless(&cli) {
        // Merge headless settings
    }
    CliSettings {
        model: cli.model.clone(),
        ..Default::default()
    }
}

/// Run in headless mode with output format support
async fn run_headless(cli: &Cli, _headless_settings: &CliSettings) -> Result<(), Box<dyn std::error::Error>> {
    let prompt = cli.single.clone().unwrap_or_default();
    let workspace = cli.cwd.clone().unwrap_or_else(|| cli.workspace.clone());
    let output_format = &cli.output_format;

    match output_format {
        OutputFormat::Plain => {
            run_headless_plain(&prompt, &workspace, cli.mock, _headless_settings).await?;
        }
        OutputFormat::Json => {
            run_headless_json(&prompt, &workspace, cli.mock, _headless_settings).await?;
        }
        OutputFormat::StreamingJson => {
            run_headless_streaming_json(&prompt, &workspace, cli.mock, _headless_settings).await?;
        }
    }

    Ok(())
}

async fn run_headless_plain(
    prompt: &str,
    _workspace: &PathBuf,
    mock: bool,
    _settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    if prompt.is_empty() {
        return Ok(());
    }

    println!("❯ {}", prompt);
    println!();

    let mut settings = Settings::load();

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
        let provider = match create_provider(mock, &settings) {
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

async fn run_headless_json(
    prompt: &str,
    workspace: &PathBuf,
    mock: bool,
    _settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(serde::Serialize)]
    struct JsonOutput {
        prompt: String,
        workspace: String,
        response: Option<String>,
        error: Option<String>,
    }

    if prompt.is_empty() {
        let output = JsonOutput {
            prompt: String::new(),
            workspace: workspace.to_string_lossy().to_string(),
            response: None,
            error: Some("No prompt provided".to_string()),
        };
        println!("{}", serde_json::to_string(&output)?);
        return Ok(());
    }

    let mut settings = Settings::load();
    let mut response_content: Option<String> = None;
    let mut error_content: Option<String> = None;

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => response_content = Some(response),
            Err(e) => error_content = Some(e.to_string()),
        }
    } else {
        let provider = match create_provider(mock, &settings) {
            Ok(p) => p,
            Err(e) => {
                let output = JsonOutput {
                    prompt: prompt.to_string(),
                    workspace: workspace.to_string_lossy().to_string(),
                    response: None,
                    error: Some(e.to_string()),
                };
                println!("{}", serde_json::to_string(&output)?);
                return Ok(());
            }
        };

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => response_content = Some(response),
            Err(e) => error_content = Some(e.to_string()),
        }
    }

    let output = JsonOutput {
        prompt: prompt.to_string(),
        workspace: workspace.to_string_lossy().to_string(),
        response: response_content,
        error: error_content,
    };

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

async fn run_headless_streaming_json(
    prompt: &str,
    workspace: &PathBuf,
    mock: bool,
    _settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(serde::Serialize)]
    struct StreamEvent {
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    }

    // Emit start event
    println!(
        "{}",
        serde_json::to_string(&StreamEvent {
            event: "start".to_string(),
            data: Some(serde_json::json!({
                "prompt": prompt,
                "workspace": workspace.to_string_lossy()
            })),
        })?
    );

    if prompt.is_empty() {
        println!(
            "{}",
            serde_json::to_string(&StreamEvent {
                event: "error".to_string(),
                data: Some(serde_json::json!({"message": "No prompt provided"})),
            })?
        );
        return Ok(());
    }

    let mut settings = Settings::load();

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "response".to_string(),
                        data: Some(serde_json::json!({"content": response})),
                    })?
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "error".to_string(),
                        data: Some(serde_json::json!({"message": e.to_string()})),
                    })?
                );
            }
        }
    } else {
        let provider = match create_provider(mock, &settings) {
            Ok(p) => p,
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "error".to_string(),
                        data: Some(serde_json::json!({"message": e.to_string()})),
                    })?
                );
                return Ok(());
            }
        };

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "response".to_string(),
                        data: Some(serde_json::json!({"content": response})),
                    })?
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "error".to_string(),
                        data: Some(serde_json::json!({"message": e.to_string()})),
                    })?
                );
            }
        }
    }

    // Emit end event
    println!(
        "{}",
        serde_json::to_string(&StreamEvent {
            event: "end".to_string(),
            data: None,
        })?
    );

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
