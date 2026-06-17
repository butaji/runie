use crate::commands::DialogState;
use crate::dialog::{Panel, PanelStack};
use crate::event::{ControlEvent, DialogEvent, SystemEvent};
use crate::model::{AppState, ChatMessage, Role};
use crate::orchestrator::ExecutionMode;
use crate::update::input::{feed_focused_hints, modal_hints, team_mode_hints};

fn fresh_state() -> AppState {
    AppState::default()
}

#[test]
fn hint_shows_expand_hotkey_by_default() {
    let state = fresh_state();
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+o"),
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
    state.update(ControlEvent::ToggleExpand);
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
    state.update(ControlEvent::ToggleExpand);
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
    state.update(ControlEvent::ToggleExpand);
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
    state.update(ControlEvent::ToggleExpand);
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
    state.update(ControlEvent::ToggleExpand);
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
    state.update(ControlEvent::ToggleExpand);
    state.update(ControlEvent::ToggleExpand);
    assert!(!state.view.all_collapsed, "Second toggle should expand all");
}

// ─── Transient messages ─────────────────────────────────────────────────

#[test]
fn transient_message_sets_content_and_expiry() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
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
    state.update(SystemEvent::TransientError {
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
    state.update(SystemEvent::TransientError {
        content: "err".into(),
    });
    state.update(SystemEvent::ClearTransient);
    assert!(state.transient_message.is_none());
    assert!(state.transient_until.is_none());
}

#[test]
fn transient_message_overwrites_existing() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
        content: "first".into(),
        level: crate::event::TransientLevel::Info,
    });
    state.update(SystemEvent::TransientMessage {
        content: "second".into(),
        level: crate::event::TransientLevel::Info,
    });
    assert_eq!(state.transient_message, Some("second".into()));
}

#[test]
fn transient_message_in_snapshot() {
    let mut state = fresh_state();
    state.update(SystemEvent::TransientMessage {
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
    state.update(SystemEvent::TransientMessage {
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
    state.update(SystemEvent::TransientError {
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

// ─────────────────────────────────────────────────────────────────────────────
// Contextual hints tests
// ─────────────────────────────────────────────────────────────────────────────

// Feed-focused hints: shown when feed is in focus (vim nav mode active)
#[test]
fn feed_focused_hints_show_navigation() {
    let hints = feed_focused_hints();
    assert!(
        hints.iter().any(|h| h.contains("j/k")),
        "should contain j/k: {hints:?}"
    );
    assert!(
        hints.iter().any(|h| h.contains("enter expand")),
        "should contain enter expand: {hints:?}"
    );
    assert!(
        hints.iter().any(|h| h.contains("q quit")),
        "should contain q quit: {hints:?}"
    );
}

#[test]
fn vim_nav_hints_show_scroll_and_quit() {
    let mut state = fresh_state();
    state.update(DialogEvent::DialogBack); // enter vim nav
    assert!(state.view.vim_nav_mode);
    let hint = state.hint_text();
    assert!(
        hint.contains("j/k"),
        "feed nav hint should show j/k: {hint}"
    );
    assert!(
        hint.contains("q quit"),
        "feed nav hint should show q quit: {hint}"
    );
    assert!(
        hint.contains("enter expand"),
        "feed nav hint should show enter expand: {hint}"
    );
}

// Team mode hints: shown when execution mode is Team
#[test]
fn team_mode_hints_show_subagent_hotkeys() {
    let hints = team_mode_hints();
    assert!(
        hints.iter().any(|h| h.contains("ctrl+0")),
        "should contain ctrl+0 orchestrator: {hints:?}"
    );
    assert!(
        hints.iter().any(|h| h.contains("ctrl+1..9")),
        "should contain ctrl+1..9 agents: {hints:?}"
    );
}

#[test]
fn hint_text_shows_team_mode_in_idle_state() {
    let mut state = fresh_state();
    state.config.execution_mode = ExecutionMode::Team;
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+0"),
        "hint should show ctrl+0 orchestrator in Team mode: {hint}"
    );
    assert!(
        hint.contains("ctrl+1..9"),
        "hint should show ctrl+1..9 agents in Team mode: {hint}"
    );
}

#[test]
fn hint_text_shows_team_mode_when_input_active() {
    let mut state = fresh_state();
    state.config.execution_mode = ExecutionMode::Team;
    state.input.input = "hello".to_string();
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+0"),
        "hint should show ctrl+0 when input has text in Team mode: {hint}"
    );
}

#[test]
fn hint_text_shows_team_mode_during_active_turn() {
    let mut state = fresh_state();
    state.config.execution_mode = ExecutionMode::Team;
    state.agent.turn_active = true;
    let hint = state.hint_text();
    assert!(
        hint.contains("ctrl+0"),
        "hint should show ctrl+0 during active turn in Team mode: {hint}"
    );
}

#[test]
fn hint_text_no_team_mode_in_solo() {
    let state = fresh_state();
    assert_eq!(state.config.execution_mode, ExecutionMode::Solo);
    let hint = state.hint_text();
    assert!(
        !hint.contains("ctrl+0"),
        "hint should NOT show Team hotkeys in Solo mode: {hint}"
    );
}

// Modal hints: shown when a dialog is open
#[test]
fn modal_hints_show_select_and_close() {
    let hints = modal_hints();
    assert!(
        hints.iter().any(|h| h.contains("↑/↓ select")),
        "should contain ↑/↓ select: {hints:?}"
    );
    assert!(
        hints.iter().any(|h| h.contains("enter confirm")),
        "should contain enter confirm: {hints:?}"
    );
    assert!(
        hints.iter().any(|h| h.contains("esc close")),
        "should contain esc close: {hints:?}"
    );
}

#[test]
fn hint_text_shows_modal_hints_when_dialog_open() {
    let mut state = fresh_state();
    // Open the command palette dialog directly
    let palette = Panel::new("palette", "Commands").keep_open();
    state.open_dialog = Some(DialogState::CommandPalette(PanelStack::new(palette)));
    let hint = state.hint_text();
    assert!(
        hint.contains("↑/↓ select"),
        "hint should show modal hints when dialog open: {hint}"
    );
    assert!(
        hint.contains("esc close"),
        "hint should show esc close when dialog open: {hint}"
    );
}

#[test]
fn hint_text_hides_other_hints_when_modal_open() {
    let mut state = fresh_state();
    state.config.execution_mode = ExecutionMode::Team;
    // Open the command palette dialog directly
    let palette = Panel::new("palette", "Commands").keep_open();
    state.open_dialog = Some(DialogState::CommandPalette(PanelStack::new(palette)));
    let hint = state.hint_text();
    // Modal hints take priority — team mode hotkeys are suppressed
    assert!(
        hint.contains("↑/↓ select"),
        "hint should be modal when dialog open: {hint}"
    );
    assert!(
        !hint.contains("ctrl+0"),
        "team hints should not appear when modal open: {hint}"
    );
}
