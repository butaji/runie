use crate::tui::update::chat::modal::{home_screen_close, home_screen_select};
use crate::tui::tests::reducer::make_state;

fn textarea_text(state: &crate::tui::state::AppState) -> String {
    state.textarea.lines().join("\n")
}

fn set_home_screen_selection(state: &mut crate::tui::state::AppState, index: usize) {
    state.home_screen.selected = index;
}

#[test]
fn test_new_worktree_clears_textarea() {
    let mut state = make_state();
    state.home_screen.visible = true;
    set_home_screen_selection(&mut state, 0); // "New worktree"

    state.textarea.insert_str("hello world");
    assert_eq!(textarea_text(&state), "hello world");

    home_screen_select(&mut state);

    assert_eq!(textarea_text(&state), "");
}

#[test]
fn test_resume_session_clears_textarea() {
    let mut state = make_state();
    state.home_screen.visible = true;
    set_home_screen_selection(&mut state, 1); // "Resume session"

    state.textarea.insert_str("hello world");
    assert_eq!(textarea_text(&state), "hello world");

    home_screen_select(&mut state);

    assert_eq!(textarea_text(&state), "");
}

#[test]
fn test_home_screen_close_clears_textarea() {
    let mut state = make_state();
    state.home_screen.visible = true;

    state.textarea.insert_str("hello world");
    assert_eq!(textarea_text(&state), "hello world");

    home_screen_close(&mut state);

    assert_eq!(textarea_text(&state), "");
}

#[test]
fn test_textarea_empty_after_home_screen_mode() {
    let mut state = make_state();
    state.home_screen.visible = true;
    set_home_screen_selection(&mut state, 0);

    // Input some text
    state.textarea.insert_str("some input text");
    assert!(!textarea_text(&state).is_empty());

    // Select "New worktree" - transitions to Chat mode
    home_screen_select(&mut state);

    // Verify mode changed to Chat
    assert_eq!(state.mode, crate::tui::state::TuiMode::Chat);
    // Verify textarea cleared
    assert_eq!(textarea_text(&state), "");
}

#[test]
fn test_old_input_does_not_persist_on_return_to_chat() {
    let mut state = make_state();
    state.home_screen.visible = true;
    set_home_screen_selection(&mut state, 1); // "Resume session"

    // Add text before selecting
    state.textarea.insert_str("previous input");
    assert_eq!(textarea_text(&state), "previous input");

    // Select "Resume session" - should clear textarea and enter Chat mode
    home_screen_select(&mut state);

    // Verify mode is Chat
    assert_eq!(state.mode, crate::tui::state::TuiMode::Chat);
    // Verify textarea is cleared - old input should not persist
    assert_eq!(textarea_text(&state), "");

    // Add new text - should not mix with old
    state.textarea.insert_str("new input");
    assert_eq!(textarea_text(&state), "new input");
}
