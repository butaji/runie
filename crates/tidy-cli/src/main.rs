use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tidy_agent::events::{AgentEvent, PermissionDecision};
use tidy_agent::loop_engine::{run_agent_loop, AgentLoopConfig};
use tidy_agent::pi::AgentTool;
use tidy_ai::providers::MockProvider;

/// Tidy coding harness — AI agent toolkit for the terminal.
///
/// USAGE:
///   tidy                  Start interactive TUI (default)
///   tidy --mock           TUI with mock provider (no API key)
///   tidy --run "prompt"   CLI one-shot mode
///   tidy sessions         List sessions
///   tidy tree <id>        Show session tree
#[derive(Parser)]
#[command(name = "tidy")]
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

    match cli.command {
        // CLI: One-shot commands
        Some(Commands::Run { prompt }) => {
            run_cli_one_shot(&prompt, &cli.workspace, cli.mock).await?;
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
                run_tui(cli.workspace, cli.mock).await?;
            }
            #[cfg(windows)]
            {
                eprintln!("TUI mode not yet supported on Windows.");
                eprintln!("Use: tidy run --prompt \"your prompt\"");
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
) -> Result<(), Box<dyn std::error::Error>> {
    if mock {
        println!("❯ {}", prompt);
        println!();
        println!("{}", generate_mock_response(prompt));
    } else {
        println!("❯ {}", prompt);
        println!();
        println!("Processing... (real provider mode — requires OPENAI_API_KEY)");
        // TODO: wire up real provider
    }
    Ok(())
}

/// TUI: Interactive terminal interface using TEA architecture.
#[cfg(not(windows))]
async fn run_tui(workspace: PathBuf, mock: bool) -> Result<(), Box<dyn std::error::Error>> {
    use tidy_tui::{Tui, TuiConfig, Msg, Cmd, event_to_msg};

    let config = TuiConfig::default();
    let mut tui = Tui::new(config)?;

    tui.state.top_bar_repo = "tidy".to_string();
    tui.state.top_bar_branch = if mock { "mock/main".to_string() } else { "main".to_string() };
    tui.state.top_bar_path = workspace.display().to_string();
    tui.state.top_bar_checks_passed = Some(4);
    tui.state.top_bar_checks_total = Some(4);
    tui.state.top_bar_percentage = Some(4.56);

    tui.state.input_right_info = if mock {
        "mock-provider · no-api-key".to_string()
    } else {
        "tidy-main · always-approve".to_string()
    };

    // Welcome message
    tui.add_message(tidy_tui::components::message_list::MessageItem::System {
        text: if mock {
            "Mock mode active — no API key needed. Type a message to test!".to_string()
        } else {
            "Welcome to Tidy! Type a message to start.".to_string()
        },
    });

    tui.render()?;

    // Channel for raw terminal events
    let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<crossterm::event::Event>();

    // Terminal reader - sends raw events
    let raw_tx2 = raw_tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = crossterm::event::read() {
                if raw_tx2.send(event).is_err() {
                    break;
                }
            }
        }
    });

    // Agent event channel (from agent task to main loop)
    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel::<AgentEvent>();

    // Permission channel (for permission decisions)
    let (perm_tx, perm_rx) = mpsc::unbounded_channel::<PermissionDecision>();
    let perm_rx = Arc::new(tokio::sync::Mutex::new(Some(perm_rx)));
    let perm_tx = Arc::new(tokio::sync::Mutex::new(perm_tx));

    // Agent task handle
    let mut agent_task: Option<tokio::task::JoinHandle<()>> = None;

    // TEA main loop
    while tui.state.running {
        tokio::select! {
            // Raw terminal events
            Some(event) = raw_rx.recv() => {
                if let Some(msg) = event_to_msg(event, &tui.state) {
                    let cmds = tidy_tui::update(&mut tui.state, msg);

                    // Execute commands
                    for cmd in cmds {
                        match cmd {
                            Cmd::SpawnAgent { messages } => {
                                if agent_task.is_none() {
                                    let event_tx = agent_tx.clone();
                                    let permission_rx = perm_rx.clone();

                                    agent_task = Some(tokio::spawn(async move {
                                        let provider = MockProvider::new();
                                        let tools: Vec<AgentTool> = vec![];
                                        let rx = permission_rx.lock().await.take().unwrap();

                                        let config = AgentLoopConfig {
                                            system_prompt: "You are a helpful coding assistant.".to_string(),
                                            model: "gpt-4".to_string(),
                                            thinking_level: "low".to_string(),
                                        };

                                        match run_agent_loop(
                                            messages,
                                            config,
                                            &provider,
                                            &tools,
                                            event_tx,
                                            rx,
                                        ).await {
                                            Ok(_) => {},
                                            Err(e) => eprintln!("Agent error: {}", e),
                                        }
                                    }));
                                }
                            }
                            Cmd::SendPermission { decision } => {
                                perm_tx.lock().await.send(decision).ok();
                            }
                        }
                    }

                    // Render after EVERY message
                    tui.render()?;
                }
            }

            // Agent events
            Some(event) = agent_rx.recv() => {
                if let AgentEvent::AgentEnd { .. } = &event {
                    agent_task = None;
                }

                let cmds = tidy_tui::update(&mut tui.state, Msg::AgentEvent(event));

                // Execute commands
                for cmd in cmds {
                    match cmd {
                        Cmd::SpawnAgent { messages: _ } => {
                            // Agent already running, ignore
                        }
                        Cmd::SendPermission { decision } => {
                            perm_tx.lock().await.send(decision).ok();
                        }
                    }
                }

                // Render after EVERY message
                tui.render()?;
            }
        }
    }

    // Abort any running agent task before cleanup
    if let Some(task) = agent_task {
        task.abort();
    }

    tui.cleanup()?;
    Ok(())
}

fn generate_mock_response(input: &str) -> String {
    let input_lower = input.to_lowercase();

    if input_lower.contains("hello") || input_lower.contains("hi") {
        "Hello! I'm your mock coding assistant. I can help you with:\n\n• Reading and editing files\n• Running commands\n• Searching code\n• Analyzing projects\n\nWhat would you like to work on?".to_string()
    } else if input_lower.contains("edit") || input_lower.contains("fix") {
        "I'll help you edit that. First, let me read the file to understand its current state.\n\n[Tool: read_file]\nReading file contents...\n\nGot it. I can see the structure. What specific changes would you like to make?".to_string()
    } else if input_lower.contains("test") || input_lower.contains("run") {
        "Running tests...\n\n```\n$ cargo test\n   Compiling tidy-core v0.1.0\n   Compiling tidy-agent v0.1.0\n    Finished test [unoptimized + debuginfo]\n     Running unittests\n\ntest result: ok. 15 passed; 0 failed;\n```\n\nAll tests pass! ✅".to_string()
    } else if input_lower.contains("help") || input_lower.contains("?") {
        "Available commands:\n\n• File operations: read, write, edit files\n• Shell: run bash commands\n• Search: find files and content\n• Code: analyze, refactor, generate\n\nJust describe what you want to do!".to_string()
    } else {
        format!("Interesting! You said: \"{}\"\n\nIn mock mode, I simulate responses without calling real LLMs. Try asking me to:\n- Edit a file\n- Run tests\n- Search for something\n- Or just say hello!", input)
    }
}
