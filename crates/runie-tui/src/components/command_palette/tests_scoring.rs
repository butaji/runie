//! Tests for alias and fuzzy scoring priority

use crate::components::command_palette::CommandPalette;

fn make_fresh_palette() -> CommandPalette {
    CommandPalette::new()
}

#[test]
fn test_alias_scoring_priority() {
    // "n" should match new_session via alias with score 850
    let mut palette = make_fresh_palette();
    palette.filter("n");

    // new_session should be first due to alias match
    let first_idx = palette.filtered_commands[0];
    let first_cmd = &palette.all_commands()[first_idx];
    assert_eq!(first_cmd.id, "new_session");
    assert!(first_cmd.aliases.contains(&"n".to_string()));
}

#[test]
fn test_fuzzy_match_subsequence() {
    // "ne" should match "New Session" via fuzzy subsequence
    let mut palette = make_fresh_palette();
    palette.filter("ne");

    let has_new_session = palette.filtered_commands.iter().any(|&idx| {
        palette.all_commands()[idx].id == "new_session"
    });
    assert!(has_new_session, "'ne' should fuzzy match 'New Session'");

    // "ew" should also match (e->e, w->w in "New")
    palette.filter("ew");
    let has_new_session_ew = palette.filtered_commands.iter().any(|&idx| {
        palette.all_commands()[idx].id == "new_session"
    });
    assert!(has_new_session_ew, "'ew' should fuzzy match 'New Session'");
}

#[test]
fn test_exact_alias_score_higher_than_contains() {
    // Exact alias match (850) > contains match (600)
    let mut palette = make_fresh_palette();

    // "n" matches new_session exactly as alias
    palette.filter("n");
    let first_id = palette.all_commands()[palette.filtered_commands[0]].id.clone();

    // "c" matches clear_chat via alias contains
    palette.filter("c");
    let first_c = palette.all_commands()[palette.filtered_commands[0]].id.clone();

    assert_eq!(first_id, "new_session");
    // clear_chat has alias "c" so it should be first for "c" filter
    assert_eq!(first_c, "clear_chat");
}
