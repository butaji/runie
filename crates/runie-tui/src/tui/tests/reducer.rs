//! Reducer tests for state updates.

use crate::tui::state::{AppState, AnimationState, CommandPaletteState, Msg, ScrollState, TopBarState, PermissionModalState, TuiMode};
use crate::components::{MessageItem, SessionTreeNavigator};
use crate::tui::update::update;
use runie_agent::{AgentEvent, AgentMessage, PermissionDecision};
use runie_ai::TokenUsage;
use ratatui_textarea::{TextArea, Input, Key};

fn make_state() -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::default(),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: None,
        top_bar: TopBarState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: TokenUsage::default(),
        session_token_usage: TokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
    }
}

fn make_state_with_text(text: &str) -> AppState {
    AppState {
        messages: vec![],
        textarea: TextArea::new(vec![text.to_string()]),
        input_right_info: String::new(),
        mode: TuiMode::Chat,
        running: true,
        show_sidebar: false,
        agent_running: false,
        current_model: None,
        top_bar: TopBarState::default(),
        permission_modal: PermissionModalState::default(),
        command_palette: CommandPaletteState::default(),
        scroll: ScrollState::default(),
        animation: AnimationState::default(),
        diff_viewer: None,
        token_usage: TokenUsage::default(),
        session_token_usage: TokenUsage::default(),
        session_tree: SessionTreeNavigator::new(),
        background_jobs: Vec::new(),
        onboarding: None,
    }
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

fn type_enter(state: &mut AppState) {
    state.textarea.input(Input { key: Key::Enter, ctrl: false, alt: false, shift: false });
}

#[test]
fn test_textarea_input() {
    let mut state = make_state();
    type_char(&mut state, 'h');
    type_char(&mut state, 'i');
    assert_eq!(state.textarea.lines(), &["hi".to_string()]);
}

#[test]
fn test_submit_clears_input() {
    let mut state = make_state_with_text("hi");
    let cmds = update(&mut state, Msg::Submit);
    assert!(state.textarea.is_empty());
    assert_eq!(state.messages.len(), 1);
    // Should return a SpawnAgent cmd
    assert_eq!(cmds.len(), 1);
    if let crate::tui::state::Cmd::SpawnAgent { .. } = &cmds[0] {
        // Expected
    } else {
        panic!("Expected SpawnAgent cmd");
    }
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "hi");
    } else {
        panic!("Expected User message");
    }
}

#[test]
fn test_submit_empty_does_nothing() {
    let mut state = make_state();
    let cmds = update(&mut state, Msg::Submit);
    assert_eq!(state.messages.len(), 0);
    assert!(cmds.is_empty());
}

#[test]
fn test_quit() {
    let mut state = make_state();
    update(&mut state, Msg::Quit);
    assert!(!state.running);
}

#[test]
fn test_toggle_sidebar() {
    let mut state = make_state();
    assert!(!state.show_sidebar);
    update(&mut state, Msg::ToggleSidebar);
    assert!(state.show_sidebar);
    update(&mut state, Msg::ToggleSidebar);
    assert!(!state.show_sidebar);
}

#[test]
fn test_agent_event_message_start() {
    let mut state = make_state();
    update(
        &mut state,
        Msg::AgentEvent(AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
            },
        }),
    );
    assert!(state.agent_running);
    assert_eq!(state.messages.len(), 1);
}

#[test]
fn test_agent_event_message_update() {
    use runie_agent::ContentPart;
    let mut state = make_state();
    // Start message
    update(
        &mut state,
        Msg::AgentEvent(AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
            },
        }),
    );

    // Update with text
    update(
        &mut state,
        Msg::AgentEvent(AgentEvent::MessageUpdate {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text {
                    text: "Hello".to_string(),
                }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
            },
        }),
    );

    assert_eq!(state.messages.len(), 1);
    if let MessageItem::Assistant { text, .. } = &state.messages[0] {
        assert_eq!(text, "Hello");
    } else {
        panic!("Expected Assistant message");
    }
}

#[test]
fn test_permission_cmds() {
    use crate::tui::state::Cmd;

    let mut state = make_state();

    // PermissionConfirm should return Allow decision
    let cmds = update(&mut state, Msg::PermissionConfirm);
    assert_eq!(cmds.len(), 1);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Allow { .. }));
    } else {
        panic!("Expected SendPermission cmd");
    }

    // PermissionCancel should return Deny decision
    let cmds = update(&mut state, Msg::PermissionCancel);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Deny { .. }));
    }

    // PermissionAlways should return AllowAlways decision
    let cmds = update(&mut state, Msg::PermissionAlways);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::AllowAlways { .. }));
    }

    // PermissionSkip should return Skip decision
    let cmds = update(&mut state, Msg::PermissionSkip);
    if let Cmd::SendPermission { decision } = &cmds[0] {
        assert!(matches!(*decision, PermissionDecision::Skip { .. }));
    }
}

#[test]
fn test_multi_line_submit() {
    let mut state = make_state();
    for c in "line1".chars() {
        type_char(&mut state, c);
    }
    // Simulate newline via textarea input
    type_enter(&mut state);
    for c in "line2".chars() {
        type_char(&mut state, c);
    }
    update(&mut state, Msg::Submit);

    assert!(state.textarea.is_empty());
    assert_eq!(state.messages.len(), 1);
    if let MessageItem::User { text, .. } = &state.messages[0] {
        assert_eq!(text, "line1\nline2");
    } else {
        panic!("Expected User message");
    }
}
