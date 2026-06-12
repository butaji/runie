//! Permission mode handlers.

use crate::tui::state::AppState;

pub(super) fn toggle_permission_mode(state: &mut AppState) {
    use crate::tui::state::PermissionMode;
    state.permission_mode = match state.permission_mode {
        PermissionMode::Normal => PermissionMode::AutoApprove,
        PermissionMode::AutoApprove => PermissionMode::Plan,
        PermissionMode::Plan => PermissionMode::Normal,
    };
    let mode_name = match state.permission_mode {
        PermissionMode::Normal => "Normal",
        PermissionMode::AutoApprove => "AutoApprove",
        PermissionMode::Plan => "Plan",
    };
    state.input_right_info = format!("Mode: {}", mode_name);
}

pub(super) fn toggle_auto_approve(state: &mut AppState) {
    use crate::tui::state::PermissionMode;
    state.permission_mode = match state.permission_mode {
        PermissionMode::Normal => PermissionMode::AutoApprove,
        PermissionMode::AutoApprove => PermissionMode::Normal,
        PermissionMode::Plan => PermissionMode::AutoApprove,
    };
    let mode_name = match state.permission_mode {
        PermissionMode::Normal => "Normal",
        PermissionMode::AutoApprove => "YOLO",
        PermissionMode::Plan => "Plan",
    };
    state.input_right_info = format!("Mode: {}", mode_name);
}

pub(super) fn clear_always_approve(state: &mut AppState) {
    let count = state.allowed_tools.len() + state.allowed_categories.len();
    state.allowed_tools.clear();
    state.allowed_categories.clear();
    state.input_right_info = format!("Cleared {} always-approve entries", count);
}

pub(super) fn toggle_scroll_focus(state: &mut AppState) {
    state.scroll.scroll_focused = !state.scroll.scroll_focused;
    state.input_right_info = if state.scroll.scroll_focused {
        "[SCROLL]".to_string()
    } else {
        String::new()
    };
}
