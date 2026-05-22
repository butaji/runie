mod context_loader;
mod git;
mod settings;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use runie_agent::events::{AgentEvent, PermissionDecision};
use runie_agent::loop_engine::{run_agent_loop, AgentLoopConfig};
use runie_agent::pi::AgentTool;
use runie_agent::{SafetyHook, Hook};
use runie_ai::providers::MockProvider;
use runie_tools::{create_default_toolkit, Workspace};
use settings::{CliSettings, Keybindings, Settings};

use crate::context_loader::{build_system_prompt, ContextLoader};

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
                run_tui(cli.workspace, cli.mock, &settings).await?;
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
    if mock {
        println!("❯ {}", prompt);
        println!();
        println!("{}", generate_mock_response(prompt));
    } else {
        println!("❯ {}", prompt);
        println!();
        println!("Model: {} ({})", settings.model, settings.provider);
        println!("Processing... (real provider mode)");
        // TODO: wire up real provider
    }
    Ok(())
}

/// TUI: Interactive terminal interface using TEA architecture.
#[cfg(not(windows))]
async fn run_tui(workspace: PathBuf, mock: bool, settings: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    use runie_tui::{Tui, TuiConfig, Msg, Cmd, event_to_msg};

    // Load AGENTS.md context files
    let context_files = ContextLoader::load();
    let loaded_paths = ContextLoader::loaded_paths();

    // Check for system prompt override
    let base_system_prompt = if let Some(override_prompt) = ContextLoader::system_override() {
        override_prompt
    } else {
        build_system_prompt(&context_files)
    };

    let config = TuiConfig::default();
    let mut tui = Tui::new(config)?;

    // Detect real git info (even in mock mode, for now)
    let git_info = git::detect_git_info(&workspace);
    tui.state.top_bar_repo = git_info.repo;
    tui.state.top_bar_branch = git_info.branch;
    tui.state.top_bar_path = git_info.relative_path;
    tui.state.top_bar_checks_passed = None;
    tui.state.top_bar_checks_total = None;
    tui.state.top_bar_percentage = None;

    tui.state.input_right_info = if mock {
        "mock-provider · no-api-key".to_string()
    } else {
        "runie-main · always-approve".to_string()
    };

    // Build startup message with context info
    let context_info = if loaded_paths.is_empty() {
        String::new()
    } else {
        format!(" · {} context file(s) loaded", loaded_paths.len())
    };

    // Welcome message
    tui.add_message(runie_tui::components::message_list::MessageItem::System {
        text: if mock {
            format!("Mock mode active — no API key needed. Type a message to test!{}", context_info)
        } else {
            format!("Welcome to Tidy! Type a message to start.{}", context_info)
        },
    });

    // Log loaded context files for debugging
    if !loaded_paths.is_empty() {
        eprintln!("Loaded context: {}", loaded_paths.join(", "));
    }

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

    // Agent task handle
    let mut agent_task: Option<tokio::task::JoinHandle<()>> = None;
    
    // Permission sender (replaced on each agent spawn)
    let mut perm_tx: Option<mpsc::UnboundedSender<PermissionDecision>> = None;

    // Animation timers
    let mut tick_interval = interval(Duration::from_millis(80));
    let mut cursor_interval = interval(Duration::from_millis(500));

    // Process Cmds that need recursive handling (SlashCommand -> more Cmds)
    fn process_cmd(
        cmd: Cmd,
        tui: &mut Tui,
        agent_task: &mut Option<tokio::task::JoinHandle<()>>,
        perm_tx: &mut Option<mpsc::UnboundedSender<PermissionDecision>>,
        agent_tx: &mpsc::UnboundedSender<AgentEvent>,
        workspace: &PathBuf,
    ) -> Vec<Cmd> {
        match cmd {
            Cmd::SpawnAgent { messages } => {
                if agent_task.is_none() {
                    let event_tx = agent_tx.clone();
                    
                    // Create fresh permission channel for this agent
                    let (fresh_perm_tx, fresh_perm_rx) = mpsc::unbounded_channel::<PermissionDecision>();
                    *perm_tx = Some(fresh_perm_tx);

                    // Create workspace and tool registry
                    let ws = Workspace::new(workspace.clone());
                    let registry = Arc::new(create_default_toolkit(ws));
                    
                    // Convert registry tools to AgentTool format with async handlers
                    let tools = create_agent_tools(registry.clone());
                    
                    // Create safety hook
                    let safety_hook: Arc<dyn Hook> = Arc::new(SafetyHook);
                    let hooks: Vec<Arc<dyn Hook>> = vec![safety_hook];

                    *agent_task = Some(tokio::spawn(async move {
                        let provider = MockProvider::new();

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
                            fresh_perm_rx,
                            Some(registry),
                            hooks,
                        ).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Agent error: {}", e),
                        }
                    }));
                }
                vec![]
            }
            Cmd::SendPermission { decision } => {
                if let Some(ref tx) = *perm_tx {
                    tx.send(decision).ok();
                }
                vec![]
            }
            Cmd::SlashCommand(slash_cmd) => {
                // Recursively process SlashCommand via update
                runie_tui::update(&mut tui.state, Msg::SlashCommand(slash_cmd))
            }
            Cmd::SaveSession { name } => {
                // TODO: Implement session saving via SessionManager
                eprintln!("SaveSession not yet implemented: {:?}", name);
                vec![]
            }
            Cmd::LoadSession { name } => {
                // TODO: Implement session loading via SessionManager
                eprintln!("LoadSession not yet implemented: {}", name);
                vec![]
            }
        }
    }

    // TEA main loop
    while tui.state.running {
        tokio::select! {
            // Animation tick (80ms)
            _ = tick_interval.tick() => {
                let cmds = runie_tui::update(&mut tui.state, Msg::Tick);
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace));
                    }
                    pending_cmds = next_cmds;
                }
                tui.render()?;
            }

            // Cursor blink (500ms)
            _ = cursor_interval.tick() => {
                let cmds = runie_tui::update(&mut tui.state, Msg::CursorBlink);
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace));
                    }
                    pending_cmds = next_cmds;
                }
                tui.render()?;
            }

            // Raw terminal events
            Some(event) = raw_rx.recv() => {
                if let Some(msg) = event_to_msg(event, &tui.state) {
                    let cmds = runie_tui::update(&mut tui.state, msg);

                    // Execute commands (may produce more commands recursively)
                    let mut pending_cmds = cmds;
                    while !pending_cmds.is_empty() {
                        let mut next_cmds = vec![];
                        for cmd in pending_cmds {
                            next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace));
                        }
                        pending_cmds = next_cmds;
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

                let cmds = runie_tui::update(&mut tui.state, Msg::AgentEvent(event));

                // Execute commands (may produce more commands recursively)
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace));
                    }
                    pending_cmds = next_cmds;
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

/// Convert ToolRegistry tools to AgentTool format with async handlers.
fn create_agent_tools(registry: Arc<runie_tools::ToolRegistry>) -> Vec<AgentTool> {
    let handle = tokio::runtime::Handle::current();
    
    registry.list().into_iter().map(|tool| {
        let name = tool.name().to_string();
        let description = tool.description().to_string();
        let parameters = tool.schema().parameters;
        let registry_clone = registry.clone();
        let handle_clone = handle.clone();
        
        AgentTool::new(name.clone(), description, parameters).with_handler(
            Arc::new(move |args| {
                let registry = registry_clone.clone();
                let handle = handle_clone.clone();
                let name = name.clone();
                handle.block_on(async move {
                    match registry.get(&name) {
                        Some(t) => t.execute(args).await
                            .map(|o| o.content)
                            .map_err(|e| e.to_string()),
                        None => Err(format!("Tool not found: {}", name)),
                    }
                })
            }),
        )
    }).collect()
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
