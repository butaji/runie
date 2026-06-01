use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use runie_agent::events::{AgentEvent, PermissionDecision};
use runie_agent::loop_engine::{AgentLoopConfig, run_agent_loop, AgentLoopError};
use runie_agent::Hook;
use runie_ai::Provider;
use runie_tui::pipe::StateChange;

use crate::settings::Settings;
use crate::provider_factory::create_provider;
use crate::agent_spawn::create_agent_tools;
use runie_tui::{Cmd, Msg};

/// Process Cmds that need recursive handling (SlashCommand -> more Cmds)
pub async fn process_cmd(
    cmd: Cmd,
    tui: &mut runie_tui::Tui,
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    msg_tx: &mpsc::Sender<Msg>,
    workspace: &PathBuf,
    mock: bool,
    settings: &mut Settings,
    base_system_prompt: &str,
    _cancel: &CancellationToken,
) -> StateChange {
    match cmd {
        Cmd::SpawnAgent { messages } => handle_spawn_agent(
            messages, agent_task, tui, msg_tx, workspace, mock, settings, base_system_prompt,
        ).await,
        Cmd::SendPermission { decision } => handle_send_permission(tui, decision).await,
        Cmd::SlashCommand(slash_cmd) => handle_slash_command(tui, slash_cmd),
        Cmd::SaveSettings { provider, model, api_key } => handle_save_settings(tui, settings, provider, model, api_key),
        Cmd::FetchModels { provider_id, api_key } => handle_fetch_models(msg_tx, provider_id, api_key).await,
        Cmd::Rollback { tool_call_id } => handle_rollback(tool_call_id),
        Cmd::Interrupt => handle_interrupt(agent_task, tui, msg_tx).await,
    }
}

async fn handle_spawn_agent(
    messages: Vec<runie_agent::events::AgentMessage>,
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    tui: &runie_tui::Tui,
    msg_tx: &mpsc::Sender<Msg>,
    workspace: &PathBuf,
    mock: bool,
    settings: &Settings,
    base_system_prompt: &str,
) -> StateChange {
    if agent_task.is_some() {
        let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
            message: "Agent is already running. Wait for completion or press Ctrl+C to interrupt.".to_string(),
            error_type: "busy".to_string(),
            recoverable: true,
            context: "Attempted to spawn agent while another is running".to_string(),
        })).await;
        return StateChange::none();
    }

    crate::event_logger::log_agent_spawned();
    let provider = match create_provider(mock, settings) {
        Ok(p) => {
            crate::event_logger::log_provider_created(&settings.provider, &settings.model, true);
            p
        }
        Err(e) => {
            error!("Failed to create provider: {}", e);
            crate::event_logger::log_provider_created(&settings.provider, &settings.model, false);
            crate::event_logger::log_agent_error(&format!("Provider creation failed: {}", e));
            // CRITICAL FIX: Send error to TUI so it can display immediately and clean up
            let error_msg = format!("No API key configured for {}. Run /onboard or set {}_API_KEY.",
                settings.provider, settings.provider.to_uppercase());
            let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
                message: error_msg,
                error_type: "auth".to_string(),
                recoverable: true,
                context: e.to_string(),
            })).await;
            return StateChange::none();
        }
    };

    let ws = runie_tools::Workspace::new(workspace.clone());
    let registry = Arc::new(runie_tools::create_default_toolkit(ws));
    let tools = create_agent_tools(registry.clone());
    let safety_hook: Arc<dyn Hook> = Arc::new(runie_agent::SafetyHook);
    let hooks: Vec<Arc<dyn Hook>> = vec![safety_hook];

    let config = AgentLoopConfig {
        system_prompt: base_system_prompt.to_string(),
        model: settings.model.clone(),
        thinking_level: if settings.enable_thinking { "high" } else { "low" }.to_string(),
        max_turns: settings.max_turns,
    };

    let permission_state_clone = tui.permission_state.clone();
    let msg_tx_clone = msg_tx.clone();

    let task = tokio::spawn(async move {
        let provider_arc: Arc<dyn Provider> = provider.into();
        let result = tokio::time::timeout(
            crate::tui_run::AGENT_LOOP_TIMEOUT,
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
                crate::event_logger::log_agent_completed();
            }
            Ok(Err(e)) => {
                handle_agent_loop_error(&msg_tx_clone, &e).await;
            }
            Err(_) => {
                handle_agent_timeout(&msg_tx_clone).await;
            }
        }
    });

    *agent_task = Some(task);
    StateChange::none()
}

async fn handle_agent_loop_error(msg_tx: &mpsc::Sender<Msg>, e: &AgentLoopError) {
    tracing::error!("Agent loop error: {}", e);
    crate::event_logger::log_agent_error(&e.to_string());
    let (error_type, recoverable, context) = classify_error(e);
    let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
        message: e.to_string(),
        error_type,
        recoverable,
        context,
    })).await;
}

fn classify_error(e: &AgentLoopError) -> (String, bool, String) {
    match e {
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
    }
}

async fn handle_agent_timeout(msg_tx: &mpsc::Sender<Msg>) {
    tracing::error!("Agent loop timed out after 10 minutes");
    crate::event_logger::log_agent_error("Agent loop timed out after 10 minutes");
    let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
        message: "Agent loop timed out after 10 minutes".to_string(),
        error_type: "timeout".to_string(),
        recoverable: false,
        context: "Agent loop exceeded 10 minute timeout".to_string(),
    })).await;
}

async fn handle_send_permission(
    tui: &mut runie_tui::Tui,
    decision: PermissionDecision,
) -> StateChange {
    tui.set_permission(decision).await;
    StateChange::none()
}

fn handle_slash_command(tui: &mut runie_tui::Tui, slash_cmd: runie_core::slash_command::SlashCommand) -> StateChange {
    tui.update(Msg::SlashCommand(slash_cmd))
}

fn handle_save_settings(
    tui: &mut runie_tui::Tui,
    settings: &mut Settings,
    provider: String,
    model: String,
    api_key: String,
) -> StateChange {
    settings.provider = provider.clone();
    settings.model = model.clone();
    settings.api_key = Some(api_key.clone());

    tui.update(Msg::SetCurrentModel(Some(format!("{}/{}", provider, model))));
    tui.update(Msg::UpdateTopBarContext {
        model: model.clone(),
        context_window: None,
        estimated_tokens: None,
    });
    tui.update(Msg::SetInputRightInfo(format!("{} · {}", provider, model)));

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

    set_provider_env_vars(&provider, &api_key);
    StateChange::none()
}

fn set_provider_env_vars(provider: &str, api_key: &str) {
    let var_name = provider_env_var_name(provider);
    if !var_name.is_empty() {
        std::env::set_var(var_name, api_key);
    }
}

fn provider_env_var_name(provider: &str) -> &'static str {
    const PROVIDERS: &[(&str, &str)] = &[
        ("openai", "OPENAI_API_KEY"),
        ("anthropic", "ANTHROPIC_API_KEY"),
        ("google", "GOOGLE_API_KEY"),
        ("cohere", "COHERE_API_KEY"),
        ("mistral", "MISTRAL_API_KEY"),
        ("deepseek", "DEEPSEEK_API_KEY"),
        ("groq", "GROQ_API_KEY"),
        ("openrouter", "OPENROUTER_API_KEY"),
        ("huggingface", "HUGGINGFACE_API_KEY"),
        ("xai", "XAI_API_KEY"),
        ("azure", "AZURE_API_KEY"),
        ("moonshot", "MOONSHOT_API_KEY"),
        ("perplexity", "PERPLEXITY_API_KEY"),
        ("ollama", "OLLAMA_API_KEY"),
        ("hyperbolic", "HYPERBOLIC_API_KEY"),
        ("together", "TOGETHER_API_KEY"),
        ("zai", "ZAI_API_KEY"),
        ("minimax", "MINIMAX_API_KEY"),
        ("mira", "MIRA_API_KEY"),
        ("galadriel", "GALADRIEL_API_KEY"),
        ("llamafile", "LLAMAFILE_API_KEY"),
    ];
    PROVIDERS.iter().find(|(k, _)| *k == provider).map(|(_, v)| *v).unwrap_or("")
}

async fn handle_fetch_models(
    msg_tx: &mpsc::Sender<Msg>,
    provider_id: String,
    api_key: String,
) -> StateChange {
    crate::event_logger::log(crate::event_logger::Subsystem::PROVIDER, crate::event_logger::LogLevel::INFO, &format!("[ACTOR:ModelPicker] Starting fetch for provider: {}", provider_id));
    tracing::info!("[ACTOR:ModelPicker] Starting fetch for provider: {}", provider_id);
    let tx = msg_tx.clone();
    let provider_id_for_log = provider_id.clone();
    tokio::spawn(async move {
        tracing::debug!("[ACTOR:ModelPicker] Fetch task started");
        let fetcher = runie_ai::model_fetcher::create_fetcher(&provider_id);
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            fetcher.fetch_models(&api_key),
        ).await;
        match result {
            Ok(Ok(models)) => {
                tracing::info!("[ACTOR:ModelPicker] Fetched {} models for {}", models.len(), provider_id_for_log);
                if let Err(e) = tx.send(Msg::ModelsFetched(models)).await {
                    tracing::error!("[ACTOR:ModelPicker] Failed to send ModelsFetched: {}", e);
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("[ACTOR:ModelPicker] Fetch failed for {}: {}", provider_id_for_log, e);
                let _ = tx.send(Msg::ModelsFetchFailed(e.to_string())).await;
            }
            Err(_) => {
                tracing::warn!("[ACTOR:ModelPicker] Fetch timed out for {}", provider_id_for_log);
                let _ = tx.send(Msg::ModelsFetchFailed("fetch timed out after 30s".to_string())).await;
            }
        }
    });
    StateChange::none()
}

fn handle_rollback(tool_call_id: String) -> StateChange {
    info!("[Rollback] Tool {} cancelled - workspace state preserved", tool_call_id);
    StateChange::none()
}

async fn handle_interrupt(
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    tui: &mut runie_tui::Tui,
    msg_tx: &mpsc::Sender<Msg>,
) -> StateChange {
    let _ = msg_tx.send(Msg::AgentEvent(AgentEvent::Error {
        message: "Agent interrupted by user".to_string(),
        error_type: "interrupted".to_string(),
        recoverable: true,
        context: "User pressed Ctrl+C or sent interrupt signal".to_string(),
    })).await;
    if let Some(task) = agent_task.take() {
        task.abort();
    }
    tui.clear_permission().await;
    StateChange::none()
}
