use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode, PendingPermission};
use runie_agent::PermissionDecision;

/// Modes that block permission requests from interrupting
fn is_blocking_mode(mode: &TuiMode) -> bool {
    matches!(mode, TuiMode::Overlay | TuiMode::DiffViewer | TuiMode::SessionTree)
}

pub fn on_permission_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) {
    // BG-1 FIX: Queue permission if already in a blocking mode
    if is_blocking_mode(&state.mode) {
        state.permission_modal.pending_queue.push(PendingPermission {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: tool_args.clone(),
        });
        // Notify user that request is queued
        state.messages.push(MessageItem::System {
            text: format!("Permission for '{}' queued (waiting for current modal)", tool_name),
        });
        return;
    }
    
    // If permission modal is already open, queue the new request
    if state.mode == TuiMode::Permission || state.permission_modal.tool.is_some() {
        state.permission_modal.pending_queue.push(PendingPermission {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: tool_args.clone(),
        });
        return;
    }
    
    state.permission_modal.tool = Some(tool_name.clone());
    state.permission_modal.tool_call_id = Some(tool_call_id);
    state.permission_modal.args = Some(tool_args.clone());
    state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", tool_name));
    // P0-1 FIX: Start timeout tracking
    state.permission_modal.timeout_start = Some(std::time::Instant::now());
    state.permission_modal.timed_out = false;
    state.mode = TuiMode::Permission;
    // Announce the permission request in the message feed so the user has
    // a persistent record (and the chat scrollback reflects what the agent
    // is asking for) — see test_permission_request_adds_system_message.
    state.messages.push(MessageItem::System {
        text: format!("Permission requested: {}", tool_name),
    });
}

/// Process the next pending permission request from the queue (FIFO)
pub fn process_pending_permission(state: &mut AppState) -> Option<PendingPermission> {
    if state.permission_modal.pending_queue.is_empty() {
        None
    } else {
        Some(state.permission_modal.pending_queue.remove(0))
    }
}

pub fn handle_permission(state: &mut AppState, decision: PermissionDecision) -> Vec<super::AgentCmd> {
    let tool_call_id = state.permission_modal.tool_call_id.clone();
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;

    // P1-4 FIX: On cancel, trigger rollback for the tool that was pending
    let should_rollback = tool_call_id.is_some()
        && (matches!(decision, PermissionDecision::Deny { .. })
            || matches!(decision, PermissionDecision::Skip { .. }));

    let mut cmds = vec![super::AgentCmd::SendPermission { decision }];
    if should_rollback {
        if let Some(id) = tool_call_id {
            cmds.push(super::AgentCmd::Rollback { tool_call_id: id });
        }
    }

    // BG-1 FIX: Process next pending permission if any
    // BUG-09 FIX: Use remove(0) instead of pop() for FIFO order
    if !state.permission_modal.pending_queue.is_empty() {
        let pending = state.permission_modal.pending_queue.remove(0);
        state.permission_modal.tool = Some(pending.tool_name.clone());
        state.permission_modal.tool_call_id = Some(pending.tool_call_id.clone());
        state.permission_modal.args = Some(pending.tool_args.clone());
        state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", pending.tool_name));
        state.permission_modal.timeout_start = Some(std::time::Instant::now());
        state.permission_modal.timed_out = false;
        state.mode = TuiMode::Permission;
    } else {
        state.mode = TuiMode::Chat;
    }

    cmds
}

pub fn handle_permission_msg(state: &mut AppState, msg: Msg) -> Vec<super::AgentCmd> {
    let tool_call_id = state.permission_modal.tool_call_id.clone().unwrap_or_default();
    let tool_name = state.permission_modal.tool.clone().unwrap_or_default();
    let tool_args = state.permission_modal.args.clone().unwrap_or_default();
    let decision = match msg {
        Msg::PermissionConfirm => PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
        Msg::PermissionCancel => PermissionDecision::Deny { tool_call_id, tool_name, tool_args },
        Msg::PermissionAlways => PermissionDecision::AllowAlways { tool_call_id, tool_name, tool_args },
        Msg::PermissionSkip => PermissionDecision::Skip { tool_call_id, tool_name, tool_args },
        _ => PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
    };
    handle_permission(state, decision)
}

/// P2-5 FIX: Handle permission request timeout - send denial to agent
pub fn handle_permission_timeout(state: &mut AppState) -> Vec<super::AgentCmd> {
    state.permission_modal.timed_out = true;
    // P2-5 FIX: Show timeout message before denial
    state.messages.push(MessageItem::System {
        text: "Permission request timed out after 5 minutes.".to_string(),
    });
    
    let tool_call_id = state.permission_modal.tool_call_id.clone().unwrap_or_default();
    state.permission_modal.tool = None;
    state.permission_modal.tool_call_id = None;

    // BG-1 FIX: Process next pending permission if any
    // BUG-09 FIX: Use remove(0) instead of pop() for FIFO order
    if !state.permission_modal.pending_queue.is_empty() {
        let pending = state.permission_modal.pending_queue.remove(0);
        // Show the queued permission immediately
        state.permission_modal.tool = Some(pending.tool_name.clone());
        state.permission_modal.tool_call_id = Some(pending.tool_call_id.clone());
        state.permission_modal.args = Some(pending.tool_args.clone());
        state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", pending.tool_name));
        state.permission_modal.timeout_start = Some(std::time::Instant::now());
        state.permission_modal.timed_out = false;
        state.mode = TuiMode::Permission;
    } else {
        state.mode = TuiMode::Chat;
    }

    // P2-5 FIX: Send denial decision to agent loop so it doesn't wait indefinitely
    let tool_name = state.permission_modal.tool.clone().unwrap_or_default();
    let tool_args = state.permission_modal.args.clone().unwrap_or_default();
    vec![super::AgentCmd::SendPermission { decision: PermissionDecision::Deny { tool_call_id, tool_name, tool_args } }]
}
