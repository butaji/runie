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

/// Check if user needs onboarding (no provider, model, or API key configured)
fn needs_onboarding(settings: &Settings) -> bool {
    // No provider configured
    if settings.provider.is_empty() {
        return true;
    }
    // No model configured
    if settings.model.is_empty() {
        return true;
    }
    // No API key in environment
    if std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("ANTHROPIC_API_KEY").is_err()
        && std::env::var("GOOGLE_API_KEY").is_err()
        && std::env::var("RUNIE_API_KEY").is_err()
    {
        return true;
    }
    false
}

pub async fn run_tui(
    workspace: PathBuf,
    mock: bool,
    settings: &Settings,
    force_setup: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    use runie_tui::{Tui, TuiConfig, TuiMode, Onboarding, Msg, Cmd, event_to_msg};

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
    tui.state.top_bar.repo = git_info.repo;
    tui.state.top_bar.branch = git_info.branch;
    tui.state.top_bar.path = if mock {
        "src/components".to_string()
    } else {
        git_info.relative_path
    };
    if mock {
        // Show demo fallback content: 4 ✓ 4.5% ████░░░░░░
        tui.state.top_bar.checks_passed = Some(4);
        tui.state.top_bar.checks_total = Some(4);
        tui.state.top_bar.percentage = Some(4.5);
        tui.state.top_bar.context_badges = Vec::new();
        tui.state.top_bar.context_pct = None;
        tui.state.top_bar.context_bar_pct = None;
    } else {
        tui.state.top_bar.checks_passed = None;
        tui.state.top_bar.checks_total = None;
        tui.state.top_bar.percentage = None;

        // Populate context badges from loaded context files
        let mut badges = Vec::new();
        if !loaded_paths.is_empty() {
            badges.push(format!("{} context", loaded_paths.len()));
        }
        tui.state.top_bar.context_badges = badges;
    }

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

    // Check if onboarding is needed
    // --mock skips onboarding unless --mock-setup is explicitly used
    let needs_setup = force_setup || (!mock && needs_onboarding(settings));
    if needs_setup {
        tui.state.mode = TuiMode::Onboarding;
        tui.state.onboarding = Some(Onboarding::new());
        tui.update(Msg::AgentEvent(AgentEvent::Message {
            role: "system".to_string(),
            content: "Welcome! Let's set up your AI assistant.".to_string(),
        }));
    } else {
        // Normal welcome message
        tui.update(Msg::AgentEvent(AgentEvent::Message {
            role: "system".to_string(),
            content: if mock {
                format!("Mock mode — no API calls. Model: {}{}", settings.model, context_info)
            } else {
                format!("Using {} ({}){}", settings.provider, settings.model, context_info)
            },
        }));
    }

    // Log loaded context files for debugging
    if !loaded_paths.is_empty() {
        eprintln!("Loaded context: {}", loaded_paths.join(", "));
    }

    tui.render()?;

    // Channel for raw terminal events
    let (raw_tx, mut raw_rx) = mpsc::channel::<crossterm::event::Event>(100);

    // Terminal reader - sends raw events (blocking thread, uses try_send with retry)
    let raw_tx2 = raw_tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = crossterm::event::read() {
                // Retry send up to 10 times with 1ms sleep to avoid dropping events
                let mut sent = false;
                for _ in 0..10 {
                    if raw_tx2.try_send(event.clone()).is_ok() {
                        sent = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                if !sent {
                    // Channel full for >10ms — drop event but keep thread alive
                    continue;
                }
            }
        }
    });

    // Agent event channel (from agent task to main loop)
    let (agent_tx, mut agent_rx) = mpsc::channel::<AgentEvent>(100);

    // Agent task handle
    let mut agent_task: Option<tokio::task::JoinHandle<()>> = None;

    // Permission sender (replaced on each agent spawn)
    let mut perm_tx: Option<mpsc::Sender<PermissionDecision>> = None;

    // Animation timers
    let mut tick_interval = interval(Duration::from_millis(80));
    let mut cursor_interval = interval(Duration::from_millis(500));

    // Helper to update top bar context percentages from current state
    fn update_top_bar_context(tui: &mut Tui, settings: &Settings) {
        use runie_ai::ModelRegistry;

        // Calculate estimated tokens from message history (rough: ~4 chars/token)
        let total_chars: usize = tui.state.messages.iter().map(|msg| match msg {
            runie_tui::MessageItem::User { text, .. } => text.len(),
            runie_tui::MessageItem::Assistant { text, .. } => text.len(),
            runie_tui::MessageItem::System { text } => text.len(),
            runie_tui::MessageItem::ToolCall { name, args, result, .. } => {
                name.len() + args.len() + result.as_ref().map(|s| s.len()).unwrap_or(0)
            }
            _ => 0,
        }).sum();

        let estimated_tokens = total_chars / 4;

        // Look up context window for current model
        let registry = ModelRegistry::new();
        let context_window = registry.get(&settings.model)
            .map(|m| m.context_window)
            .unwrap_or(128_000); // default fallback

        let pct = if context_window > 0 {
            ((estimated_tokens as f32 / context_window as f32) * 100.0).min(100.0)
        } else {
            0.0
        };

        tui.state.top_bar.context_pct = Some(pct);
        tui.state.top_bar.context_bar_pct = Some(pct);

        // Ensure badges always show something meaningful
        if tui.state.top_bar.context_badges.is_empty() {
            tui.state.top_bar.context_badges = vec![settings.model.clone()];
        }
    }

    // Process Cmds that need recursive handling (SlashCommand -> more Cmds)
    async fn process_cmd(
        cmd: Cmd,
        tui: &mut Tui,
        agent_task: &mut Option<tokio::task::JoinHandle<()>>,
        perm_tx: &mut Option<mpsc::Sender<PermissionDecision>>,
        agent_tx: &mpsc::Sender<AgentEvent>,
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
                    let (fresh_perm_tx, fresh_perm_rx) = mpsc::channel::<PermissionDecision>(100);
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
                                let _ = event_tx.send(AgentEvent::Error { message: e }).await;
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
                            registry,
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
                    let _ = tx.send(decision).await;
                }
                vec![]
            }
            Cmd::SlashCommand(slash_cmd) => {
                // Recursively process SlashCommand via update
                tui.update(Msg::SlashCommand(slash_cmd))
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
            Cmd::SaveSettings { provider, model, api_key } => {
                // Save to ~/.runie/config.toml
                let config_path = dirs::home_dir()
                    .map(|h| h.join(".runie").join("config.toml"))
                    .unwrap_or_else(|| PathBuf::from(".runie/config.toml"));

                // Create .runie directory if needed
                if let Some(parent) = config_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                // Write config
                let config = format!(
                    "provider = \"{}\"\nmodel = \"{}\"\napi_key = \"{}\"\n",
                    provider, model, api_key
                );
                let _ = std::fs::write(&config_path, config);

                // Set API key env var for current session
                match provider.as_str() {
                    "openai" => std::env::set_var("OPENAI_API_KEY", &api_key),
                    "anthropic" => std::env::set_var("ANTHROPIC_API_KEY", &api_key),
                    "google" => std::env::set_var("GOOGLE_API_KEY", &api_key),
                    _ => {}
                }

                vec![]
            }
        }
    }

    // TEA main loop
    while tui.state.running {
        tokio::select! {
            // Bias: check keyboard and agent events before ticks
            // This prevents tick starvation — keyboard gets priority
            biased;

            // Raw terminal events — HIGHEST PRIORITY
            Some(event) = raw_rx.recv() => {
                if let Some(msg) = event_to_msg(event, &tui.state) {
                    let cmds = tui.update(msg);

                    // Execute commands (may produce more commands recursively)
                    let mut pending_cmds = cmds;
                    while !pending_cmds.is_empty() {
                        let mut next_cmds = vec![];
                        for cmd in pending_cmds {
                            next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt).await);
                        }
                        pending_cmds = next_cmds;
                    }

                    // Render after EVERY message
                    tui.render()?;
                }
            }

            // Agent events — SECOND PRIORITY
            Some(event) = agent_rx.recv() => {
                if let AgentEvent::AgentEnd { .. } = &event {
                    agent_task = None;
                }

                let cmds = tui.update(Msg::AgentEvent(event));

                // Execute commands (may produce more commands recursively)
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt).await);
                    }
                    pending_cmds = next_cmds;
                }

                // Update context info after agent events (skip in mock mode)
                if !mock {
                    update_top_bar_context(&mut tui, &settings);
                }

                // Render after EVERY message
                tui.render()?;
            }

            // Cursor blink (500ms) — THIRD PRIORITY
            _ = cursor_interval.tick() => {
                let cmds = tui.update(Msg::CursorBlink);
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt).await);
                    }
                    pending_cmds = next_cmds;
                }
                tui.render()?;
            }

            // Animation tick (80ms) — LOWEST PRIORITY
            _ = tick_interval.tick() => {
                if !mock {
                    update_top_bar_context(&mut tui, &settings);
                }
                let cmds = tui.update(Msg::Tick);
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &mut perm_tx, &agent_tx, &workspace, mock, &settings, &base_system_prompt).await);
                    }
                    pending_cmds = next_cmds;
                }
                tui.render()?;
            }
        }
    }

    // Abort any running agent task before cleanup
    if let Some(task) = agent_task.take() {
        task.abort();
        let _ = task.await;
    }

    tui.cleanup()?;
    Ok(())
}