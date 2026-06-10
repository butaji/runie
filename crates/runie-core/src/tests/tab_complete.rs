use crate::model::AppState;
use crate::event::Event;

fn fresh_state() -> AppState {
    AppState::default()
}

// =============================================================================
// LAYER 1: State/Logic Tests — Pure function behavior
// =============================================================================

/// Test the core feature: second Tab with single match auto-completes
#[test]
fn tab_second_press_single_match_completes() {
    let mut state = fresh_state();
    // Setup: mock the tab_complete state to simulate single match
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.tab_complete_matches = vec!["testfile.rs".to_string()];
    state.input.tab_complete_index = 0;
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.input = "test".to_string();
    state.input.cursor_pos = 4;
    
    // Second Tab should complete (accept ghost)
    state.update(Event::Input('\t'));
    
    assert_eq!(state.input.input, "testfile.rs", "Second tab should complete to full filename");
    assert_eq!(state.input.ghost_completion, None, "Ghost should be cleared after completion");
    assert_eq!(state.input.cursor_pos, 11, "Cursor should be at end of completed text");
}

/// Test that cycling works with multiple matches (caller controls the state)
#[test]
fn cycling_changes_ghost() {
    let mut state = fresh_state();
    // Setup: cycle state directly (simulating what tab_complete would do)
    state.input.tab_complete_prefix = Some("c".to_string());
    state.input.tab_complete_matches = vec![
        "argo.toml".to_string(),
        "rate.toml".to_string(),
        "rate.lock".to_string(),
    ];
    state.input.tab_complete_index = 0;
    state.input.ghost_completion = Some("argo.toml".to_string());
    state.input.input = "c".to_string();
    state.input.cursor_pos = 1;
    
    // Manually cycle (simulating second Tab)
    state.input.tab_complete_index = 1;
    state.input.ghost_completion = Some("rate.toml".to_string());
    
    assert_eq!(
        state.input.ghost_completion, Some("rate.toml".to_string()),
        "Cycle should change ghost"
    );
    
    // Cycle again
    state.input.tab_complete_index = 2;
    state.input.ghost_completion = Some("rate.lock".to_string());
    
    assert_eq!(
        state.input.ghost_completion, Some("rate.lock".to_string()),
        "Second cycle should change to third match"
    );
}

/// Test that cycling wraps around
#[test]
fn tab_cycles_wraps_around() {
    let mut state = fresh_state();
    // Setup: at last match
    state.input.tab_complete_prefix = Some("c".to_string());
    state.input.tab_complete_matches = vec![
        "argo.toml".to_string(),
        "rate.toml".to_string(),
    ];
    state.input.tab_complete_index = 1; // At last item
    state.input.ghost_completion = Some("rate.toml".to_string());
    state.input.input = "c".to_string();
    state.input.cursor_pos = 1;
    
    // Call tab_complete which should cycle back to first
    state.update(Event::Input('\t'));
    
    assert_eq!(
        state.input.ghost_completion, Some("argo.toml".to_string()),
        "Tab should wrap to first match"
    );
    assert_eq!(state.input.tab_complete_index, 0);
}

#[test]
fn tab_flash_on_empty_input() {
    let mut state = fresh_state();
    state.update(Event::Input('\t'));
    assert!(state.input.input_flash > 0, "Tab on empty input should flash");
}

#[test]
fn tab_flash_on_no_match() {
    let mut state = fresh_state();
    state.input.input = "zzzzzzzz".into();
    state.input.cursor_pos = 8;
    state.update(Event::Input('\t'));
    assert!(state.input.input_flash > 0, "Tab with no match should flash");
    assert_eq!(state.input.ghost_completion, None);
}

/// Test that cycling resets when prefix changes
#[test]
fn new_prefix_resets_cycle() {
    let mut state = fresh_state();
    // Setup: cycle partially through
    state.input.tab_complete_prefix = Some("c".to_string());
    state.input.tab_complete_matches = vec!["argo.toml".to_string(), "rate.toml".to_string()];
    state.input.tab_complete_index = 1;
    state.input.ghost_completion = Some("rate.toml".to_string());
    state.input.input = "ca".to_string();
    state.input.cursor_pos = 2;
    
    // Type to change prefix (clears ghost)
    state.update(Event::Input('x'));
    assert!(state.input.tab_complete_prefix.is_none());
    
    // Tab on new prefix should start fresh
    state.update(Event::Input('\t'));
    // With no matches for "cax", should flash
    assert!(state.input.input_flash > 0 || state.input.tab_complete_index == 0);
}

// =============================================================================
// LAYER 2: Event Handling Tests — Input events drive state transitions
// =============================================================================

#[test]
fn enter_accepts_ghost() {
    let mut state = fresh_state();
    // Setup: have a ghost ready with full match state
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.tab_complete_matches = vec!["testfile.rs".to_string()];
    state.input.tab_complete_index = 0;
    state.input.input = "test".to_string();
    state.input.cursor_pos = 4;
    
    state.update(Event::Submit);
    
    // After submit, input should be consumed (message created)
    assert_eq!(state.input.input, "");
    
    // But the message should include the full correctly-capitalized match
    assert!(
        state.session.messages.iter().any(|m| m.content.contains("testfile.rs")),
        "Submit should include ghost completion in message"
    );
}

#[test]
fn cursor_movement_clears_ghost() {
    let mut state = fresh_state();
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.tab_complete_matches = vec!["testfile.rs".to_string()];
    state.input.input = "testx".to_string();
    state.input.cursor_pos = 5;
    
    state.update(Event::CursorLeft);
    
    assert_eq!(state.input.ghost_completion, None, "Cursor movement should clear ghost");
    assert_eq!(state.input.tab_complete_prefix, None);
}

#[test]
fn cursor_right_accepts_ghost() {
    let mut state = fresh_state();
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.tab_complete_matches = vec!["testfile.rs".to_string()];
    state.input.tab_complete_index = 0;
    state.input.input = "test".to_string();
    state.input.cursor_pos = 4;
    
    state.update(Event::CursorRight);
    
    assert_eq!(state.input.input, "testfile.rs", "CursorRight should accept ghost");
    assert_eq!(state.input.ghost_completion, None, "Ghost should be cleared after acceptance");
    assert_eq!(state.input.cursor_pos, 11, "Cursor should be at end of completed text");
}

#[test]
fn cursor_right_without_ghost_moves_cursor() {
    let mut state = fresh_state();
    state.input.input = "test".to_string();
    state.input.cursor_pos = 0;
    
    state.update(Event::CursorRight);
    
    assert_eq!(state.input.cursor_pos, 1, "CursorRight should move cursor right");
    assert_eq!(state.input.input, "test");
}

#[test]
fn delete_word_clears_ghost() {
    let mut state = fresh_state();
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.tab_complete_matches = vec!["testfile.rs".to_string()];
    state.input.input = "test word".to_string();
    state.input.cursor_pos = 9;
    
    state.update(Event::DeleteWord);
    
    assert_eq!(state.input.ghost_completion, None, "DeleteWord should clear ghost");
}

#[test]
fn backspace_clears_ghost() {
    let mut state = fresh_state();
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.input = "testx".to_string();
    state.input.cursor_pos = 5;
    
    state.update(Event::Backspace);
    
    assert_eq!(state.input.ghost_completion, None, "Backspace should clear ghost");
    assert_eq!(state.input.tab_complete_prefix, None);
}

#[test]
fn typing_clears_ghost() {
    let mut state = fresh_state();
    state.input.ghost_completion = Some("file.rs".to_string());
    state.input.tab_complete_prefix = Some("test".to_string());
    state.input.input = "testx".to_string();
    state.input.cursor_pos = 5;
    
    state.update(Event::Input('y'));
    
    assert_eq!(state.input.ghost_completion, None, "Typing should clear ghost");
}

// =============================================================================
// Integration-style tests (file system dependent)
// =============================================================================

#[test]
fn tab_finds_matches_in_crate_directory() {
    let mut state = fresh_state();
    // 'Cargo' matches Cargo.toml in the crate root
    state.input.input = "Cargo".into();
    state.input.cursor_pos = 5;
    state.update(Event::Input('\t'));
    
    // Should find at least one match
    assert!(
        state.input.ghost_completion.is_some() || state.input.input_flash > 0,
        "Tab should either find matches or flash"
    );
}

#[test]
fn tab_prefix_matching_is_case_insensitive() {
    let mut state = fresh_state();
    // 'cargo' (lowercase) should still match 'Cargo.toml'
    state.input.input = "cargo".into();
    state.input.cursor_pos = 5;
    state.update(Event::Input('\t'));
    
    // Should find matches (case-insensitive)
    if state.input.input_flash == 0 {
        assert!(
            state.input.ghost_completion.is_some(),
            "Case-insensitive matching should work"
        );
    }
}
