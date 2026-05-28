use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use runie_agent::events::{AgentEvent, PermissionDecision};
use runie_agent::loop_engine::{AgentLoopConfig, run_agent_loop};
use runie_agent::{SafetyHook, Hook};
use runie_ai::Provider;
use runie_tools::{create_default_toolkit, Workspace};
use crate::event_stream::EventStreamLogger;
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
    // API key set in settings file
    if settings.api_key.is_some() {
        return false;
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
    settings: &mut Settings,
    force_setup: bool,
    event_logger: Option<&EventStreamLogger>,
) -> Result<(), Box<dyn std::error::Error>> {
    use runie_tui::{Tui, TuiConfig, Msg, Cmd};

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
    let path = if mock {
        "src/components".to_string()
    } else {
        git_info.relative_path.clone()
    };
    tui.update(Msg::SetGitInfo {
        repo: git_info.repo,
        branch: git_info.branch,
        path,
    });
    if mock {
        // Show demo fallback content: 4 ✓ 4.5% ████░░░░░░
        tui.update(Msg::SetTopBarMockChecks {
            checks_passed: Some(4),
            checks_total: Some(4),
            percentage: Some(4.5),
            context_badges: Vec::new(),
        });
    } else {
        // Populate context badges from loaded context files
        let mut badges = Vec::new();
        if !loaded_paths.is_empty() {
            badges.push(format!("{} context", loaded_paths.len()));
        }
        tui.update(Msg::SetTopBarRealChecks {
            context_badges: badges,
        });
    }

    let input_right_info = if mock {
        format!("mock · {}", settings.model)
    } else {
        format!("{} · {}", settings.provider, settings.model)
    };
    tui.update(Msg::SetInputRightInfo(input_right_info));

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
        tui.update(Msg::EnterOnboarding);
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
        info!("Loaded context: {}", loaded_paths.join(", "));
    }

    // Log startup event
    if let Some(logger) = event_logger {
        logger.log_ui_event("startup");
    }

    tui.render()?;

    // Unified message channel for ALL event sources
    let (msg_tx, mut msg_rx) = mpsc::channel::<Msg>(100);

    // Cooperative cancellation token for graceful shutdown
    let cancel = CancellationToken::new();

    // Terminal reader - sends Msg directly (using spawn_blocking for cancellation support)
    let task_cancel = cancel.child_token();
    let msg_tx3 = msg_tx.clone();
    // Capture current mode to check before allowing Paste (blocks in Permission/Overlay)
    let current_mode = tui.state.mode.clone();
    tokio::task::spawn_blocking(move || {
        while !task_cancel.is_cancelled() {
            if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    // BUG-03 FIX: Check mode before emitting Paste — block in Permission/Overlay
                    let msgs = match event {
                        crossterm::event::Event::Resize(w, h) => vec![Msg::Resize(w, h)],
                        crossterm::event::Event::Paste(text) => {
                            if matches!(current_mode, runie_tui::TuiMode::Permission | runie_tui::TuiMode::Overlay) {
                                vec![]
                            } else {
                                vec![Msg::Paste(text)]
                            }
                        }
                        crossterm::event::Event::Key(key) => vec![Msg::TextareaKey(key)],
                        _ => vec![],
                    };
                    // Retry send up to 10 times with 1ms sleep to avoid dropping events
                    for msg in msgs {
                        let mut sent = false;
                        for _ in 0..10 {
                            if msg_tx3.try_send(msg.clone()).is_ok() {
                                sent = true;
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        if !sent {
                            // Channel full for >10ms — drop event but keep thread alive
                            break;
                        }
                    }
                }
            }
        }
    });

    // Agent task handle for the running agent loop
    let mut agent_task: Option<tokio::task::JoinHandle<()>> = None;

    // Shared permission state (replaces mpsc::channel<PermissionDecision>)
    let permission_state: Arc<tokio::sync::Mutex<Option<PermissionDecision>>> = Arc::new(tokio::sync::Mutex::new(None));

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

        // Update top bar with model and token info
        tui.state.top_bar.model = settings.model.clone();
        tui.state.top_bar.context_window = Some(context_window);
        tui.state.top_bar.estimated_tokens = Some(estimated_tokens);
    }

    // Process Cmds that need recursive handling (SlashCommand -> more Cmds)
    async fn process_cmd(
        cmd: Cmd,
        tui: &mut Tui,
        agent_task: &mut Option<tokio::task::JoinHandle<()>>,
        permission_state: &Arc<tokio::sync::Mutex<Option<PermissionDecision>>>,
        msg_tx: &mpsc::Sender<Msg>,
        workspace: &PathBuf,
        mock: bool,
        settings: &mut Settings,
        base_system_prompt: &str,
        _cancel: &CancellationToken,
    ) -> Vec<Cmd> {
        match cmd {
            Cmd::SpawnAgent { messages } => {
                if agent_task.is_none() {
                    let provider = match create_provider(mock, settings) {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Failed to create provider: {}", e);
                            return vec![];
                        }
                    };

                    let ws = Workspace::new(workspace.clone());
                    let registry = Arc::new(create_default_toolkit(ws));
                    let tools = create_agent_tools(registry.clone());
                    let safety_hook: Arc<dyn Hook> = Arc::new(SafetyHook);
                    let hooks: Vec<Arc<dyn Hook>> = vec![safety_hook];

                    let config = AgentLoopConfig {
                        system_prompt: base_system_prompt.to_string(),
                        model: settings.model.clone(),
                        thinking_level: if settings.enable_thinking { "high" } else { "low" }.to_string(),
                        max_turns: settings.max_turns,
                    };

                    let permission_state_clone = permission_state.clone();
                    let msg_tx_clone = msg_tx.clone();

                    let task = tokio::spawn(async move {
                        let provider_arc: Arc<dyn Provider> = provider.into();
                        let result = run_agent_loop(
                            messages,
                            config,
                            provider_arc,
                            tools,
                            msg_tx_clone,
                            permission_state_clone,
                            registry,
                            hooks,
                        ).await;

                        if let Err(e) = result {
                            tracing::error!("Agent loop error: {}", e);
                        }
                    });

                    *agent_task = Some(task);
                }
                vec![]
            }
            Cmd::SendPermission { decision } => {
                // Write permission decision to shared state for agent to poll
                let mut guard = permission_state.lock().await;
                *guard = Some(decision);
                vec![]
            }
            Cmd::SlashCommand(slash_cmd) => {
                // Recursively process SlashCommand via update
                tui.update(Msg::SlashCommand(slash_cmd))
            }
            Cmd::SaveSettings { provider, model, api_key } => {
                // Update local settings
                settings.provider = provider.clone();
                settings.model = model.clone();
                settings.api_key = Some(api_key.clone());

                // Update TUI state
                tui.state.current_model = Some(format!("{}/{}", provider, model));
                tui.state.top_bar.model = model.clone();

                let config_path = dirs::home_dir()
                    .map(|h| h.join(".runie").join("config.toml"))
                    .unwrap_or_else(|| PathBuf::from(".runie/config.toml"));

                if let Some(parent) = config_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                let config = format!(
                    "provider = \"{}\"\nmodel = \"{}\"\napi_key = \"{}\"\nmax_turns = {}\nenable_thinking = {}\nshell = \"{}\"\n",
                    provider, model, api_key, settings.max_turns, settings.enable_thinking, settings.shell
                );
                if let Err(e) = std::fs::write(&config_path, config) {
                    tracing::error!("[SaveSettings] Failed to write config to {}: {}", config_path.display(), e);
                    return vec![];
                }

                match provider.as_str() {
                    "openai" => std::env::set_var("OPENAI_API_KEY", &api_key),
                    "anthropic" => std::env::set_var("ANTHROPIC_API_KEY", &api_key),
                    "google" => std::env::set_var("GOOGLE_API_KEY", &api_key),
                    "cohere" => std::env::set_var("COHERE_API_KEY", &api_key),
                    "mistral" => std::env::set_var("MISTRAL_API_KEY", &api_key),
                    "deepseek" => std::env::set_var("DEEPSEEK_API_KEY", &api_key),
                    "groq" => std::env::set_var("GROQ_API_KEY", &api_key),
                    "openrouter" => std::env::set_var("OPENROUTER_API_KEY", &api_key),
                    "huggingface" => std::env::set_var("HUGGINGFACE_API_KEY", &api_key),
                    "xai" => std::env::set_var("XAI_API_KEY", &api_key),
                    "azure" => std::env::set_var("AZURE_API_KEY", &api_key),
                    "moonshot" => std::env::set_var("MOONSHOT_API_KEY", &api_key),
                    "perplexity" => std::env::set_var("PERPLEXITY_API_KEY", &api_key),
                    "ollama" => std::env::set_var("OLLAMA_API_KEY", &api_key),
                    "hyperbolic" => std::env::set_var("HYPERBOLIC_API_KEY", &api_key),
                    "together" => std::env::set_var("TOGETHER_API_KEY", &api_key),
                    "zai" => std::env::set_var("ZAI_API_KEY", &api_key),
                    "minimax" => std::env::set_var("MINIMAX_API_KEY", &api_key),
                    "mira" => std::env::set_var("MIRA_API_KEY", &api_key),
                    "galadriel" => std::env::set_var("GALADRIEL_API_KEY", &api_key),
                    "llamafile" => std::env::set_var("LLAMAFILE_API_KEY", &api_key),
                    _ => {}
                }

                vec![]
            }
            Cmd::FetchModels { provider_id, api_key } => {
                tracing::info!("[FetchModels] Starting fetch for provider: {}", provider_id);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    tracing::debug!("[FetchModels] Fetch task started");
                    let fetcher = runie_ai::model_fetcher::create_fetcher(&provider_id);
                    match fetcher.fetch_models(&api_key).await {
                        Ok(models) => {
                            tracing::info!("[FetchModels] Fetched {} models for {}", models.len(), provider_id);
                            let result = tx.send(Msg::ModelsFetched(models)).await;
                            if let Err(e) = result {
                                tracing::error!("[FetchModels] Failed to send ModelsFetched: {}", e);
                            } else {
                                tracing::debug!("[FetchModels] ModelsFetched sent successfully");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[FetchModels] Fetch failed for {}: {}", provider_id, e);
                            let _ = tx.send(Msg::ModelsFetchFailed(e.to_string())).await;
                        }
                    }
                });
                vec![]
            }
            Cmd::Rollback { tool_call_id } => {
                info!("[Rollback] Tool {} cancelled - workspace state preserved", tool_call_id);
                vec![]
            }
            Cmd::Interrupt => {
                if let Some(task) = agent_task.take() {
                    task.abort();
                }
                // Clear permission state
                let mut guard = permission_state.lock().await;
                *guard = None;
                vec![]
            }
        }
    }

    // TEA main loop
    while tui.state.running {
        tokio::select! {
            // Bias: check messages before ticks to prevent starvation
            biased;

            // Unified message channel — handles terminal events, agent events, model fetch
            Some(msg) = msg_rx.recv() => {
                // Convert raw key events to proper routed messages
                let msgs = match msg {
                    Msg::TextareaKey(key) => runie_tui::event_to_msg(crossterm::event::Event::Key(key), &tui.state),
                    other => vec![other],
                };

                for msg in msgs {
                // Log agent events to event stream
                if let Msg::AgentEvent(ref agent_event) = msg {
                    if let Some(logger) = event_logger {
                        logger.log_agent_event(agent_event);
                    }
                }

                // Handle AgentEnd to clear agent task
                if let Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
                    agent_task = None;
                    let mut guard = permission_state.lock().await;
                    *guard = None;
                }

                // Log model fetch events
                match &msg {
                    Msg::ModelsFetched(models) => {
                        tracing::info!("[MainLoop] Received ModelsFetched with {} models", models.len());
                    }
                    Msg::ModelsFetchFailed(err) => {
                        tracing::warn!("[MainLoop] Received ModelsFetchFailed: {}", err);
                    }
                    _ => {}
                }

                let cmds = tui.update(msg.clone());

                // Batch all pending messages before rendering
                let mut pending_cmds = cmds;
                while let Ok(msg) = msg_rx.try_recv() {
                    if let Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
                        agent_task = None;
                        let mut guard = permission_state.lock().await;
                        *guard = None;
                    }
                    // Convert raw key events in batched messages too
                    let msgs = match msg {
                        Msg::TextareaKey(key) => runie_tui::event_to_msg(crossterm::event::Event::Key(key), &tui.state),
                        other => vec![other],
                    };
                    for msg in msgs {
                        let more_cmds = tui.update(msg);
                        pending_cmds.extend(more_cmds);
                    }
                }

                // Execute commands (may produce more commands recursively)
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &permission_state, &msg_tx, &workspace, mock, &mut *settings, &base_system_prompt, &cancel).await);
                    }
                    pending_cmds = next_cmds;
                }

                // Update context info after agent events (skip in mock mode)
                if !mock {
                    update_top_bar_context(&mut tui, &settings);
                }

                // Log key UI events
                if let Some(logger) = event_logger {
                    match &msg {
                        Msg::Submit => logger.log_ui_event("submit"),
                        Msg::Quit => logger.log_ui_event("quit"),
                        Msg::ToggleSidebar => logger.log_ui_event("toggle_sidebar"),
                        Msg::ClearChat => logger.log_ui_event("clear_chat"),
                        Msg::AgentEvent(AgentEvent::PermissionRequest { .. }) => {
                            logger.log_ui_event("permission_request")
                        }
                        _ => {}
                    }
                }

                // Render after every message batch — state changed, screen must reflect it
                tui.render()?;
                } // end for msg in msgs
            }

            // Cursor blink (500ms)
            _ = cursor_interval.tick() => {
                let cmds = tui.update(Msg::CursorBlink);
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &permission_state, &msg_tx, &workspace, mock, &mut *settings, &base_system_prompt, &cancel).await);
                    }
                    pending_cmds = next_cmds;
                }
                tui.render()?;
            }

            // Animation tick (80ms)
            _ = tick_interval.tick() => {
                if !mock {
                    update_top_bar_context(&mut tui, &settings);
                }
                let cmds = tui.update(Msg::Tick);
                let mut pending_cmds = cmds;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        next_cmds.extend(process_cmd(cmd, &mut tui, &mut agent_task, &permission_state, &msg_tx, &workspace, mock, &mut *settings, &base_system_prompt, &cancel).await);
                    }
                    pending_cmds = next_cmds;
                }
                tui.render()?;
            }
        }
    }

    // Graceful shutdown: signal cancellation, wait up to 2s, then force if needed
    cancel.cancel();
    if let Some(task) = agent_task.take() {
        task.abort();
        let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
    }

    tui.cleanup()?;

    if let Some(logger) = event_logger {
        logger.log_ui_event("shutdown");
    }

    Ok(())
}
