// ============================================================================
// View Model Builder Tests - Message List
// ============================================================================

use crate::components::MessageItem;
use crate::tui::state::AppState;
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
fn test_message_list_vm_empty() {
    let state = make_state();
    let vms = build_vms(&state);
    assert!(vms.message_list.messages.is_empty());
}

#[test]
fn test_message_list_vm_single_user_message() {
    let mut state = make_state();
    state.messages = std::sync::Arc::new([
        MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        },
    ]);
    let vms = build_vms(&state);
    assert_eq!(vms.message_list.messages.len(), 1);
}

#[test]
fn test_message_list_vm_single_assistant_message() {
    let mut state = make_state();
    state.messages = std::sync::Arc::new([
        MessageItem::Assistant {
            text: "Hi there".to_string(),
            model: Some("gpt-4o".to_string()),
            timestamp: None,
        },
    ]);
    let vms = build_vms(&state);
    assert_eq!(vms.message_list.messages.len(), 1);
}

#[test]
fn test_message_list_vm_mixed_messages() {
    let mut state = make_state();
    state.messages = std::sync::Arc::new([
        MessageItem::User {
            text: "Hello".to_string(),
            model: None,
            timestamp: None,
        },
        MessageItem::Assistant {
            text: "Hi".to_string(),
            model: Some("gpt-4o".to_string()),
            timestamp: None,
        },
        MessageItem::ToolCall {
            name: "read_file".to_string(),
            args: "{}".to_string(),
            result: Some("file contents".to_string()),
            is_error: false,
        },
    ]);
    let vms = build_vms(&state);
    assert_eq!(vms.message_list.messages.len(), 3);
}

#[test]
fn test_message_list_vm_scroll_offset() {
    let mut state = make_state();
    state.scroll.feed_offset = 42;
    let vms = build_vms(&state);
    assert_eq!(vms.message_list.scroll_offset, 42);
}

#[test]
fn test_message_list_vm_agent_running() {
    let mut state = make_state();
    state.agent_running = true;
    let vms = build_vms(&state);
    assert!(vms.message_list.agent_running);
}

#[test]
fn test_message_list_vm_not_running() {
    let state = make_state();
    let vms = build_vms(&state);
    assert!(!vms.message_list.agent_running);
}
