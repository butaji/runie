use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use runie_agent::events::{AgentEvent, PermissionDecision};
use runie_agent::loop_engine::AgentLoopConfig;
use runie_agent::{SafetyHook, Hook};
use runie_tools::{create_default_toolkit, Workspace};
use crate::settings::Settings;
use crate::context_loader::ContextLoader;
use crate::provider_factory::create_provider;
use crate::agent_spawn::create_agent_tools;

pub async fn run_tui(
    workspace: PathBuf,
    mock: bool,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    use runie_tui::{Tui, TuiConfig, Msg, Cmd, event_to_msg};

    // Load AGENTS.md context files
    let context_files = ContextLoader::load();
    let loaded_paths = ContextLoader::loaded_paths();

    // Check for system prompt override
    let base_system_prompt = if let Some(override_prompt) = ContextLoader::system_override() {
        override_prompt
    } else {
        crate::context_loader::build_system_prompt(&context_files)
    };

    let config = TuiConfig::default();
    let mut tui = Tui::new(config)?;

    // Detect real git info (even in mock mode, for now)
    let git_info = crate::git::detect_git_info(&workspace);
    tui.state.top_bar_repo = git_info.repo;
    tui.state.top_bar_branch = git_info.branch;
    tui.state.top_bar_path = git_info.relative_path;
    tui.state.top_bar_checks_passed = None;
    tui.state.top_bar_checks_total = None;
    tui.state.top_bar_percentage = None;

    tui.state.input_right_info = if mock {
        format!("mock · {}", settings.model)
    } else {
        format!("{} · {}", settings.provider, settings.model)
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
            format!("Mock mode — no API calls. Model: {}{}", settings.model, context_info)
        } else {
            format!("Connected to {} ({}){}", settings.provider, settings.model, context_info)
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
        mock: bool,
        settings: &Settings,
        base_system_prompt: &str,
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

                    let mock_flag = mock;
                    let settings_clone = settings.clone();
                    let system_prompt = base_system_prompt.to_string();

                    *agent_task = Some(tokio::spawn(async move {
                        let provider = match create_provider(mock_flag, &settings_clone) {
                            Ok(p) => p,
                            Err(e) => {
                                event_tx.send(AgentEvent::Error { message: e }).ok();
                                return;
                            }
                        };

                        let config = AgentLoopConfig {
                            system_prompt,
                            model: settings_clone.model.clone(),
                            thinking_level: if settings_clone.enable_thinking { "high" } else { "low" }.to_string(),
                            max_turns: settings_clone.max_turns,
                        };

                        match runie_agent::loop_engine::run_agent_loop(
                            messages,
                            config,
                            &*provider,
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
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt));
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
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt));
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
                            next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt));
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
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt));
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