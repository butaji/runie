// ============================================================================
// View Model Builder Tests - Permission Modal
// ============================================================================

use crate::tui::state::{AppState, TuiMode};
use crate::tui::view_models::ViewModels;
use crate::components::CommandPalette;

fn make_state() -> AppState {
    AppState::default()
}

fn build_vms(state: &AppState) -> ViewModels {
    let render = crate::tui::state::RenderState::from(state);
    ViewModels::from_render_state(&render, &CommandPalette::default())
}

#[test]
fn test_permission_modal_vm_chat_mode_returns_none() {
    let mut state = make_state();
    state.mode = TuiMode::Chat;
    let vms = build_vms(&state);
    assert!(vms.permission_modal.is_none());
}

#[test]
fn test_permission_modal_vm_permission_mode_without_tool() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = None;
    let vms = build_vms(&state);
    assert!(vms.permission_modal.is_some());
    let modal = vms.permission_modal.unwrap();
    assert!(modal.tool.is_empty());
    assert!(modal.visible);
}

#[test]
fn test_permission_modal_vm_with_tool() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("Read".to_string());
    state.permission_modal.args = Some("{\"path\": \"foo.txt\"}".to_string());
    state.permission_modal.desc = Some("Read a file".to_string());
    let vms = build_vms(&state);
    let modal = vms.permission_modal.unwrap();
    assert_eq!(modal.tool, "Read");
    assert_eq!(modal.args, "{\"path\": \"foo.txt\"}");
    assert_eq!(modal.desc, "Read a file");
    assert!(modal.visible);
    assert_eq!(modal.selected, 0);
}

#[test]
fn test_permission_modal_vm_selected_always_zero() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("Write".to_string());
    let vms = build_vms(&state);
    assert_eq!(vms.permission_modal.unwrap().selected, 0);
}

#[test]
fn test_permission_modal_vm_invisible_when_not_permission_mode() {
    let mut state = make_state();
    state.mode = TuiMode::DiffViewer;
    state.permission_modal.tool = Some("Tool".to_string());
    let vms = build_vms(&state);
    assert!(vms.permission_modal.is_none());
}

#[test]
fn test_permission_modal_vm_write_tool() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("Write".to_string());
    state.permission_modal.args = Some("{\"path\": \"out.txt\", \"content\": \"hi\"}".to_string());
    state.permission_modal.desc = Some("Write content to a file".to_string());
    let vms = build_vms(&state);
    assert_eq!(vms.permission_modal.unwrap().tool, "Write");
}

#[test]
fn test_permission_modal_vm_bash_tool() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;
    state.permission_modal.tool = Some("Bash".to_string());
    state.permission_modal.args = Some("{\"command\": \"ls\"}".to_string());
    state.permission_modal.desc = Some("Execute a shell command".to_string());
    let vms = build_vms(&state);
    let modal = vms.permission_modal.unwrap();
    assert_eq!(modal.tool, "Bash");
    assert!(modal.desc.contains("shell"));
}
