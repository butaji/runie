mod agent_spawn;
mod context_loader;
mod git;
mod provider_factory;
mod settings;
mod tui_run;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use settings::{CliSettings, Keybindings, Settings};

use crate::provider_factory::create_provider;

/// Tidy coding harness — AI agent toolkit for the terminal.
///
/// USAGE:
///   runie                  Start interactive TUI (default)
///   runie --mock           TUI with mock provider (no API key)
///   runie --run "prompt"   CLI one-shot mode
///   runie sessions         List sessions
///   runie tree <id>        Show session tree
#[derive(Parser)]
#[command(name = "runie")]
#[command(about = "Tidy coding harness — AI agent toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Workspace directory
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Session ID to resume
    #[arg(short, long)]
    session: Option<String>,

    /// Use mock provider for testing (no API key needed)
    #[arg(long)]
    mock: bool,

    /// Model to use (e.g., gpt-4o, claude-3-opus)
    #[arg(long)]
    model: Option<String>,

    /// Provider to use (e.g., openai, anthropic)
    #[arg(long)]
    provider: Option<String>,

    /// API key for the provider
    #[arg(long)]
    api_key: Option<String>,

    /// Custom base URL for the provider
    #[arg(long)]
    base_url: Option<String>,

    /// Maximum number of conversation turns
    #[arg(long)]
    max_turns: Option<usize>,

    /// Temperature for generation (0.0-2.0)
    #[arg(long)]
    temperature: Option<f32>,

    /// Theme name
    #[arg(long)]
    theme: Option<String>,

    /// Auto-save sessions
    #[arg(long, default_value_t = true)]
    auto_save: bool,

    /// Token threshold for compaction
    #[arg(long)]
    compact_threshold: Option<usize>,

    /// Tool mode: parallel or sequential
    #[arg(long)]
    tool_mode: Option<String>,

    /// Enable thinking blocks
    #[arg(long)]
    enable_thinking: Option<bool>,

    /// Default shell for bash tool
    #[arg(long)]
    shell: Option<String>,

    /// Keybinding: submit
    #[arg(long)]
    kb_submit: Option<String>,

    /// Keybinding: new line
    #[arg(long)]
    kb_new_line: Option<String>,

    /// Keybinding: exit
    #[arg(long)]
    kb_exit: Option<String>,

    /// Keybinding: sidebar toggle
    #[arg(long)]
    kb_sidebar: Option<String>,

    /// Keybinding: command palette
    #[arg(long)]
    kb_command_palette: Option<String>,

    /// Force onboarding/setup wizard (even if already configured)
    #[arg(long)]
    setup: bool,

    /// Test rendering pipeline without API
    #[arg(long)]
    test_render: bool,

    /// Test event handling pipeline
    #[arg(long)]
    test_events: bool,

    /// Test full TUI pipeline (render + input + events)
    #[arg(long)]
    test_pipeline: bool,
}

impl From<&Cli> for CliSettings {
    fn from(cli: &Cli) -> Self {
        let keybindings = if cli.kb_submit.is_some()
            || cli.kb_new_line.is_some()
            || cli.kb_exit.is_some()
            || cli.kb_sidebar.is_some()
            || cli.kb_command_palette.is_some()
        {
            Some(Keybindings {
                submit: cli.kb_submit.clone(),
                new_line: cli.kb_new_line.clone(),
                exit: cli.kb_exit.clone(),
                sidebar: cli.kb_sidebar.clone(),
                command_palette: cli.kb_command_palette.clone(),
            })
        } else {
            None
        };

        Self {
            model: cli.model.clone(),
            provider: cli.provider.clone(),
            api_key: cli.api_key.clone(),
            base_url: cli.base_url.clone(),
            max_turns: cli.max_turns,
            temperature: cli.temperature,
            theme: cli.theme.clone(),
            keybindings,
            auto_save: Some(cli.auto_save),
            compact_threshold: cli.compact_threshold,
            tool_mode: cli.tool_mode.clone(),
            enable_thinking: cli.enable_thinking,
            shell: cli.shell.clone(),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// List saved sessions (CLI)
    Sessions,
    /// Show session tree (CLI)
    Tree { session_id: String },
    /// Compact a session (CLI)
    Compact { session_id: String },
    /// Run a single prompt without entering TUI (CLI)
    Run { prompt: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Ensure runie directories exist
    settings::ensure_dirs();

    // Create default config if none exists
    if let Some(path) = settings::config_path() {
        settings::create_default_config(&path);
    }

    // Load settings with layered resolution, then apply CLI overrides
    let mut settings = Settings::load();
    settings.merge_cli(&CliSettings::from(&cli));

    // Part 4: Validate model after loading settings
    if !settings.validate_model() {
        eprintln!("Warning: model '{}' not found in model registry. Using default.", settings.model);
    }

    match cli.command {
        // CLI: One-shot commands
        Some(Commands::Run { prompt }) => {
            run_cli_one_shot(&prompt, &cli.workspace, cli.mock, &settings).await?;
        }
        Some(Commands::Sessions) => {
            println!("Saved sessions:");
            // TODO: implement session listing
        }
        Some(Commands::Tree { session_id }) => {
            println!("Session tree for: {}", session_id);
            // TODO: implement tree display
        }
        Some(Commands::Compact { session_id }) => {
            println!("Compacting session: {}", session_id);
            // TODO: implement compaction
        }
        // TUI: Interactive terminal interface (default)
        None => {
            #[cfg(not(windows))]
            {
                // Run test modes if requested
                if cli.test_pipeline {
                    return tui_run::test_pipeline().await;
                }
                if cli.test_render {
                    return tui_run::test_render().await;
                }
                if cli.test_events {
                    return tui_run::test_events().await;
                }
                tui_run::run_tui(cli.workspace, cli.mock, &settings, cli.setup).await?;
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
        println!("{}", generate_mock_response(prompt));
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

fn generate_mock_response(input: &str) -> String {
    let input_lower = input.to_lowercase();

    if input_lower.contains("hello") || input_lower.contains("hi") {
        "Hello! I'm your mock coding assistant. I can help you with:\n\n• Reading and editing files\n• Running commands\n• Searching code\n• Analyzing projects\n\nWhat would you like to work on?".to_string()
    } else if input_lower.contains("edit") || input_lower.contains("fix") {
        "I'll help you edit that. First, let me read the file to understand its current state.\n\n[Tool: read_file]\nReading file contents...\n\nGot it. I can see the structure. What specific changes would you like to make?".to_string()
    } else if input_lower.contains("test") || input_lower.contains("run") {
        "Running tests...\n\n```\n$ cargo test\n   Compiling runie-core v0.1.0\n   Compiling runie-agent v0.1.0\n    Finished test [unoptimized + debuginfo]\n     Running unittests\n\ntest result: ok. 15 passed; 0 failed;\n```\n\nAll tests pass! ✅".to_string()
    } else if input_lower.contains("help") || input_lower.contains("?") {
        "Available commands:\n\n• File operations: read, write, edit files\n• Shell: run bash commands\n• Search: find files and content\n• Code: analyze, refactor, generate\n\nJust describe what you want to do!".to_string()
    } else {
        format!("Interesting! You said: \"{}\"\n\nIn mock mode, I simulate responses without calling real LLMs. Try asking me to:\n- Edit a file\n- Run tests\n- Search for something\n- Or just say hello!", input)
    }
}
