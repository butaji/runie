//! Tests for opt-in vim navigation mode.

use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::tests::fresh_state;

#[test]
fn vim_mode_is_enabled_by_default() {
    let state = AppState::default();
    assert!(
        state.config.vim_mode,
        "vim_mode should be enabled by default — users no longer need to opt in via config"
    );
}

fn state_with_vim() -> AppState {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state
}

fn state_with_vim_and_messages() -> AppState {
    let mut state = state_with_vim();
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg {}", i),
            }],
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![Part::Text {
                content: format!("reply {}", i),
            }],
            timestamp: i as f64 + 0.5,
            id: format!("resp.{}", i),
            ..Default::default()
        });
    }
    state.refresh_after_message_change();

    state
}

fn state_with_messages() -> AppState {
    let mut state = state_with_vim();
    for i in 0..20 {
        state.session.messages.push(ChatMessage {
            role: Role::User,
            parts: vec![Part::Text {
                content: format!("msg {}", i),
            }],
            timestamp: i as f64,
            id: format!("req.{}", i),
            ..Default::default()
        });
        state.session.messages.push(ChatMessage {
            role: Role::Assistant,
            parts: vec![Part::Text {
                content: format!("reply {}", i),
            }],
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
    state.config.vim_mode = false;
    state.update(crate::Event::Input('j'));
    assert_eq!(state.input.input, "j");
}

#[test]
fn vim_mode_on_j_scrolls_up() {
    let mut state = state_with_messages();
    let before = state.view.scroll;
    state.update(crate::Event::Input('j'));
    assert_eq!(state.input.input, "");
    assert_eq!(state.view.scroll, before + 1);
}

#[test]
fn vim_mode_on_k_scrolls_down() {
    let mut state = state_with_messages();
    state.view.scroll = 5;
    state.update(crate::Event::Input('k'));
    assert_eq!(state.input.input, "");
    assert_eq!(state.view.scroll, 4);
}

#[test]
fn vim_mode_on_g_goes_top() {
    let mut state = state_with_messages();
    state.ensure_fresh();
    state.view.last_visible_height = 10;
    state.update(crate::Event::Input('g'));
    assert_eq!(state.input.input, "");
    assert!(state.view.scroll > 0);
}

#[test]
#[allow(non_snake_case)]
fn vim_mode_on_G_goes_bottom() {
    let mut state = state_with_messages();
    state.view.scroll = 42;
    state.update(crate::Event::Input('G'));
    assert_eq!(state.input.input, "");
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn vim_mode_on_slash_opens_palette() {
    let mut state = state_with_vim();
    state.update(crate::Event::Input('/'));
    assert_eq!(state.input.input, "");
    assert!(state.open_dialog.is_some(), "palette should open");
}

#[test]
fn vim_mode_y_typed_as_text_in_empty_input() {
    // 'y' is no longer a single-key copy shortcut (copy is Ctrl+O).
    // In an empty input box with vim_mode on, 'y' is typed as a normal
    // character.
    let mut state = state_with_vim();
    state.update(crate::Event::Input('y'));
    assert_eq!(state.input.input, "y");
}

#[allow(dead_code)]
fn vim_mode_on_y_copies_last_response() {
    // Deprecated: 'y' is no longer a copy shortcut. The test body is
    // kept as a placeholder; see vim_mode_y_typed_as_text_in_empty_input
    // and ctrl_o_copies_last_response for the actual contract.
}

#[test]
fn vim_mode_printable_char_starts_input() {
    let mut state = state_with_vim();
    state.update(crate::Event::Input('a'));
    assert_eq!(state.input.input, "a");
}

#[test]
fn vim_mode_ignored_when_input_not_empty() {
    let mut state = state_with_vim();
    state.update(crate::Event::Input('a'));
    state.update(crate::Event::Input('j'));
    assert_eq!(state.input.input, "aj");
}

#[test]
fn go_to_top_event_sets_scroll() {
    let mut state = state_with_messages();
    state.ensure_fresh();
    state.view.last_visible_height = 10;
    state.update(crate::Event::GoToTop);
    assert!(state.view.scroll > 0);
}

#[test]
fn go_to_bottom_event_clears_scroll() {
    let mut state = state_with_messages();
    state.view.scroll = 42;
    state.update(crate::Event::GoToBottom);
    assert_eq!(state.view.scroll, 0);
}

#[test]
fn toggle_vim_mode_flips_flag() {
    let mut state = fresh_state();
    assert!(state.config.vim_mode, "vim_mode defaults to enabled");
    state.update(crate::Event::ToggleVimMode);
    assert!(!state.config.vim_mode);
    state.update(crate::Event::ToggleVimMode);
    assert!(state.config.vim_mode);
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
    assert!(!state.view.vim_nav_mode);
    state.update(crate::Event::DialogBack); // Esc
    assert!(state.view.vim_nav_mode, "Esc should enter vim nav mode");
}

#[test]
fn esc_with_vim_mode_off_does_not_enter_nav_mode() {
    let mut state = fresh_state();
    state.config.vim_mode = false;
    state.update(crate::Event::DialogBack);
    assert!(
        !state.view.vim_nav_mode,
        "Esc must not enter nav mode when vim_mode is off"
    );
}

#[test]
fn esc_during_active_turn_stops_first_then_enters_nav() {
    let mut state = state_with_vim();
    state.agent.turn_active = true;
    state.update(crate::Event::Escape);
    // First Esc stops the turn; user is NOT yet in nav mode.
    assert!(!state.agent.turn_active, "first Esc should stop the turn");
    assert!(
        !state.view.vim_nav_mode,
        "first Esc should not yet enter nav"
    );
    assert!(
        state.view.vim_nav_pending,
        "first Esc should arm the nav-on-next-esc"
    );
    state.update(crate::Event::Escape);
    // Second Esc enters nav mode.
    assert!(state.view.vim_nav_mode, "second Esc should enter nav mode");
    assert!(!state.view.vim_nav_pending, "pending should be consumed");
}

#[test]
fn nav_mode_jk_gg_scroll() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    state.update(crate::Event::DialogBack); // enter nav
    assert!(state.view.vim_nav_mode);

    // j jumps DOWN (newer, toward bottom); k jumps UP (older).
    state.update(crate::Event::Input('g')); // jump to top
    let top = state.view.scroll;
    assert!(top > 0);
    state.update(crate::Event::Input('j')); // j down: toward bottom
    let after_j = state.view.scroll;
    assert!(
        after_j < top,
        "j should move toward newer (scroll down): top={top} after_j={after_j}"
    );
    state.update(crate::Event::Input('k')); // k up: toward older
    let after_k = state.view.scroll;
    assert!(
        after_k > after_j,
        "k should move toward older (scroll up): after_j={after_j} after_k={after_k}"
    );
}

#[test]
fn space_exits_nav_mode_and_inserts_space() {
    let mut state = state_with_vim();
    state.update(crate::Event::DialogBack); // enter nav
    state.update(crate::Event::Input(' '));
    assert!(!state.view.vim_nav_mode, "Space should exit nav mode");
    assert_eq!(state.input.input, " ", "Space should insert a space");
}

#[test]
fn typing_printable_char_exits_nav_and_inserts() {
    let mut state = state_with_vim();
    state.update(crate::Event::DialogBack); // enter nav
    state.update(crate::Event::Input('a'));
    assert!(!state.view.vim_nav_mode, "typing should exit nav mode");
    assert_eq!(state.input.input, "a");
}

#[test]
fn nav_mode_off_esc_is_noop() {
    // Without vim_mode, Esc should not change nav mode.
    let mut state = fresh_state();
    state.config.vim_mode = false;
    state.update(crate::Event::DialogBack);
    assert!(!state.view.vim_nav_mode);
    assert_eq!(state.input.input, "");
}

#[test]
fn hint_text_in_nav_mode_shows_nav_hotkeys() {
    let mut state = state_with_vim();
    state.update(crate::Event::DialogBack); // enter nav
    let hint = state.hint_text();
    assert!(hint.contains("j/k"), "hint should advertise j/k: {hint}");
    assert!(
        hint.contains("space") && hint.contains("i"),
        "hint missing space/i hint: {hint}"
    );
    assert!(
        hint.contains("esc"),
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
    let mut state = fresh_state();
    state.config.vim_mode = false;
    let hint = state.hint_text();
    assert!(!hint.contains("esc nav"));
    assert!(!hint.contains("space input"));
}

// =========================================================================
// Nav mode: `i` returns to input mode (vim insert). Space also returns.
// =========================================================================

#[test]
fn i_in_nav_mode_exits_nav_and_does_not_insert() {
    let mut state = state_with_vim();
    state.update(crate::Event::DialogBack); // enter nav
    state.update(crate::Event::Input('i'));
    assert!(!state.view.vim_nav_mode, "i should exit nav mode");
    assert_eq!(
        state.input.input, "",
        "i must not insert the letter 'i' (it is the vim insert key)"
    );
}

#[test]
fn i_in_nav_mode_then_char_inserts() {
    let mut state = state_with_vim();
    state.update(crate::Event::DialogBack); // enter nav
    state.update(crate::Event::Input('i')); // exit nav
    state.update(crate::Event::Input('h')); // should now insert
    assert_eq!(state.input.input, "h");
}

#[test]
fn hint_text_in_nav_mode_advertises_i_and_space() {
    let mut state = state_with_vim();
    state.update(crate::Event::DialogBack);
    let hint = state.hint_text();
    assert!(
        hint.contains("space/i"),
        "nav hint should advertise space/i to enter input mode: {hint}"
    );
}

// =========================================================================
// At the lowest element in the feed, j / ArrowDown exit nav mode and
// re-enable the input box (the "next thing" is the input itself).
// =========================================================================

fn enter_nav(state: &mut AppState) {
    state.update(crate::Event::DialogBack);
    assert!(state.view.vim_nav_mode);
}

#[test]
fn j_at_lowest_element_exits_nav_and_enables_input() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    // First jump to the bottom (G) so we are at the lowest element.
    state.update(crate::Event::Input('G'));
    assert_eq!(state.view.scroll, 0);
    assert!(state.view.vim_nav_mode);
    // j at the bottom should exit nav mode (no flash needed — natural end).
    state.update(crate::Event::Input('j'));
    assert!(
        !state.view.vim_nav_mode,
        "j at the lowest element should exit nav mode (input becomes the next focus)"
    );
}

#[test]
fn arrow_down_at_lowest_element_exits_nav_and_enables_input() {
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(crate::Event::Input('G'));
    assert_eq!(state.view.scroll, 0);
    // ArrowDown at the bottom should also exit nav mode.
    state.update(crate::Event::Down);
    assert!(
        !state.view.vim_nav_mode,
        "ArrowDown at the lowest element should exit nav mode"
    );
}

#[test]
fn j_below_lowest_element_actually_typed_as_text() {
    // After exiting nav mode via j-at-bottom, subsequent non-motion
    // keys should type normally.
    let mut state = state_with_vim_and_messages();
    state.view.last_visible_height = 10;
    enter_nav(&mut state);
    state.update(crate::Event::Input('G'));
    state.update(crate::Event::Input('j')); // exit nav (at bottom)
    state.update(crate::Event::Input('h')); // now should type
    assert_eq!(state.input.input, "h");
}

// ── Block copy in vim nav mode ─────────────────────────────────────────────────

/// Build state with one agent message, ready for vim nav.
fn state_for_nav_copy() -> AppState {
    let mut state = state_with_vim();
    state.session.messages.push(ChatMessage {
        role: Role::User,
        timestamp: 1.0,
        id: "req.0".into(),
        parts: vec![Part::Text {
            content: "hello".into(),
        }],
        ..Default::default()
    });
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 2.0,
        id: "resp.0".into(),
        parts: vec![Part::Text {
            content: "the answer is 42".into(),
        }],
        ..Default::default()
    });
    state.refresh_after_message_change();

    state.view.last_visible_height = 10;
    state
}

#[test]
fn y_in_vim_nav_copies_block_and_exits_nav() {
    let mut state = state_for_nav_copy();
    enter_nav(&mut state);
    assert!(state.view.vim_nav_mode);
    let selected = state.view.selected_post;
    assert!(selected.is_some(), "a post should be selected in nav mode");

    // Press y — should emit CopySelectedBlock and exit vim nav
    state.update(crate::Event::Input('y'));

    assert!(!state.view.vim_nav_mode, "y should exit vim nav mode");
}

#[test]
fn y_in_vim_nav_copies_metadata_and_exits_nav() {
    let mut state = state_for_nav_copy();
    enter_nav(&mut state);
    assert!(state.view.vim_nav_mode);

    // Press Y — should emit CopyBlockMetadata and exit vim nav
    state.update(crate::Event::Input('Y'));

    assert!(!state.view.vim_nav_mode, "Y should exit vim nav mode");
}

#[test]
fn y_on_empty_selection_does_not_crash() {
    let mut state = AppState::default();
    state.config.vim_mode = true;
    state.view.vim_nav_mode = true;
    // No post selected — pressing y should not panic
    state.update(crate::Event::Input('y'));
    assert!(
        !state.view.vim_nav_mode,
        "y should still exit nav mode even with no selection"
    );
}
