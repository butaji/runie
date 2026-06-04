use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg, TuiMode, PendingPermission};
use runie_agent::PermissionDecision;

/// Categorize a tool by name for always-approve grouping.
pub fn tool_category(tool_name: &str) -> &str {
    match tool_name {
        "read_file" | "search" => "read-only",
        "write_file" | "edit_file" => "file-write",
        "bash" => "shell",
        _ => "other",
    }
}

/// Check if a tool is pre-approved via always-allow list.
fn is_tool_preapproved(state: &AppState, tool_name: &str) -> bool {
    state.allowed_tools.contains(tool_name)
        || state.allowed_categories.contains(tool_category(tool_name))
}

/// Modes that block permission requests from interrupting
fn is_blocking_mode(mode: &TuiMode) -> bool {
    matches!(mode, TuiMode::Overlay | TuiMode::DiffViewer | TuiMode::SessionTree)
}

pub fn on_permission_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) -> Vec<super::AgentCmd> {
    use crate::tui::state::PermissionMode;

    // Always-approve: check pre-approved tools/categories
    if is_tool_preapproved(state, &tool_name) {
        state.messages.push(MessageItem::System {
            text: format!("Always-approved: {}", tool_name),
        });
        return vec![super::AgentCmd::SendPermission {
            decision: PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
        }];
    }

    match state.permission_mode {
        PermissionMode::Plan => handle_plan_mode_request(state, tool_call_id, tool_name, tool_args),
        PermissionMode::AutoApprove => handle_auto_approve_request(state, tool_call_id, tool_name, tool_args),
        PermissionMode::Normal => handle_normal_permission_request(state, tool_call_id, tool_name, tool_args),
    }
}

fn handle_plan_mode_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) -> Vec<super::AgentCmd> {
    state.plan_modal.tools.push(crate::components::PlanTool {
        tool_call_id: tool_call_id.clone(),
        tool_name: tool_name.clone(),
        tool_args: tool_args.clone(),
    });
    state.messages.push(MessageItem::System {
        text: format!("Plan mode: auto-approved '{}'", tool_name),
    });
    vec![super::AgentCmd::SendPermission {
        decision: PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
    }]
}

fn handle_auto_approve_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) -> Vec<super::AgentCmd> {
    state.messages.push(MessageItem::System {
        text: format!("Auto-approved: {}", tool_name),
    });
    vec![super::AgentCmd::SendPermission {
        decision: PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
    }]
}

fn handle_normal_permission_request(state: &mut AppState, tool_call_id: String, tool_name: String, tool_args: String) -> Vec<super::AgentCmd> {
    const MAX_PENDING_PERMISSIONS: usize = 32;
    if is_blocking_mode(&state.mode) {
        if state.permission_modal.pending_queue.len() >= MAX_PENDING_PERMISSIONS {
            state.messages.push(MessageItem::System {
                text: format!(
                    "Permission queue full ({}); dropping '{}'",
                    MAX_PENDING_PERMISSIONS, tool_name
                ),
            });
            return vec![];
        }
        state.permission_modal.pending_queue.push(PendingPermission {
            tool_call_id: tool_call_id.clone(),
            tool_name: tool_name.clone(),
            tool_args: tool_args.clone(),
        });
        state.messages.push(MessageItem::System {
            text: format!("Permission for '{}' queued (waiting for current modal)", tool_name),
        });
        return vec![];
    }
    if state.mode == TuiMode::Permission || state.permission_modal.tool.is_some() {
        if state.permission_modal.pending_queue.len() >= MAX_PENDING_PERMISSIONS {
            state.messages.push(MessageItem::System {
                text: format!(
                    "Permission queue full ({}); dropping '{}'",
                    MAX_PENDING_PERMISSIONS, tool_name
                ),
            });
            return vec![];
        }
        state.permission_modal.pending_queue.push(PendingPermission {
            tool_call_id, tool_name, tool_args,
        });
        return vec![];
    }
    state.permission_modal.tool = Some(tool_name.clone());
    state.permission_modal.tool_call_id = Some(tool_call_id);
    state.permission_modal.args = Some(tool_args);
    state.permission_modal.desc = Some(format!("Agent wants to execute '{}'", tool_name));
    state.permission_modal.timeout_start = Some(std::time::Instant::now());
    state.permission_modal.timed_out = false;
    state.mode = TuiMode::Permission;
    state.messages.push(MessageItem::System {
        text: format!("Permission requested: {}", tool_name),
    });
    vec![]
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
    // If the modal is gone (race during teardown, or duplicate keypress),
    // the decision's tool_call_id would be "" which doesn't match any
    // pending request in the agent loop -- the agent would stall until
    // its 5-minute timeout. Early-return to avoid that.
    let Some(tool_call_id) = state.permission_modal.tool_call_id.clone() else {
        return vec![];
    };
    let Some(tool_name) = state.permission_modal.tool.clone() else {
        return vec![];
    };
    let tool_args = state.permission_modal.args.clone().unwrap_or_default();
    let decision = match msg {
        Msg::PermissionConfirm => PermissionDecision::Allow { tool_call_id, tool_name, tool_args },
        Msg::PermissionCancel => PermissionDecision::Deny { tool_call_id, tool_name, tool_args },
        Msg::PermissionAlways => {
            state.allowed_tools.insert(tool_name.clone());
            state.allowed_categories.insert(tool_category(&tool_name).to_string());
            state.messages.push(MessageItem::System {
                text: format!("Always-approve: {} ({})", tool_name, tool_category(&tool_name)),
            });
            PermissionDecision::AllowAlways { tool_call_id, tool_name, tool_args }
        }
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
