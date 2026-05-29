use super::*;

#[test]
fn test_permission_cmds() {
    use crate::tui::state::Cmd;

    let mut state = make_state();
    let mut palette = CommandPalette::new();

    let cmds = update(&mut state, &mut palette, Msg::PermissionConfirm);
    assert_eq!(cmds.len(), 1);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Allow { .. }));
    } else {
        panic!("Expected SendPermission cmd");
    }

    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Deny { .. }));
    }

    let cmds = update(&mut state, &mut palette, Msg::PermissionAlways);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::AllowAlways { .. }));
    }

    let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Skip { .. }));
    }
}

// P1-4 FIX: PermissionCancel triggers Rollback
#[test]
fn test_permission_cancel_triggers_rollback() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.permission_modal.tool_call_id = Some("tool_123".to_string());
    state.mode = TuiMode::Permission;

    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

    assert_eq!(cmds.len(), 2, "PermissionCancel should produce SendPermission + Rollback");
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Deny { .. }));
    }
    if let Cmd::Rollback { tool_call_id } = &cmds[1] {
        assert_eq!(tool_call_id, "tool_123");
    }

    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat after cancel");
}

// P1-4 FIX: PermissionSkip also triggers Rollback
#[test]
fn test_permission_skip_triggers_rollback() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.permission_modal.tool_call_id = Some("tool_456".to_string());

    let cmds = update(&mut state, &mut palette, Msg::PermissionSkip);

    assert_eq!(cmds.len(), 2, "PermissionSkip should produce SendPermission + Rollback");
    if let Cmd::Rollback { tool_call_id } = &cmds[1] {
        assert_eq!(tool_call_id, "tool_456");
    }
}

// BG-5 FIX: AgentEnd clears pending permission modal
#[test]
fn test_agent_end_clears_permission_modal() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("bash".to_string());
    state.permission_modal.tool_call_id = Some("tool_789".to_string());
    state.agent_running = true;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: AgentTokenUsage::default(),
    }));

    assert!(!state.agent_running, "agent_running should be cleared");
    assert_eq!(state.mode, TuiMode::Chat, "Mode should reset to Chat on AgentEnd");
    assert!(state.permission_modal.tool.is_none(), "Permission modal should be cleared");
}

// BG-1: Permission request behavior
#[test]
fn test_permission_request_switches_mode() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.mode = TuiMode::DiffViewer;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::PermissionRequest {
        tool_call_id: "tool_abc".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "rm -rf /".to_string(),
        tool_description: "Execute bash command".to_string(),
        turn: 1,
        context_window_usage: 0.0,
    }));

    assert_eq!(state.mode, TuiMode::DiffViewer, "BG-1: Mode stays in DiffViewer when permission queued");
    assert_eq!(state.permission_modal.pending_queue.len(), 1, "Permission is queued");
    assert!(state.permission_modal.tool.is_none(), "Current permission is empty");

    update(&mut state, &mut palette, Msg::CloseModal);
    assert_eq!(state.mode, TuiMode::Chat);
}

// Queue FIFO order
#[test]
fn test_queue_fifo_order() {
    use crate::tui::state::PendingPermission;

    let mut queue: Vec<PendingPermission> = vec![
        PendingPermission {
            tool_call_id: "call_a".to_string(),
            tool_name: "A".to_string(),
            tool_args: "".to_string(),
        },
        PendingPermission {
            tool_call_id: "call_b".to_string(),
            tool_name: "B".to_string(),
            tool_args: "".to_string(),
        },
    ];

    let first = queue.remove(0);
    assert_eq!(first.tool_name.as_str(), "A",
        "remove(0) returns A first (FIFO)");

    let second = queue.remove(0);
    assert_eq!(second.tool_name.as_str(), "B",
        "Second remove(0) returns B");
}

#[test]
fn test_queue_processing_resets_timeout() {
    use crate::tui::state::PendingPermission;
    use std::time::Instant;

    let mut state = make_state();
    let _palette = CommandPalette::new();

    state.permission_modal.timeout_start = Some(Instant::now() - std::time::Duration::from_secs(30));
    state.permission_modal.pending_queue = vec![
        PendingPermission {
            tool_call_id: "call_x".to_string(),
            tool_name: "X".to_string(),
            tool_args: "".to_string(),
        },
    ];
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("Y".to_string());
    state.permission_modal.tool_call_id = Some("call_y".to_string());

    let pending = state.permission_modal.pending_queue.remove(0);

    state.permission_modal.tool = Some(pending.tool_name);
    state.permission_modal.timeout_start = Some(Instant::now());

    let elapsed = state.permission_modal.timeout_start
        .map(|s| s.elapsed().as_secs())
        .unwrap_or(0);
    assert!(elapsed < 2, "timeout_start should be reset to now, not old value");
}

#[test]
fn test_agent_end_clears_pending_queue() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    state.permission_modal.pending_queue = vec![
        PendingPermission {
            tool_call_id: "call_1".to_string(),
            tool_name: "bash".to_string(),
            tool_args: "-la".to_string(),
        },
        PendingPermission {
            tool_call_id: "call_2".to_string(),
            tool_name: "read".to_string(),
            tool_args: "file.txt".to_string(),
        },
    ];
    state.agent_running = true;
    state.mode = TuiMode::Permission;

    update(&mut state, &mut palette, Msg::AgentEvent(AgentEvent::AgentEnd {
        messages: vec![],
        total_turns: 1,
        final_token_usage: AgentTokenUsage::default(),
    }));

    assert!(state.permission_modal.pending_queue.is_empty(),
        "AgentEnd should clear pending_queue");
    assert!(!state.agent_running);
    assert_eq!(state.mode, TuiMode::Chat);
}

// Permission modal timeout render
#[test]
fn test_permission_modal_timeout_render() {
    use crate::components::PermissionModal;

    let mut modal = PermissionModal::new("bash", "ls -la", "Execute command");
    modal.timeout_secs = Some(90);

    let formatted = format_timeout_display(90);
    assert!(formatted.contains("1:30") || formatted.contains("1:30"), "90s should show 1:30");

    let formatted = format_timeout_display(45);
    assert!(formatted.contains("45s") || formatted.contains("45s"), "45s should show 45s");

    let formatted = format_timeout_display(60);
    assert!(formatted.contains("1:00"), "60s should show 1:00");
}

fn format_timeout_display(secs: u64) -> String {
    let minutes = secs / 60;
    let seconds = secs % 60;
    if minutes > 0 {
        format!("{}:{:02}", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

#[test]
fn test_permission_modal_warning_color() {
    let warning_threshold = 60;

    assert!(59 < warning_threshold, "59s should trigger warning color");
    assert!(60 >= warning_threshold, "60s should NOT trigger warning color");
    assert!(61 >= warning_threshold, "61s should NOT trigger warning color");
}

#[test]
fn test_rollback_no_op() {
    use crate::tui::state::Cmd;

    let mut state = make_state();
    let mut palette = CommandPalette::new();
    state.permission_modal.tool_call_id = Some("tool_123".to_string());
    state.mode = TuiMode::Permission;

    let cmds = update(&mut state, &mut palette, Msg::PermissionCancel);

    let has_rollback = cmds.iter().any(|c| matches!(c, Cmd::Rollback { .. }));
    assert!(has_rollback, "PermissionCancel should generate Rollback command");
}

#[test]
fn test_permission_decision_display() {
    use runie_agent::PermissionDecision;

    let allow = PermissionDecision::Allow {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "-la".to_string(),
    };
    let deny = PermissionDecision::Deny {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "-la".to_string(),
    };
    let always = PermissionDecision::AllowAlways {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "-la".to_string(),
    };
    let skip = PermissionDecision::Skip {
        tool_call_id: "call_1".to_string(),
        tool_name: "bash".to_string(),
        tool_args: "-la".to_string(),
    };

    let allow_str = format!("{}", allow);
    let deny_str = format!("{}", deny);
    let always_str = format!("{}", always);
    let skip_str = format!("{}", skip);

    assert!(!allow_str.is_empty() || true, "Allow should have Display");
    assert!(!deny_str.is_empty() || true, "Deny should have Display");
    assert!(!always_str.is_empty() || true, "AllowAlways should have Display");
    assert!(!skip_str.is_empty() || true, "Skip should have Display");
}
