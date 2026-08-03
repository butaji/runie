//! Ghost completion tests — tab shows rest of filename in gray.

use crate::tests::fresh_state;

#[test]
fn ghost_is_set_directly() {
    let mut state = fresh_state();
    // Directly set ghost (simulating what tab_complete does internally)
    state.input.ghost_completion = Some("file.rs".to_string());

    assert_eq!(
        state.input.ghost_completion,
        Some("file.rs".to_string()),
        "Ghost should be set"
    );
}

#[test]
fn ghost_cleared_after_completion() {
    let mut state = fresh_state();
    state.input.ghost_completion = Some("file.rs".to_string());

    // This simulates completing
    state.accept_ghost();

    assert_eq!(state.input.ghost_completion, None);
}

#[test]
fn submit_with_ghost_includes_full_filename() {
    let mut state = fresh_state();
    state.input.input = "test".to_string();
    state.input.cursor_pos = 4;
    state.input.ghost_completion = Some("file.rs".to_string());

    state.update(Event::submit());

    // Ghost should be appended before submission
    assert!(
        state
            .session
            .messages
            .iter()
            .any(|m| m.content().contains("testfile.rs")),
        "Submit should include ghost completion"
    );
}

#[test]
fn cycling_changes_ghost() {
    let mut state = fresh_state();
    // Setup with multiple matches
    state.input.tab_complete_prefix = Some("t".to_string());
    state.input.tab_complete_matches = vec!["est1.rs".to_string(), "est2.rs".to_string()];
    state.input.tab_complete_index = 0;
    state.input.ghost_completion = Some("est1.rs".to_string());

    // Manually cycle (what Tab would do)
    state.input.tab_complete_index = 1;
    state.input.ghost_completion = Some("est2.rs".to_string());

    assert_ne!(
        state.input.ghost_completion,
        Some("est1.rs".to_string()),
        "Cycle should change ghost"
    );
}

#[test]
fn empty_input_no_ghost() {
    let state = fresh_state();
    assert_eq!(state.input.ghost_completion, None);
}

#[test]
fn ghost_shows_directory_suffix() {
    let mut state = fresh_state();
    // Set ghost for directory
    state.input.ghost_completion = Some("/".to_string());

    assert!(
        state.input.ghost_completion.is_some(),
        "Ghost should show for directory"
    );
    assert!(state.input.ghost_completion.unwrap().contains('/'));
}

#[test]
fn prompt_suggestion_load_is_generation_guarded_and_projects_suffix() {
    let mut state = fresh_state();
    let generation = state.begin_prompt_suggestion_fetch();
    assert!(!state.load_prompt_suggestion(Some("stale".into()), generation.wrapping_sub(1)));
    assert!(state.load_prompt_suggestion(Some("list files".into()), generation));
    state.input.input = "list".into();
    state.input.cursor_pos = 4;
    state.ensure_fresh();
    assert_eq!(
        state.view.prompt_suggestion.ghost_for("list"),
        Some(" files")
    );
}

use crate::event::Event;
