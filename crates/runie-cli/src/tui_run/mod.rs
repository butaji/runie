mod cmd;
mod events;
mod handlers;
mod setup;

pub use cmd::process_cmd;
pub use events::{input_to_msg, route_key_event, triggers_state_change};
pub use setup::{needs_onboarding, update_top_bar_context};
pub use handlers::{handle_input_msg, handle_msgs, handle_cursor_blink, handle_timer_tick};

use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::signal::ctrl_c;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::info;
use runie_agent::events::AgentEvent;
use runie_tui::actors::{spawn_actor, input::InputActor as TuiInputActor, timer::TimerActor};
use runie_tui::{Msg, Tui, TuiConfig};

use crate::event_stream::EventStreamLogger;
use crate::settings::Settings;
use crate::context_loader::ContextLoader;

// Watchdog timeouts for agent stuck detection
pub const AGENT_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(30);
pub const AGENT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);
pub const MIN_RENDER_INTERVAL_MS: u64 = 33; // ~30 FPS max render rate

pub async fn run_tui(
    workspace: PathBuf,
    mock: bool,
    settings: &mut Settings,
    force_setup: bool,
    event_logger: Option<&EventStreamLogger>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut tui, base_system_prompt) = initialize_tui(&workspace, mock, settings, force_setup, event_logger).await?;

    // Unified message channel for ALL event sources
    let (msg_tx, mut msg_rx) = mpsc::channel::<Msg>(100);

    // Cooperative cancellation token for graceful shutdown
    let cancel = CancellationToken::new();

    // Spawn new actors using the actor framework
    let (input_handle, mut input_rx) = spawn_actor(TuiInputActor::new());
    let (timer_handle, mut timer_rx) = spawn_actor(TimerActor::new(80)); // 80ms animation tick

    // Agent task handle for the running agent loop
    let mut agent_task: Option<tokio::task::JoinHandle<()>> = None;

    // Track last agent event time for watchdog
    let mut last_agent_event = Instant::now();

    // Animation timers - cursor blink only (animation tick is now via TimerActor)
    let mut cursor_interval = interval(Duration::from_millis(500));

    // TEA main loop
    let mut last_render = Instant::now();

    while tui.is_running() {
        tokio::select! {
            biased;
            Some(input_msg) = input_rx.recv() => {
                handle_input_msg(
                    input_msg,
                    &mut tui,
                    &mut agent_task,
                    &msg_tx,
                    &mut msg_rx,
                    &workspace,
                    mock,
                    settings,
                    &base_system_prompt,
                    &cancel,
                    event_logger,
                    &mut last_agent_event,
                    &mut last_render,
                ).await?;
            }
            _ = cursor_interval.tick() => {
                handle_cursor_blink(
                    &mut tui,
                    &mut agent_task,
                    &msg_tx,
                    &workspace,
                    mock,
                    settings,
                    &base_system_prompt,
                    &cancel,
                    &mut last_render,
                ).await?;
            }

            Some(_timer_msg) = timer_rx.recv() => {
                handle_timer_tick(
                    &mut tui,
                    &mut agent_task,
                    &msg_tx,
                    &workspace,
                    mock,
                    settings,
                    &base_system_prompt,
                    &cancel,
                    &mut last_agent_event,
                    &mut last_render,
                ).await?;
            }
            Some(msg) = msg_rx.recv() => {
                handle_msgs(
                    vec![msg],
                    &mut tui,
                    &mut agent_task,
                    &msg_tx,
                    &mut msg_rx,
                    &workspace,
                    mock,
                    settings,
                    &base_system_prompt,
                    &cancel,
                    event_logger,
                    &mut last_agent_event,
                    &mut last_render,
                ).await?;
            }
            _ = ctrl_c() => {
                handle_ctrl_c(&mut agent_task, &cancel, event_logger, &msg_tx).await;
                break;
            }
        }
    }

    shutdown_tui(tui, cancel, input_handle, timer_handle, agent_task, event_logger).await
}

async fn initialize_tui(
    workspace: &PathBuf,
    mock: bool,
    settings: &mut Settings,
    force_setup: bool,
    event_logger: Option<&EventStreamLogger>,
) -> Result<(Tui, String), Box<dyn std::error::Error>> {
    // Load AGENTS.md context files (skip in mock mode)
    let (context_files, loaded_paths, git_info) = if mock {
        (Vec::new(), Vec::new(), crate::git::GitInfo::default())
    } else {
        let context_files = ContextLoader::load();
        let loaded_paths = ContextLoader::loaded_paths();
        let git_info = crate::git::detect_git_info(workspace);
        (context_files, loaded_paths, git_info)
    };

    // Check for system prompt override
    let base_system_prompt = if let Some(override_prompt) = ContextLoader::system_override() {
        override_prompt
    } else {
        crate::context_loader::build_system_prompt(&context_files)
    };

    let mut tui = Tui::new(TuiConfig::default())?;

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
        tui.update(Msg::SetTopBarMockChecks {
            checks_passed: Some(4),
            checks_total: Some(4),
            percentage: Some(4.5),
            context_badges: Vec::new(),
        });
    } else {
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

    // Propagate mock mode to the TUI so model pickers and onboarding can show the mock provider.
    tui.update(Msg::SetMockMode(mock));

    // Check if onboarding is needed
    let needs_setup = force_setup || (!mock && needs_onboarding(settings));
    if needs_setup {
        tui.update(Msg::EnterOnboarding);
        tui.update(Msg::AgentEvent(AgentEvent::Message {
            role: "system".to_string(),
            content: "Welcome! Let's set up your AI assistant.".to_string(),
        }));
    } else {
        tui.update(Msg::AgentEvent(AgentEvent::Message {
            role: "system".to_string(),
            content: if mock {
                format!("Mock mode — no API calls. Model: {}{}", settings.model, context_info)
            } else {
                format!("Using {} ({}){}", settings.provider, settings.model, context_info)
            },
        }));
    }

    if !loaded_paths.is_empty() {
        info!("Loaded context: {}", loaded_paths.join(", "));
    }

    if let Some(logger) = event_logger {
        logger.log_ui_event("startup");
    }

    tui.render()?;
    Ok((tui, base_system_prompt))
}

async fn handle_ctrl_c(
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    cancel: &CancellationToken,
    event_logger: Option<&EventStreamLogger>,
    msg_tx: &mpsc::Sender<Msg>,
) {
    info!("Received Ctrl+C, shutting down gracefully...");
    if let Some(logger) = event_logger {
        logger.log_ui_event("sigint");
    }
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
}

async fn shutdown_tui(
    mut tui: Tui,
    cancel: CancellationToken,
    input_handle: runie_tui::actors::ActorHandle,
    timer_handle: runie_tui::actors::ActorHandle,
    agent_task: Option<tokio::task::JoinHandle<()>>,
    event_logger: Option<&EventStreamLogger>,
) -> Result<(), Box<dyn std::error::Error>> {
    cancel.cancel();
    input_handle.shutdown();
    timer_handle.shutdown();
    if let Some(task) = agent_task {
        task.abort();
        let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
    }
    tui.cleanup()?;
    if let Some(logger) = event_logger {
        logger.log_ui_event("shutdown");
    }
    Ok(())
}
