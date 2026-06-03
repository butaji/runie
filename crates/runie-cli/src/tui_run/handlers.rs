use std::path::PathBuf;
use std::time::Instant;
use std::io;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use runie_agent::events::AgentEvent;
use runie_tui::pipe::InputMsg;
use runie_tui::Tui;

use crate::event_stream::EventStreamLogger;
use crate::event_logger as log;
use crate::settings::Settings;

use super::{process_cmd, input_to_msg, route_key_event, triggers_state_change, update_top_bar_context, AGENT_WATCHDOG_TIMEOUT, MIN_RENDER_INTERVAL_MS};

/// Returns true if the error is a transient PTY/tty error that should not cause exit.
/// Examples: "Device not configured", "Not a typewriter", broken pipe, etc.
fn is_pty_error(err: &io::Error) -> bool {
    use std::io::ErrorKind::*;
    match err.kind() {
        NotConnected | BrokenPipe | ConnectionReset | ConnectionRefused | 
        InvalidInput | TimedOut => true,
        _ => err.raw_os_error().is_some_and(|e| {
            // ENODEV (macOS "Device not configured"), ENXIO (No such device), etc.
            e == 6 || e == 19 || e == 255
        }),
    }
}

/// Attempt to render, catching PTY errors gracefully.
/// Returns Ok(true) if render succeeded, Ok(false) if PTY error occurred (logged and skipped).
/// Returns Err only for non-PTY errors.
fn try_render(tui: &mut Tui) -> Result<bool, io::Error> {
    match tui.render() {
        Ok(()) => Ok(true),
        Err(e) => {
            tracing::warn!("Render error (transient): {}", e);
            Ok(false)
        }
    }
}

// ============================================================================
// Event Handlers - Extracted from tokio::select! arms
// ============================================================================

pub async fn handle_input_msg(
    input_msg: InputMsg,
    tui: &mut Tui,
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    msg_tx: &mpsc::Sender<runie_tui::Msg>,
    msg_rx: &mut mpsc::Receiver<runie_tui::Msg>,
    workspace: &PathBuf,
    mock: bool,
    settings: &mut Settings,
    base_system_prompt: &str,
    cancel: &CancellationToken,
    event_logger: Option<&EventStreamLogger>,
    last_render: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = input_to_msg(input_msg);
    let msgs = route_key_event(msg, &tui.state);
    handle_msgs(msgs, tui, agent_task, msg_tx, msg_rx, workspace, mock, settings, base_system_prompt, cancel, event_logger, last_render).await
}

pub async fn handle_msgs(
    msgs: Vec<runie_tui::Msg>,
    tui: &mut Tui,
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    msg_tx: &mpsc::Sender<runie_tui::Msg>,
    msg_rx: &mut mpsc::Receiver<runie_tui::Msg>,
    workspace: &PathBuf,
    mock: bool,
    settings: &mut Settings,
    base_system_prompt: &str,
    cancel: &CancellationToken,
    event_logger: Option<&EventStreamLogger>,
    last_render: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state_changed = false;

    for msg in msgs {
        tracing::debug!("[RAILS] Routing {:?} to update()", msg);

        if let runie_tui::Msg::AgentEvent(ref agent_event) = msg {
            if let Some(logger) = event_logger {
                logger.log_agent_event(agent_event);
            }
        }

        if let runie_tui::Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
            tracing::debug!("[RAILS] Received AgentEnd: clearing agent_task, is_agent_running={}", tui.is_agent_running());
            *agent_task = None;
            tui.clear_permission().await;
            state_changed = true;
        }

        if let runie_tui::Msg::AgentEvent(AgentEvent::Error { .. }) = &msg {
            state_changed = true;
        }

        match &msg {
            runie_tui::Msg::ModelsFetched(models) => {
                tracing::info!("[RAILS] Received ModelsFetched with {} models", models.len());
            }
            runie_tui::Msg::ModelsFetchFailed(err) => {
                tracing::warn!("[RAILS] Received ModelsFetchFailed: {}", err);
            }
            _ => {}
        }

        if triggers_state_change(&msg) {
            state_changed = true;
        }

        let change = tui.update(msg.clone());

        let mut pending_cmds = change.cmds;
        while let Ok(msg) = msg_rx.try_recv() {
            if let runie_tui::Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
                *agent_task = None;
                tui.clear_permission().await;
            }
            let msgs = route_key_event(msg, &tui.state);
            for msg in msgs {
                let more_change = tui.update(msg);
                pending_cmds.extend(more_change.cmds);
            }
        }

        let mut needs_render = change.needs_render;
        while !pending_cmds.is_empty() {
            let mut next_cmds = vec![];
            for cmd in pending_cmds {
                let change = process_cmd(cmd, tui, agent_task, msg_tx, workspace, mock, settings, base_system_prompt, cancel).await;
                needs_render |= change.needs_render;
                next_cmds.extend(change.cmds);
            }
            pending_cmds = next_cmds;
        }

        if !mock {
            update_top_bar_context(tui, settings);
        }

        if let Some(logger) = event_logger {
            match &msg {
                runie_tui::Msg::Submit => {
                    logger.log_ui_event("submit");
                    log::log_submit(&tui.input_text().chars().take(50).collect::<String>());
                }
                runie_tui::Msg::Quit => logger.log_ui_event("quit"),
                runie_tui::Msg::ToggleSidebar => logger.log_ui_event("toggle_sidebar"),
                runie_tui::Msg::ClearChat => logger.log_ui_event("clear_chat"),
                runie_tui::Msg::AgentEvent(AgentEvent::PermissionRequest { .. }) => {
                    logger.log_ui_event("permission_request")
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let elapsed_ms = now.duration_since(*last_render).as_millis() as u64;
        let time_for_render = tui.is_agent_running() && elapsed_ms >= MIN_RENDER_INTERVAL_MS;
        if state_changed || needs_render || time_for_render {
            if try_render(tui)? {
                *last_render = now;
            }
        }
    }

    Ok(())
}

pub async fn handle_cursor_blink(
    tui: &mut Tui,
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    msg_tx: &mpsc::Sender<runie_tui::Msg>,
    workspace: &PathBuf,
    mock: bool,
    settings: &mut Settings,
    base_system_prompt: &str,
    cancel: &CancellationToken,
) -> Result<(), Box<dyn std::error::Error>> {
    let change = tui.update(runie_tui::Msg::CursorBlink);
    let mut pending_cmds = change.cmds;
    let mut needs_render = change.needs_render;
    while !pending_cmds.is_empty() {
        let mut next_cmds = vec![];
        for cmd in pending_cmds {
            let change = process_cmd(cmd, tui, agent_task, msg_tx, workspace, mock, settings, base_system_prompt, cancel).await;
            needs_render |= change.needs_render;
            next_cmds.extend(change.cmds);
        }
        pending_cmds = next_cmds;
    }
    if needs_render {
        let _ = try_render(tui)?;
    }
    Ok(())
}

pub async fn handle_timer_tick(
    tui: &mut Tui,
    agent_task: &mut Option<tokio::task::JoinHandle<()>>,
    msg_tx: &mpsc::Sender<runie_tui::Msg>,
    workspace: &PathBuf,
    mock: bool,
    settings: &mut Settings,
    base_system_prompt: &str,
    cancel: &CancellationToken,
    last_render: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    // WATCHDOG: if the agent has been alive for more than AGENT_WATCHDOG_TIMEOUT
    // without producing an AgentEnd, force recovery.
    if tui.is_agent_running() {
        if let Some(start_time) = tui.agent_start_time() {
            if start_time.elapsed() > AGENT_WATCHDOG_TIMEOUT {
                tracing::error!("[WATCHDOG] Agent stuck, forcing recovery");
                let _ = msg_tx.send(runie_tui::Msg::AgentEvent(AgentEvent::Error {
                    message: format!("Agent timed out after {}s", AGENT_WATCHDOG_TIMEOUT.as_secs()),
                    error_type: "watchdog".to_string(),
                    recoverable: true,
                    context: "Agent was stuck and was recovered".to_string(),
                })).await;
                if let Some(task) = agent_task.take() {
                    task.abort();
                }
                tui.update(runie_tui::Msg::ResetAgentState);
                log::log_agent_error("Agent watchdog timeout - forced recovery");
            }
        }
    }

    if !mock {
        update_top_bar_context(tui, settings);
    }
    let change = tui.update(runie_tui::Msg::Tick);
    let mut pending_cmds = change.cmds;
    let mut needs_render = change.needs_render;
    while !pending_cmds.is_empty() {
        let mut next_cmds = vec![];
        for cmd in pending_cmds {
            let change = process_cmd(cmd, tui, agent_task, msg_tx, workspace, mock, settings, base_system_prompt, cancel).await;
            needs_render |= change.needs_render;
            next_cmds.extend(change.cmds);
        }
        pending_cmds = next_cmds;
    }
    let now = Instant::now();
    let elapsed_ms = now.duration_since(*last_render).as_millis() as u64;
    if needs_render || (tui.is_agent_running() && elapsed_ms >= MIN_RENDER_INTERVAL_MS) {
        if try_render(tui)? {
            *last_render = now;
        }
    }
    Ok(())
}
