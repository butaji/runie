//! Tests for opt-in vim navigation mode.

use crate::event::Event;
use crate::model::{AppState, ChatMessage, Role};

fn fresh_state() -> AppState {
    AppState::default()
}

fn state_with_vim() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state
}

fn state_with_messages() -> AppState {
    let mut state = state_with_vim();
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            content: format!("msg {}", i),
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            content: format!("reply {}", i),
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    state.messages_changed();
    state
}

#[test]
fn vim_mode_off_does_not_intercept_j() {
    let mut state = fresh_state();
    state.update(Event::Input('j'));
    assert_eq!(state.input.input, "j");
}

#[test]
fn vim_mode_on_j_scrolls_up() {
    let mut state = state_with_messages();
    let before = state.view.scroll;
    state.update(Event::Input('j'));
    assert_eq!(state.input.input, "");
    assert_eq!(state.view.scroll, before + 1);
}

#[test]
fn vim_mode_on_k_scrolls_down() {
    let mut state = state_with_messages();
    state.view.scroll = 5;
    state.update(Event::Input('k'));
    assert_eq!(state.input.input, "");
    assert_eq!(state.view.scroll, 4);
}

#[test]
fn vim_mode_on_g_goes_top() {
    let mut state = state_with_messages();
    state.update(Event::Input('g'));
    assert_eq!(state.input.input, "");
    assert!(state.view.scroll > 0);
}

#[test]
#[allow(non_snake_case)]
fn vim_mode_on_G_goes_bottom() {
    let mut state = state_with_messages();
    state.view.scroll = 42;
    state.update(Event::Input('G'));
    assert_eq!(state.input.input, "");
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn vim_mode_on_slash_opens_palette() {
    let mut state = state_with_vim();
    state.update(Event::Input('/'));
    assert_eq!(state.input.input, "");
    assert!(state.open_dialog.is_some(), "palette should open");
}

#[test]
fn vim_mode_on_y_copies_last_response() {
    let mut state = state_with_vim();
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        content: "last answer".into(),
        timestamp: 0.0,
        id: "resp.0".into(),
        ..Default::default()
    });
    state.messages_changed();
    state.update(Event::Input('y'));
    assert_eq!(state.input.input, "");
}

#[test]
fn vim_mode_printable_char_starts_input() {
    let mut state = state_with_vim();
    state.update(Event::Input('a'));
    assert_eq!(state.input.input, "a");
}

#[test]
fn vim_mode_ignored_when_input_not_empty() {
    let mut state = state_with_vim();
    state.update(Event::Input('a'));
    state.update(Event::Input('j'));
    assert_eq!(state.input.input, "aj");
}

#[test]
fn go_to_top_event_sets_scroll() {
    let mut state = state_with_messages();
    state.update(Event::GoToTop);
    assert!(state.view.scroll > 0);
}

#[test]
fn go_to_bottom_event_clears_scroll() {
    let mut state = state_with_messages();
    state.view.scroll = 42;
    state.update(Event::GoToBottom);
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn toggle_vim_mode_flips_flag() {
    let mut state = fresh_state();
    assert!(!state.config.vim_mode);
    state.update(Event::ToggleVimMode);
    assert!(state.config.vim_mode);
    state.update(Event::ToggleVimMode);
    assert!(!state.config.vim_mode);
}

#[test]
fn hint_text_shows_esc_nav_when_vim_enabled_and_idle() {
    let state = state_with_vim();
    // When idle and empty, the hint advertises Esc to enter nav mode.
    assert!(state.hint_text().contains("esc nav"));
}

#[test]
fn hint_text_does_not_show_vim_scroll_when_disabled() {
    let state = fresh_state();
    assert!(!state.hint_text().contains("j/k scroll"));
}

// =========================================================================
// Vim nav mode: Esc enters feed nav, Space returns to input box.
// =========================================================================

#[test]
fn esc_with_vim_mode_on_and_no_turn_enters_nav_mode() {
    let mut state = state_with_vim();
    assert!(!state.vim_nav_mode);
    state.update(Event::DialogBack); // Esc
    assert!(state.vim_nav_mode, "Esc should enter vim nav mode");
}

#[test]
fn esc_with_vim_mode_off_does_not_enter_nav_mode() {
    let mut state = fresh_state();
    state.update(Event::DialogBack);
    assert!(!state.vim_nav_mode, "Esc must not enter nav mode when vim_mode is off");
}

#[test]
fn esc_during_active_turn_stops_first_then_enters_nav() {
    let mut state = state_with_vim();
    state.agent.turn_active = true;
    state.update(Event::DialogBack);
    // First Esc stops the turn; user is NOT yet in nav mode.
    assert!(!state.agent.turn_active, "first Esc should stop the turn");
    assert!(!state.vim_nav_mode, "first Esc should not yet enter nav");
    assert!(state.vim_nav_pending, "first Esc should arm the nav-on-next-esc");
    state.update(Event::DialogBack);
    // Second Esc enters nav mode.
    assert!(state.vim_nav_mode, "second Esc should enter nav mode");
    assert!(!state.vim_nav_pending, "pending should be consumed");
}

#[test]
fn nav_mode_jk_gg_scroll() {
    let mut state = state_with_vim();
    state.update(Event::DialogBack); // enter nav
    assert!(state.vim_nav_mode);

    let before = state.view.scroll;
    state.update(Event::Input('j'));
    assert_eq!(state.view.scroll, before + 1);
    state.update(Event::Input('k'));
    assert_eq!(state.view.scroll, before);
    state.update(Event::Input('G'));
    assert_eq!(state.view.scroll, 0);
    state.update(Event::Input('g'));
    assert!(state.view.scroll > 0);
}

#[test]
fn space_exits_nav_mode_and_inserts_space() {
    let mut state = state_with_vim();
    state.update(Event::DialogBack); // enter nav
    state.update(Event::Input(' '));
    assert!(!state.vim_nav_mode, "Space should exit nav mode");
    assert_eq!(state.input.input, " ", "Space should insert a space");
}

#[test]
fn typing_printable_char_exits_nav_and_inserts() {
    let mut state = state_with_vim();
    state.update(Event::DialogBack); // enter nav
    state.update(Event::Input('a'));
    assert!(!state.vim_nav_mode, "typing should exit nav mode");
    assert_eq!(state.input.input, "a");
}

#[test]
fn nav_mode_off_esc_is_noop() {
    // Without vim_mode, Esc should not change nav mode.
    let mut state = fresh_state();
    state.update(Event::DialogBack);
    assert!(!state.vim_nav_mode);
    assert_eq!(state.input.input, "");
}

#[test]
fn hint_text_in_nav_mode_shows_nav_hotkeys() {
    let mut state = state_with_vim();
    state.update(Event::DialogBack); // enter nav
    let hint = state.hint_text();
    assert!(hint.contains("j/k scroll"), "hint missing j/k: {hint}");
    assert!(hint.contains("space input"), "hint missing space hint: {hint}");
    assert!(
        hint.contains("esc input"),
        "hint should advertise Esc to return: {hint}"
    );
}

#[test]
fn hint_text_idle_no_turn_mentions_esc_nav() {
    let state = state_with_vim();
    let hint = state.hint_text();
    assert!(
        hint.contains("esc nav"),
        "idle vim-mode hint should advertise esc nav: {hint}"
    );
}

#[test]
fn hint_text_without_vim_mode_does_not_mention_nav() {
    let state = fresh_state();
    let hint = state.hint_text();
    assert!(!hint.contains("esc nav"));
    assert!(!hint.contains("space input"));
}
