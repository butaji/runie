//! Input handling tests (keyboard events, paste, shortcuts).
//!
//! Tests event_to_msg and key_to_msg functions for:
//! - Paste bypassing blocking modes
//! - Ctrl+C quit/clear behavior
//! - Esc key handling in all modes
//! - Ctrl+P/B/T shortcuts

use crate::tui::state::{AppState, CommandPaletteState, Msg, TuiMode};
use crate::components::CommandPalette;
use crate::tui::update::update;
use crate::tui::events::{event_to_msg, key_to_msg};
use ratatui_textarea::{TextArea, Input, Key};
use crossterm::event::{Event, KeyCode, KeyModifiers, KeyEvent, KeyEventKind, KeyEventState};

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
        top_bar: Default::default(),
        permission_modal: Default::default(),
        command_palette: CommandPaletteState::default(),
        scroll: Default::default(),
        animation: Default::default(),
        diff_viewer: None,
        token_usage: Default::default(),
        session_token_usage: Default::default(),
        session_tree: Default::default(),
        background_jobs: Vec::new(),
        onboarding: None,
        terminal_size: (0, 0),
        clear_input_confirm: Default::default(),
        model_picker: None,
    }
}

fn make_state_with_text(text: &str) -> AppState {
    let mut state = make_state();
    state.textarea = TextArea::new(vec![text.to_string()]);
    state
}

fn type_char(state: &mut AppState, c: char) {
    state.textarea.input(Input { key: Key::Char(c), ctrl: false, alt: false, shift: false });
}

fn make_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

// BUG-03: Paste in Permission modal bypasses blocking mode and goes to textarea
#[test]
fn test_paste_bypasses_blocking_mode() {
    // event_to_msg handles Paste separately from key events, bypassing blocking_mode_handler
    let mut state = make_state();
    state.mode = TuiMode::Permission; // Blocking mode

    let msgs = event_to_msg(Event::Paste("hello".to_string()), &state);

    assert_eq!(msgs.len(), 1);
    assert!(matches!(&msgs[0], Msg::Paste(p) if p == "hello"));
}

// test_ctrl_c_empty_textarea_quits — Ctrl+C with empty lines → Msg::Quit
#[test]
fn test_ctrl_c_empty_textarea_quits() {
    let state = make_state(); // Empty textarea

    let key = make_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::Quit)));
}

// test_ctrl_c_non_empty_shows_hint — Ctrl+C with text → "Ctrl+C again to clear"
#[test]
fn test_ctrl_c_non_empty_shows_hint() {
    let mut state = make_state();
    type_char(&mut state, 'h');
    type_char(&mut state, 'i');

    let key = make_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::ClearInputConfirm)));

    // First tap shows hint
    update(&mut state, &mut CommandPalette::new(), Msg::ClearInputConfirm);
    assert!(state.input_right_info.contains("Ctrl+C again"));
}

// test_double_tap_clear_within_2s — Two Ctrl+C within 2s clears input
#[test]
fn test_double_tap_clear_within_2s() {
    let mut state = make_state();
    type_char(&mut state, 'h');
    type_char(&mut state, 'i');

    let key = make_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let msgs = key_to_msg(key, &state);
    assert!(matches!(msgs, Some(Msg::ClearInputConfirm)));

    // First tap
    update(&mut state, &mut CommandPalette::new(), Msg::ClearInputConfirm);
    assert!(!state.textarea.is_empty());

    // Second tap within 2s clears
    update(&mut state, &mut CommandPalette::new(), Msg::ClearInputConfirm);
    assert!(state.textarea.is_empty());
}

// test_double_tap_timeout_resets — Second tap after 2s → shows hint again
#[test]
fn test_double_tap_timeout_resets() {
    let mut state = make_state();
    type_char(&mut state, 'h');
    type_char(&mut state, 'i');

    // First tap
    update(&mut state, &mut CommandPalette::new(), Msg::ClearInputConfirm);

    // Simulate timeout: last_press was 3 seconds ago
    state.clear_input_confirm.last_press = Some(
        std::time::Instant::now() - std::time::Duration::from_secs(3)
    );

    // Next tap is treated as first tap (timeout reset)
    update(&mut state, &mut CommandPalette::new(), Msg::ClearInputConfirm);
    assert!(!state.textarea.is_empty());
    assert!(state.input_right_info.contains("Ctrl+C again"));
}

// test_esc_closes_palette — Esc in CommandPalette → CommandPaletteCancelArgument
#[test]
fn test_esc_closes_palette() {
    let mut state = make_state();
    state.mode = TuiMode::CommandPalette;

    let key = make_key(KeyCode::Esc, KeyModifiers::NONE);
    let msgs = key_to_msg(key, &state);

    // Esc in palette sends CancelArgument (closes if not in argument mode)
    assert!(matches!(msgs, Some(Msg::CommandPaletteCancelArgument)));
}

// test_esc_cancels_permission — Esc in Permission → PermissionCancel
#[test]
fn test_esc_cancels_permission() {
    let mut state = make_state();
    state.mode = TuiMode::Permission;

    let key = make_key(KeyCode::Esc, KeyModifiers::NONE);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::PermissionCancel)));
}

// test_esc_closes_diff_viewer — Esc in DiffViewer → CloseModal
#[test]
fn test_esc_closes_diff_viewer() {
    let mut state = make_state();
    state.mode = TuiMode::DiffViewer;

    let key = make_key(KeyCode::Esc, KeyModifiers::NONE);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::CloseModal)));
}

// test_esc_closes_session_tree — Esc in SessionTree → CloseModal
#[test]
fn test_esc_closes_session_tree() {
    let mut state = make_state();
    state.mode = TuiMode::SessionTree;

    let key = make_key(KeyCode::Esc, KeyModifiers::NONE);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::CloseModal)));
}

// test_ctrl_p_opens_palette — Ctrl+P in Chat → OpenCommandPalette
#[test]
fn test_ctrl_p_opens_palette() {
    let state = make_state();

    let key = make_key(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::OpenCommandPalette)));
}

// test_ctrl_b_toggles_sidebar — Ctrl+B → ToggleSidebar
#[test]
fn test_ctrl_b_toggles_sidebar() {
    let state = make_state();

    let key = make_key(KeyCode::Char('b'), KeyModifiers::CONTROL);
    let msgs = key_to_msg(key, &state);

    assert!(matches!(msgs, Some(Msg::ToggleSidebar)));
}

// test_ctrl_t_toggles_session_tree — Ctrl+T → TextareaKey (not mapped to ToggleSessionTree)
#[test]
fn test_ctrl_t_toggles_session_tree() {
    let state = make_state();

    let key = make_key(KeyCode::Char('t'), KeyModifiers::CONTROL);
    let msgs = key_to_msg(key, &state);

    // Ctrl+T is passed to textarea, not mapped to ToggleSessionTree
    assert!(matches!(msgs, Some(Msg::TextareaKey(_))));
}
