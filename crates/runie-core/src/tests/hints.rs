use crate::model::{AppState, ChatMessage, Role};
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn hint_shows_expand_hotkey_by_default() {
    let state = fresh_state();
    let hint = state.hint_text();
    assert!(hint.contains("Ctrl+Shift+E"), "Hint should show expand key: {}", hint);
}

#[test]
fn hint_shows_send_when_input_has_text() {
    let mut state = fresh_state();
    state.input = "hello".to_string();
    let hint = state.hint_text();
    assert!(hint.contains("Enter=send"), "Hint should show send: {}", hint);
}

#[test]
fn hint_shows_steer_when_turn_active() {
    let mut state = fresh_state();
    state.turn_active = true;
    let hint = state.hint_text();
    assert!(hint.contains("Enter=steer"), "Hint should show steer: {}", hint);
}

#[test]
fn hint_shows_at_ref_when_suggestions_active() {
    let mut state = fresh_state();
    state.at_suggestions = Some(vec!["a.rs".to_string()]);
    let hint = state.hint_text();
    assert!(hint.contains("Tab=cycle"), "Hint should show @ref cycle: {}", hint);
    assert!(hint.contains("Enter=insert"), "Hint should show @ref insert: {}", hint);
}

#[test]
fn hint_always_shows_quit() {
    let state = fresh_state();
    let hint = state.hint_text();
    assert!(hint.contains("Ctrl+C=quit"), "Hint should always show quit: {}", hint);
}

#[test]
fn toggle_expand_collapses_all_thoughts() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "ToggleExpand should set global collapse");
}

#[test]
fn toggle_expand_collapses_all_tools() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s\nfile1".into(),
        timestamp: 0.0,
        id: "x1".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "ToggleExpand should set global collapse");
}

#[test]
fn toggle_expand_affects_all_elements() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "older thought".into(),
        timestamp: 0.0,
        id: "old".into(),
    });
    state.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 1.0,
        id: "new".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "Toggle should collapse ALL thoughts and tools globally");
}

#[test]
fn toggle_expand_noop_when_no_collapsible() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".into(),
        timestamp: 0.0,
        id: "u1".into(),
    });
    state.update(Event::ToggleExpand);
    assert!(state.all_collapsed, "Toggle should still flip global flag even with no thoughts/tools");
}

#[test]
fn toggle_expand_rebuilds_cache() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.ensure_fresh();
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let cache = state.elements_cache().to_vec();
    assert!(
        cache.iter().any(|e| matches!(e, crate::ui::elements::Element::ThoughtSummary { .. })),
        "ToggleExpand should rebuild cache"
    );
}

#[test]
fn toggle_expand_twice_restores() {
    let mut state = fresh_state();
    state.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
    });
    state.update(Event::ToggleExpand);
    state.update(Event::ToggleExpand);
    assert!(!state.all_collapsed, "Second toggle should expand all");
}
