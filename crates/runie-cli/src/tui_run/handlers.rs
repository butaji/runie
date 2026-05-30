use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use runie_agent::events::AgentEvent;
use runie_tui::pipe::InputMsg;
use runie_tui::Tui;

use crate::event_stream::EventStreamLogger;
use crate::event_logger as log;
use crate::settings::Settings;

use super::{process_cmd, input_to_msg, route_key_event, triggers_state_change, update_top_bar_context, MIN_RENDER_INTERVAL_MS};

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
    last_agent_event: &mut Instant,
    last_render: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let msg = input_to_msg(input_msg);
    let msgs = route_key_event(msg, &tui.state);
    handle_msgs(msgs, tui, agent_task, msg_tx, msg_rx, workspace, mock, settings, base_system_prompt, cancel, event_logger, last_agent_event, last_render).await
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
    last_agent_event: &mut Instant,
    last_render: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state_changed = false;

    for msg in msgs {
        // [RAILS] Route message to update()
        tracing::debug!("[RAILS] Routing {:?} to update()", msg);

        // Log agent events to event stream
        if let runie_tui::Msg::AgentEvent(ref agent_event) = msg {
            if let Some(logger) = event_logger {
                logger.log_agent_event(agent_event);
            }
            // Reset heartbeat tracker on any agent event
            *last_agent_event = Instant::now();
        }

        // Handle AgentEnd to clear agent task
        if let runie_tui::Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
            *agent_task = None;
            tui.clear_permission().await;
            state_changed = true;
        }

        // Handle Error to ensure state_changed is set for renders
        if let runie_tui::Msg::AgentEvent(AgentEvent::Error { .. }) = &msg {
            state_changed = true;
        }

        // Log model fetch events
        match &msg {
            runie_tui::Msg::ModelsFetched(models) => {
                tracing::info!("[RAILS] Received ModelsFetched with {} models", models.len());
            }
            runie_tui::Msg::ModelsFetchFailed(err) => {
                tracing::warn!("[RAILS] Received ModelsFetchFailed: {}", err);
            }
            _ => {}
        }

        // Mark state as changed for meaningful events (triggers immediate render)
        if triggers_state_change(&msg) {
            state_changed = true;
        }

        let change = tui.update(msg.clone());

        // Batch all pending messages before rendering
        let mut pending_cmds = change.cmds;
        while let Ok(msg) = msg_rx.try_recv() {
            if let runie_tui::Msg::AgentEvent(AgentEvent::AgentEnd { .. }) = &msg {
                *agent_task = None;
                tui.clear_permission().await;
            }
            // Convert raw key events in batched messages too
            let msgs = route_key_event(msg, &tui.state);
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
                let change = process_cmd(cmd, tui, agent_task, msg_tx, workspace, mock, settings, base_system_prompt, cancel).await;
                needs_render |= change.needs_render;
                next_cmds.extend(change.cmds);
            }
            pending_cmds = next_cmds;
        }

        // Update context info after agent events (skip in mock mode)
        if !mock {
            update_top_bar_context(tui, settings);
        }

        // Log key UI events
        if let Some(logger) = event_logger {
            match &msg {
                runie_tui::Msg::Submit => {
                    logger.log_ui_event("submit");
                    // Also log to our structured logger
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

        // Render: debounce to ~30 FPS max when streaming, immediate on state change
        let now = Instant::now();
        let elapsed_ms = now.duration_since(*last_render).as_millis() as u64;
        let time_for_render = tui.is_agent_running() && elapsed_ms >= MIN_RENDER_INTERVAL_MS;
        if state_changed || needs_render || time_for_render {
            tui.render()?;
            *last_render = now;
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
    _last_render: &mut Instant,
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
        tui.render()?;
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
    last_agent_event: &mut Instant,
    last_render: &mut Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    use super::{AGENT_WATCHDOG_TIMEOUT, AGENT_HEARTBEAT_TIMEOUT};

    // WATCHDOG: Check if agent has been running too long without any events
    if tui.is_agent_running() {
        // Watchdog: 30 second timeout - force recovery if agent stuck
        if let Some(start_time) = tui.agent_start_time() {
            if start_time.elapsed() > AGENT_WATCHDOG_TIMEOUT {
                tracing::error!("[WATCHDOG] Agent stuck for 30s, forcing recovery");

                // Send error event so UI gets notified (on_agent_error will add to messages)
                let _ = msg_tx.send(runie_tui::Msg::AgentEvent(AgentEvent::Error {
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
                tui.update(runie_tui::Msg::ResetAgentState);

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
    // Debounce tick renders: only render if needs_render or 33ms passed and agent running
    let now = Instant::now();
    let elapsed_ms = now.duration_since(*last_render).as_millis() as u64;
    if needs_render || (tui.is_agent_running() && elapsed_ms >= MIN_RENDER_INTERVAL_MS) {
        tui.render()?;
        *last_render = now;
    }
    Ok(())
}
