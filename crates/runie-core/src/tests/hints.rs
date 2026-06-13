use crate::event::Event;
use crate::model::{AppState, ChatMessage, Role};

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn hint_shows_expand_hotkey_by_default() {
    let state = fresh_state();
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+shift+e"),
        "Hint should show expand key: {}",
        hint
    );
}

#[test]
fn hint_shows_send_when_input_has_text() {
    let mut state = fresh_state();
    state.input.input = "hello".to_string();
    let hint = state.hint_text();
    assert!(
        hint.contains("enter send"),
        "Hint should show send: {}",
        hint
    );
}

#[test]
fn hint_shows_steer_when_turn_active() {
    let mut state = fresh_state();
    state.agent.turn_active = true;
    let hint = state.hint_text();
    assert!(
        hint.contains("enter steer"),
        "Hint should show steer: {}",
        hint
    );
}

#[test]
fn hint_shows_at_ref_when_suggestions_active() {
    let mut state = fresh_state();
    state.completion.at_suggestions = Some(vec!["a.rs".to_string()]);
    let hint = state.hint_text();
    assert!(
        hint.contains("tab cycle"),
        "Hint should show @ref cycle: {}",
        hint
    );
    assert!(
        hint.contains("enter insert"),
        "Hint should show @ref insert: {}",
        hint
    );
}

#[test]
fn hint_always_shows_quit() {
    let state = fresh_state();
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+c quit"),
        "Hint should always show quit: {}",
        hint
    );
}

#[test]
fn toggle_expand_collapses_all_thoughts() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "ToggleExpand should set global collapse"
    );
}

#[test]
fn toggle_expand_collapses_all_tools() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s\nfile1".into(),
        timestamp: 0.0,
        id: "x1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "ToggleExpand should set global collapse"
    );
}

#[test]
fn toggle_expand_affects_all_elements() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "older thought".into(),
        timestamp: 0.0,
        id: "old".into(),
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Tool,
        content: "◆ Ran list_files 0.5s".into(),
        timestamp: 1.0,
        id: "new".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle should collapse ALL thoughts and tools globally"
    );
}

#[test]
fn toggle_expand_noop_when_no_collapsible() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        content: "hello".into(),
        timestamp: 0.0,
        id: "u1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    assert!(
        state.view.all_collapsed,
        "Toggle should still flip global flag even with no thoughts/tools"
    );
}

#[test]
fn toggle_expand_rebuilds_cache() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning\nline two".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.ensure_fresh();
    state.update(Event::ToggleExpand);
    state.ensure_fresh();
    let cache = state.view.elements_cache().to_vec();
    assert!(
        cache
            .iter()
            .any(|e| matches!(e, crate::ui::elements::Element::ThoughtSummary { .. })),
        "ToggleExpand should rebuild cache"
    );
}

#[test]
fn toggle_expand_twice_restores() {
    let mut state = fresh_state();
    state.session.messages.push(ChatMessage {
        role: Role::Thought,
        content: "Deep reasoning".into(),
        timestamp: 0.0,
        id: "t1".into(),
        ..Default::default()
    });
    state.update(Event::ToggleExpand);
    state.update(Event::ToggleExpand);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
}

// ─── Transient messages ─────────────────────────────────────────────────

#[test]
fn transient_message_sets_content_and_expiry() {
    let mut state = fresh_state();
    state.update(Event::TransientMessage {
        content: "hello".into(),
        level: crate::event::TransientLevel::Info,
    });
    assert_eq!(state.transient_message, Some("hello".into()));
    assert!(
        state.transient_until.is_some(),
        "Transient message should have expiry"
    );
}

#[test]
fn transient_error_sets_content_without_expiry() {
    let mut state = fresh_state();
    state.update(Event::TransientError {
        content: "err".into(),
    });
    assert_eq!(state.transient_message, Some("err".into()));
    assert!(
        state.transient_until.is_none(),
        "Transient error should NOT have expiry"
    );
}

#[test]
fn clear_transient_unsets_message() {
    let mut state = fresh_state();
    state.update(Event::TransientError {
        content: "err".into(),
    });
    state.update(Event::ClearTransient);
    assert!(state.transient_message.is_none());
    assert!(state.transient_until.is_none());
}

#[test]
fn transient_message_overwrites_existing() {
    let mut state = fresh_state();
    state.update(Event::TransientMessage {
        content: "first".into(),
        level: crate::event::TransientLevel::Info,
    });
    state.update(Event::TransientMessage {
        content: "second".into(),
        level: crate::event::TransientLevel::Info,
    });
    assert_eq!(state.transient_message, Some("second".into()));
}

#[test]
fn transient_message_in_snapshot() {
    let mut state = fresh_state();
    state.update(Event::TransientMessage {
        content: "snap".into(),
        level: crate::event::TransientLevel::Info,
    });
    state.ensure_fresh();
    let snap = state.snapshot();
    assert_eq!(snap.transient_message, Some("snap".into()));
    assert_eq!(
        snap.transient_level,
        Some(crate::event::TransientLevel::Info)
    );
}

#[test]
fn transient_success_has_expiry() {
    let mut state = fresh_state();
    state.update(Event::TransientMessage {
        content: "ok".into(),
        level: crate::event::TransientLevel::Success,
    });
    assert!(
        state.transient_until.is_some(),
        "Success transient should have expiry"
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[test]
fn transient_error_has_no_expiry() {
    let mut state = fresh_state();
    state.update(Event::TransientError {
        content: "err".into(),
    });
    assert!(
        state.transient_until.is_none(),
        "Error transient should NOT have expiry"
    );
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Error)
    );
}

#[test]
fn notify_emits_success_transient() {
    let mut state = fresh_state();
    state.notify(
        "Theme switched".into(),
        crate::event::TransientLevel::Success,
    );
    assert_eq!(state.transient_message, Some("Theme switched".into()));
    assert_eq!(
        state.transient_level,
        Some(crate::event::TransientLevel::Success)
    );
}

#[test]
fn system_msg_not_in_feed_when_using_notify() {
    let mut state = fresh_state();
    state.notify("hello".into(), crate::event::TransientLevel::Info);
    let sys_count = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::System)
        .count();
    assert_eq!(sys_count, 0, "notify should not add to message feed");
}
