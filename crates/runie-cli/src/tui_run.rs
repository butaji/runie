use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::ctrl_c;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use runie_agent::events::{AgentEvent, PermissionDecision};
use runie_agent::loop_engine::{AgentLoopConfig, run_agent_loop, AgentLoopError};
use runie_agent::{SafetyHook, Hook};
use runie_ai::Provider;
use runie_tools::{create_default_toolkit, Workspace};
use runie_tui::actors::{spawn_actor, input::InputActor as TuiInputActor, timer::TimerActor};
use runie_tui::pipe::{InputMsg, StateChange};
use crate::event_stream::EventStreamLogger;
use crate::event_logger as log;
use crate::settings::Settings;
use crate::context_loader::ContextLoader;
use crate::provider_factory::create_provider;
use crate::agent_spawn::create_agent_tools;

// Watchdog timeouts for agent stuck detection
const AGENT_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);
const AGENT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);


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
        && std::env::var("MINIMAX_API_KEY").is_err()
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

    // Load AGENTS.md context files (skip in mock mode)
    let (context_files, loaded_paths, git_info) = if mock {
        (Vec::new(), Vec::new(), crate::git::GitInfo::default())
    } else {
        let context_files = ContextLoader::load();
        let loaded_paths = ContextLoader::loaded_paths();
        let git_info = crate::git::detect_git_info(&workspace);
        (context_files, loaded_paths, git_info)
    };

    // Check for system prompt override
    let base_system_prompt = if let Some(override_prompt) = ContextLoader::system_override() {
        override_prompt
    } else {
        crate::context_loader::build_system_prompt(&context_files)
    };

    let config = TuiConfig::default();
    let mut tui = Tui::new(config)?;

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

    // Override settings to mock provider when --mock flag is used
    if mock {
        settings.provider = "mock".to_string();
        settings.model = "mock-gpt-4".to_string();
        settings.api_key = Some("mock-key".to_string());
    }

    let input_right_info = if mock {
        "mock".to_string()
    } else {
        format!("{} · {}", settings.provider, settings.model)
    };
    tui.update(Msg::SetInputRightInfo(input_right_info));

    // P0-MODEL-INIT: Initialize current_model from settings on startup
    if !settings.provider.is_empty() && !settings.model.is_empty() {
        tui.update(Msg::SetCurrentModel(Some(format!("{}/{}", settings.provider, settings.model))));
        tui.update(Msg::UpdateTopBarContext {
            model: settings.model.clone(),
            context_window: Some(128_000),
            estimated_tokens: Some(0),
        });
    }

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
        tui.update(Msg::SetMockMode(mock));
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

    // Spawn new actors using the actor framework
    let (input_handle, mut input_rx) = spawn_actor(TuiInputActor::new());
    let (timer_handle, mut timer_rx) = spawn_actor(TimerActor::new(80)); // 80ms animation tick

    // InputActor - reads terminal events and sends them as InputMsg
    // Note: InputActor runs via spawn_actor(), not manually spawned

    // Agent task handle for the running agent loop
    let mut agent_task: Option<tokio::task::JoinHandle<()>> = None;

    // Track last agent event time for watchdog
    let mut last_agent_event = Instant::now();

    // Shared permission state (replaces mpsc::channel<PermissionDecision>)
    let permission_state: Arc<tokio::sync::Mutex<Option<PermissionDecision>>> = Arc::new(tokio::sync::Mutex::new(None));

    // Animation timers - cursor blink only (animation tick is now via TimerActor)
    let mut cursor_interval = interval(Duration::from_millis(500));

    // Helper to update top bar context percentages from current state (Critical #4)
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

        // Update top bar via Msg (Critical #4)
        tui.update(Msg::UpdateTopBarContext {
            model: settings.model.clone(),
            context_window: Some(context_window),
            estimated_tokens: Some(estimated_tokens),
        });
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
    ) -> StateChange {
        match cmd {
            Cmd::SpawnAgent { messages } => {
                if agent_task.is_none() {
                    log::log_agent_spawned();
                    let provider = match create_provider(mock, settings) {
                        Ok(p) => {
                            log::log_provider_created(&settings.provider, &settings.model, true);
                            p
                        }
                        Err(e) => {
                            error!("Failed to create provider: {}", e);
                            log::log_provider_created(&settings.provider, &settings.model, false);
                            log::log_agent_error(&format!("Provider creation failed: {}", e));
                            return StateChange::none();
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
                        // 10-minute timeout safeguard — prevents agent running forever
                        let result = tokio::time::timeout(
                            Duration::from_secs(600),
                            run_agent_loop(
                                messages,
                                config,
                                provider_arc,
                                tools,
                                msg_tx_clone.clone(),
                                permission_state_clone,
                                registry,
                                hooks,
                            )
                        ).await;

                        match result {
                            Ok(Ok(_messages)) => {
                                // Agent completed normally — AgentEvent::AgentEnd will be sent by the loop
                                log::log_agent_completed();
                            }
                            Ok(Err(e)) => {
                                tracing::error!("Agent loop error: {}", e);
                                log::log_agent_error(&e.to_string());
                                // Classify error to get error_type, recoverable, context
                                let (error_type, recoverable, context) = match &e {
                                    AgentLoopError::ProviderError(msg) => (
                                        "provider".to_string(),
                                        true,
                                        format!("Provider error: {}", msg),
                                    ),
                                    AgentLoopError::ToolError(msg) => (
                                        "tool".to_string(),
                                        true,
                                        format!("Tool error: {}", msg),
                                    ),
                                    AgentLoopError::SendError(msg) => (
                                        "send".to_string(),
                                        true,
                                        format!("Send error: {}", msg),
                                    ),
                                    AgentLoopError::MaxTurnsExceeded => (
                                        "max_turns".to_string(),
                                        false,
                                        "Max turns exceeded".to_string(),
                                    ),
                                };
                                let _ = msg_tx_clone.send(Msg::AgentEvent(AgentEvent::Error {
                                    message: e.to_string(),
                                    error_type,
                                    recoverable,
                                    context,
                                })).await;
                            }
                            Err(_) => {
                                // Timeout
                                tracing::error!("Agent loop timed out after 10 minutes");
                                log::log_agent_error("Agent loop timed out after 10 minutes");
                                let _ = msg_tx_clone.send(Msg::AgentEvent(AgentEvent::Error {
                                    message: "Agent loop timed out after 10 minutes".to_string(),
                                    error_type: "timeout".to_string(),
                                    recoverable: false,
                                    context: "Agent loop exceeded 10 minute timeout".to_string(),
                                })).await;
                            }
                        }
                    });

                    *agent_task = Some(task);
                }
                StateChange::none()
            }
            Cmd::SendPermission { decision } => {
                // Write permission decision to shared state for agent to poll
                let mut guard = permission_state.lock().await;
                *guard = Some(decision);
                StateChange::none()
            }
            Cmd::SlashCommand(slash_cmd) => {
                // Recursively process SlashCommand via state pipe
                tui.update(Msg::SlashCommand(slash_cmd))
            }
            Cmd::SaveSettings { provider, model, api_key } => {
                // Update local settings
                settings.provider = provider.clone();
                settings.model = model.clone();
                settings.api_key = Some(api_key.clone());

                // Update TUI state via Msg (Critical #3)
                tui.update(Msg::SetCurrentModel(Some(format!("{}/{}", provider, model))));
                tui.update(Msg::UpdateTopBarContext {
                    model: model.clone(),
                    context_window: None,
                    estimated_tokens: None,
                });
                tui.update(Msg::SetInputRightInfo(format!("{} · {}", provider, model)));

                // P0-CONFIG-PATH: Use crate::settings::config_path() which respects RUNIE_HOME
                let config_path = crate::settings::config_path()
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
                    return StateChange::none();
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

                StateChange::none()
            }
            Cmd::FetchModels { provider_id, api_key } => {
                log::log(log::Subsystem::PROVIDER, log::LogLevel::INFO, &format!("[ACTOR:ModelPicker] Starting fetch for provider: {}", provider_id));
                tracing::info!("[ACTOR:ModelPicker] Starting fetch for provider: {}", provider_id);
                let tx = msg_tx.clone();
                tokio::spawn(async move {
                    tracing::debug!("[ACTOR:ModelPicker] Fetch task started");
                    let fetcher = runie_ai::model_fetcher::create_fetcher(&provider_id);
                    match fetcher.fetch_models(&api_key).await {
                        Ok(models) => {
                            tracing::info!("[ACTOR:ModelPicker] Fetched {} models for {}", models.len(), provider_id);
                            let result = tx.send(Msg::ModelsFetched(models)).await;
                            if let Err(e) = result {
                                tracing::error!("[ACTOR:ModelPicker] Failed to send ModelsFetched: {}", e);
                            } else {
                                tracing::debug!("[ACTOR:ModelPicker] ModelsFetched sent successfully");
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[ACTOR:ModelPicker] Fetch failed for {}: {}", provider_id, e);
                            let _ = tx.send(Msg::ModelsFetchFailed(e.to_string())).await;
                        }
                    }
                });
                StateChange::none()
            }
            Cmd::Rollback { tool_call_id } => {
                info!("[Rollback] Tool {} cancelled - workspace state preserved", tool_call_id);
                StateChange::none()
            }
            Cmd::Interrupt => {
                // Send error event before aborting so UI gets notified
                let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
                    message: "Agent interrupted by user".to_string(),
                    error_type: "interrupted".to_string(),
                    recoverable: true,
                    context: "User pressed Ctrl+C or sent interrupt signal".to_string(),
                })).await;
                if let Some(task) = agent_task.take() {
                    task.abort();
                }
                // Clear permission state
                let mut guard = permission_state.lock().await;
                *guard = None;
                StateChange::none()
            }
        }
    }

    // TEA main loop
    let mut last_render = Instant::now();
    const MIN_RENDER_INTERVAL_MS: u64 = 33; // ~30 FPS max render rate

    while tui.state.running {
        tokio::select! {
            // Bias: check messages before ticks to prevent starvation
            biased;

            // Input events from InputActor (via new actor framework)
            Some(input_msg) = input_rx.recv() => {
                // Convert InputMsg to Msg
                let msg = match input_msg {
                    InputMsg::Key(key) => Msg::TextareaKey(key),
                    InputMsg::Paste(text) => Msg::Paste(text),
                    InputMsg::Resize(w, h) => Msg::Resize(w, h),
                };

                // Convert raw key events to proper routed messages
                let msgs = match msg {
                    Msg::TextareaKey(key) => runie_tui::event_to_msg(crossterm::event::Event::Key(key), &tui.state),
                    other => vec![other],
                };

                // Track if state changed — determines whether to force render
                let mut state_changed = false;

                for msg in msgs {
                // [RAILS] Route message to update()
                tracing::debug!("[RAILS] Routing {:?} to update()", msg);

                // Log agent events to event stream
                if let Msg::AgentEvent(ref agent_event) = msg {
                    if let Some(logger) = event_logger {
                        logger.log_agent_event(agent_event);
                    }
                    // Reset heartbeat tracker on any agent event
                    last_agent_event = Instant::now();
                }

                // Handle AgentEnd to clear agent task
                if let Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
                    agent_task = None;
                    let mut guard = permission_state.lock().await;
                    *guard = None;
                    state_changed = true;
                }

                // Handle Error to ensure state_changed is set for renders
                if let Msg::AgentEvent(AgentEvent::Error { .. }) = &msg {
                    state_changed = true;
                }

                // Log model fetch events
                match &msg {
                    Msg::ModelsFetched(models) => {
                        tracing::info!("[RAILS] Received ModelsFetched with {} models", models.len());
                    }
                    Msg::ModelsFetchFailed(err) => {
                        tracing::warn!("[RAILS] Received ModelsFetchFailed: {}", err);
                    }
                    _ => {}
                }

                // Mark state as changed for meaningful events (triggers immediate render)
                match &msg {
                    // Agent events that affect UI
                    Msg::AgentEvent(AgentEvent::MessageUpdate { .. }) => state_changed = true,
                    Msg::AgentEvent(AgentEvent::PermissionRequest { .. }) => state_changed = true,
                    Msg::AgentEvent(AgentEvent::Error { .. }) => state_changed = true,
                    // Input messages
                    Msg::TextareaKey(_) => state_changed = true,
                    Msg::InsertNewline => state_changed = true,
                    Msg::Paste(_) => state_changed = true,
                    Msg::ClearInput => state_changed = true,
                    Msg::ClearInputConfirm => state_changed = true,
                    // Command palette
                    Msg::CommandPaletteFilter(_) => state_changed = true,
                    Msg::CommandPaletteBackspace => state_changed = true,
                    Msg::CommandPaletteUp => state_changed = true,
                    Msg::CommandPaletteDown => state_changed = true,
                    Msg::CommandPaletteConfirm => state_changed = true,
                    Msg::CommandPaletteCancelArgument => state_changed = true,
                    // Navigation
                    Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown => state_changed = true,
                    Msg::SessionTreeUp | Msg::SessionTreeDown => state_changed = true,
                    Msg::SessionTreeConfirm => state_changed = true,
                    Msg::OnboardingNavigateUp | Msg::OnboardingNavigateDown => state_changed = true,
                    Msg::OnboardingSelectProvider(_) | Msg::OnboardingSelectModel(_) => state_changed = true,
                    Msg::OnboardingKeyInput(_) | Msg::OnboardingKeyBackspace => state_changed = true,
                    Msg::OnboardingSearchInput(_) | Msg::OnboardingSearchBackspace => state_changed = true,
                    Msg::SelectUp | Msg::SelectDown => state_changed = true,
                    Msg::SelectConfirm | Msg::SelectToggleDetails => state_changed = true,
                    // Permission
                    Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways | Msg::PermissionSkip => state_changed = true,
                    // Mode changes
                    Msg::OpenCommandPalette | Msg::CloseModal | Msg::ConfirmModal => state_changed = true,
                    Msg::ToggleSessionTree | Msg::ToggleSidebar => state_changed = true,
                    Msg::SwitchModel => state_changed = true,
                    Msg::OnboardingNext | Msg::OnboardingBack | Msg::OnboardingSubmit | Msg::OnboardingSkip => state_changed = true,
                    Msg::EnterOnboarding => state_changed = true,
                    Msg::DirectCommand(_) => state_changed = true,
                    // Terminal events
                    Msg::Resize(..) => state_changed = true,
                    // Commands
                    Msg::Submit | Msg::Quit | Msg::ClearChat => state_changed = true,
                    Msg::Stop => state_changed = true,
                    // State updates
                    Msg::ModelsFetched(_) | Msg::ModelsFetchFailed(_) => state_changed = true,
                    Msg::SetGitInfo { .. } | Msg::SetTopBarMockChecks { .. } | Msg::SetTopBarRealChecks { .. } => state_changed = true,
                    Msg::SetInputRightInfo(_) | Msg::SetCurrentModel(_) | Msg::SetMockMode(_) => state_changed = true,
                    Msg::ResetAgentState | Msg::UpdateTopBarContext { .. } => state_changed = true,
                    Msg::SlashCommand(_) => state_changed = true,
                    Msg::PermissionTimeout => state_changed = true,
                    _ => {}
                }

                let change = tui.update(msg.clone());

                // Batch all pending messages before rendering
                let mut pending_cmds = change.cmds;
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
                        let more_change = tui.update(msg);
                        pending_cmds.extend(more_change.cmds);
                    }
                }

                // Execute commands (may produce more commands recursively)
                let mut needs_render = change.needs_render;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        let change = process_cmd(cmd, &mut tui, &mut agent_task, &permission_state, &msg_tx, &workspace, mock, &mut *settings, &base_system_prompt, &cancel).await;
                        needs_render |= change.needs_render;
                        next_cmds.extend(change.cmds);
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
                        Msg::Submit => {
                            logger.log_ui_event("submit");
                            // Also log to our structured logger
                            log::log_submit(&tui.state.textarea.lines().join("\n").chars().take(50).collect::<String>());
                        }
                        Msg::Quit => logger.log_ui_event("quit"),
                        Msg::ToggleSidebar => logger.log_ui_event("toggle_sidebar"),
                        Msg::ClearChat => logger.log_ui_event("clear_chat"),
                        Msg::AgentEvent(AgentEvent::PermissionRequest { .. }) => {
                            logger.log_ui_event("permission_request")
                        }
                        _ => {}
                    }
                }

                // Render: debounce to ~30 FPS max when streaming, immediate on state change
                let now = Instant::now();
                let elapsed_ms = now.duration_since(last_render).as_millis() as u64;
                let time_for_render = tui.state.agent_running && elapsed_ms >= MIN_RENDER_INTERVAL_MS;
                if state_changed || needs_render || time_for_render {
                    tui.render()?;
                    last_render = now;
                }
                } // end for msg in msgs
            }

            // Cursor blink (500ms)
            _ = cursor_interval.tick() => {
                let change = tui.update(Msg::CursorBlink);
                let mut pending_cmds = change.cmds;
                let mut needs_render = change.needs_render;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        let change = process_cmd(cmd, &mut tui, &mut agent_task, &permission_state, &msg_tx, &workspace, mock, &mut *settings, &base_system_prompt, &cancel).await;
                        needs_render |= change.needs_render;
                        next_cmds.extend(change.cmds);
                    }
                    pending_cmds = next_cmds;
                }
                if needs_render {
                    tui.render()?;
                }
            }

            // Animation tick (80ms) from TimerActor
            Some(_timer_msg) = timer_rx.recv() => {
                // WATCHDOG: Check if agent has been running too long without any events
                if tui.state.agent_running {
                    // Watchdog: 30 second timeout - force recovery if agent stuck
                    if let Some(start_time) = tui.state.agent_start_time {
                        if start_time.elapsed() > AGENT_WATCHDOG_TIMEOUT {
                            tracing::error!("[WATCHDOG] Agent stuck for 30s, forcing recovery");

                            // Send error event so UI gets notified (on_agent_error will add to messages)
                            let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
                                message: "Agent timed out after 30 seconds".to_string(),
                                error_type: "watchdog".to_string(),
                                recoverable: true,
                                context: "Agent was stuck and was recovered".to_string(),
                            })).await;

                            // Abort the agent task
                            if let Some(task) = agent_task.take() {
                                task.abort();
                            }

                            // Reset state via Msg (Critical #4)
                            tui.update(Msg::ResetAgentState);

                            // Log it
                            log::log_agent_error("Agent watchdog timeout - forced recovery");
                        }
                    }

                    // Heartbeat warning: 15 seconds without events - agent is slow
                    if last_agent_event.elapsed() > AGENT_HEARTBEAT_TIMEOUT {
                        // Agent is alive but slow - add a thinking indicator if not already present
                        // The UI can display this via existing state
                        tracing::debug!("[WATCHDOG] Agent silent for 15s, may be thinking...");
                    }
                }

                if !mock {
                    update_top_bar_context(&mut tui, &settings);
                }
                let change = tui.update(Msg::Tick);
                let mut pending_cmds = change.cmds;
                let mut needs_render = change.needs_render;
                while !pending_cmds.is_empty() {
                    let mut next_cmds = vec![];
                    for cmd in pending_cmds {
                        let change = process_cmd(cmd, &mut tui, &mut agent_task, &permission_state, &msg_tx, &workspace, mock, &mut *settings, &base_system_prompt, &cancel).await;
                        needs_render |= change.needs_render;
                        next_cmds.extend(change.cmds);
                    }
                    pending_cmds = next_cmds;
                }
                // Debounce tick renders: only render if needs_render or 33ms passed and agent running
                let now = Instant::now();
                let elapsed_ms = now.duration_since(last_render).as_millis() as u64;
                if needs_render || (tui.state.agent_running && elapsed_ms >= MIN_RENDER_INTERVAL_MS) {
                    tui.render()?;
                    last_render = now;
                }
            }

            // Signal handler: Ctrl+C (SIGINT) / SIGTERM
            _ = ctrl_c() => {
                info!("Received Ctrl+C, shutting down gracefully...");
                if let Some(logger) = event_logger {
                    logger.log_ui_event("sigint");
                }
                // Send error event before aborting so UI gets notified
                let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
                    message: "Agent interrupted by Ctrl+C".to_string(),
                    error_type: "interrupted".to_string(),
                    recoverable: true,
                    context: "User pressed Ctrl+C to stop the agent".to_string(),
                })).await;
                cancel.cancel();
                if let Some(task) = agent_task.take() {
                    task.abort();
                }
                break;
            }
        }
    }

    // Graceful shutdown: signal cancellation, wait up to 2s, then force if needed
    cancel.cancel();
    input_handle.shutdown();
    timer_handle.shutdown();
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
