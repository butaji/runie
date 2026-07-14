//! Chip (atomic region) behavior for pasted blocks and picked @-mentions.
//!
//! Grok parity (see GROK.md §23 in runie-tests):
//! - Pasting more than 3 lines collapses to a `[Pasted: N lines]` chip.
//! - A chip is atomic: one Backspace at its end deletes the whole chip.
//! - A picked @-mention is atomic too: BS1 removes the trailing space,
//!   BS2 removes the whole `@path` in one press.
//! - Editing *inside* a chip dissolves it (falls back to char-wise edits).

use crate::model::AppState;

fn paste(state: &mut AppState, text: &str) {
    state.update(crate::Event::Paste(text.to_string()));
}

// ── Paste chips ───────────────────────────────────────────────────────────

#[test]
fn paste_three_lines_or_fewer_stays_chip_free() {
    let mut state = AppState::default();
    paste(&mut state, "a\nb\nc");
    assert_eq!(state.input.input, "a\nb\nc");
    assert!(
        state.input.chips.is_empty(),
        "3 lines must render inline, no chip"
    );
}

#[test]
fn paste_more_than_three_lines_creates_labeled_chip() {
    let mut state = AppState::default();
    paste(&mut state, "l1\nl2\nl3\nl4");
    assert_eq!(
        state.input.input, "l1\nl2\nl3\nl4",
        "full pasted content must stay in the buffer"
    );
    assert_eq!(state.input.chips.len(), 1, ">3 lines must create one chip");
    let chip = &state.input.chips[0];
    assert_eq!(chip.start, 0);
    assert_eq!(chip.end, "l1\nl2\nl3\nl4".len());
    assert_eq!(chip.label.as_deref(), Some("[Pasted: 4 lines]"));
}

#[test]
fn chip_display_view_substitutes_label_and_maps_cursor() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('x'));
    paste(&mut state, "a\nb\nc\nd");
    let (display, cursor) = state.input.display_view();
    assert_eq!(
        display, "x[Pasted: 4 lines]",
        "labeled chip must render its label, not the buffer text"
    );
    assert_eq!(
        cursor,
        "x[Pasted: 4 lines]".len(),
        "cursor after the chip must map to the end of the label"
    );
}

#[test]
fn display_view_maps_cursor_before_chip_unchanged() {
    let mut state = AppState::default();
    state.update(crate::Event::Input('x'));
    state.update(crate::Event::Input('y'));
    paste(&mut state, "a\nb\nc\nd");
    // Move cursor back before the chip: "xy|…"
    state.update(crate::Event::CursorStart);
    state.update(crate::Event::CursorRight);
    let (display, cursor) = state.input.display_view();
    assert_eq!(display, "xy[Pasted: 4 lines]");
    assert_eq!(cursor, 1, "cursor before the chip is unaffected");
}

#[test]
fn backspace_at_chip_end_deletes_whole_chip() {
    let mut state = AppState::default();
    paste(&mut state, "l1\nl2\nl3\nl4");
    state.update(crate::Event::Backspace);
    assert_eq!(
        state.input.input, "",
        "one Backspace at the chip end must delete the whole chip"
    );
    assert!(state.input.chips.is_empty());
    assert_eq!(state.input.cursor_pos, 0);
}

#[test]
fn backspace_inside_chip_dissolves_it() {
    let mut state = AppState::default();
    paste(&mut state, "l1\nl2\nl3\nl4");
    // Cursor into the middle of the chip, then Backspace: char-wise delete
    // and the chip loses its atomicity.
    state.update(crate::Event::CursorStart);
    state.update(crate::Event::CursorRight);
    state.update(crate::Event::CursorRight); // cursor after "l1"
    state.update(crate::Event::Backspace); // deletes '1'
    assert_eq!(state.input.input, "l\nl2\nl3\nl4");
    assert!(
        state.input.chips.is_empty(),
        "editing inside a chip must dissolve it"
    );
}

#[test]
fn typing_before_chip_shifts_it() {
    let mut state = AppState::default();
    paste(&mut state, "a\nb\nc\nd");
    state.update(crate::Event::CursorStart);
    state.update(crate::Event::Input('z'));
    assert_eq!(state.input.input, "za\nb\nc\nd");
    assert_eq!(state.input.chips.len(), 1);
    assert_eq!(state.input.chips[0].start, 1, "chip must shift right");
    assert_eq!(state.input.chips[0].end, "za\nb\nc\nd".len());
}

#[test]
fn submit_clears_chips() {
    let mut state = AppState::default();
    paste(&mut state, "a\nb\nc\nd");
    state.update(crate::Event::Submit);
    assert!(state.input.chips.is_empty());
}

/// The InputActor's `InputChanged` echo wholesale-replaces the projection
/// `InputState`; while the file picker is open (backup present), chips (like
/// the backup itself) must survive it, or the Clear echo sent when the
/// picker opens would silently drop them.
#[test]
fn input_changed_preserves_chips_while_picker_open() {
    let mut state = AppState::default();
    paste(&mut state, "a\nb\nc\nd");
    assert_eq!(state.input.chips.len(), 1);
    state.input.file_picker_backup = Some(("read @".to_string(), 6, 6, false));
    state.update(crate::Event::InputChanged {
        state: Box::new(crate::model::InputState::default()),
    });
    assert_eq!(
        state.input.chips.len(),
        1,
        "chips must survive the InputActor echo while the picker is open"
    );
}

/// Outside the picker window the actor is authoritative: chips echoed by the
/// InputActor (e.g. a fresh paste chip) must replace the projection's.
#[test]
fn input_changed_takes_actor_chips_outside_picker() {
    let mut state = AppState::default();
    assert!(state.input.chips.is_empty());
    let mut echoed = crate::model::InputState::default();
    echoed.input = "l1\nl2\nl3\nl4".to_string();
    echoed.cursor_pos = echoed.input.len();
    echoed.chips = vec![crate::model::InputChip {
        start: 0,
        end: echoed.input.len(),
        label: Some("[Pasted: 4 lines]".to_string()),
    }];
    state.update(crate::Event::InputChanged {
        state: Box::new(echoed),
    });
    assert_eq!(state.input.chips.len(), 1);
    assert_eq!(
        state.input.chips[0].label.as_deref(),
        Some("[Pasted: 4 lines]")
    );
}

// ── Mention chips (@ picker) ──────────────────────────────────────────────

fn mock_file_entry(name: &str) -> crate::model::FffFileEntry {
    crate::model::FffFileEntry {
        name: name.to_owned(),
        path: name.to_owned(),
        is_dir: false,
        score: 1.0,
        git_status: None,
    }
}

fn pick_first_file(state: &mut AppState) {
    state.fff_file_results = vec![mock_file_entry("README.md")];
    for c in "read @".chars() {
        state.update(crate::Event::Input(c));
    }
    assert!(state.open_dialog.is_some(), "@ must open the picker");
    state.update(crate::Event::Submit);
    assert!(state.open_dialog.is_none(), "pick must close the picker");
}

#[test]
fn picked_mention_gets_trailing_space_and_chip() {
    let mut state = AppState::default();
    pick_first_file(&mut state);
    assert!(
        state.input.input.starts_with("read @"),
        "pick must preserve the typed prefix, got {:?}",
        state.input.input
    );
    assert!(
        state.input.input.ends_with(' '),
        "pick must append a trailing space, got {:?}",
        state.input.input
    );
    assert_eq!(
        state.input.cursor_pos,
        state.input.input.len(),
        "cursor must land after the trailing space"
    );
    assert_eq!(
        state.input.chips.len(),
        1,
        "pick must record a mention chip"
    );
    let chip = &state.input.chips[0];
    assert_eq!(
        chip.label, None,
        "mention chips render the buffer text (no label)"
    );
    let mention = &state.input.input[chip.start..chip.end];
    assert!(
        mention.starts_with('@') && !mention.ends_with(' '),
        "chip must cover exactly the @mention, got {:?}",
        mention
    );
}

#[test]
fn picked_mention_backspace_is_atomic() {
    let mut state = AppState::default();
    pick_first_file(&mut state);
    let picked = state.input.input.clone();

    // BS1: only the trailing space goes.
    state.update(crate::Event::Backspace);
    assert_eq!(
        state.input.input,
        picked.trim_end().to_string(),
        "BS1 must delete just the trailing space"
    );
    assert_eq!(state.input.chips.len(), 1, "mention chip survives BS1");

    // BS2: the whole @mention goes in one press.
    state.update(crate::Event::Backspace);
    assert_eq!(
        state.input.input, "read ",
        "BS2 must delete the whole @mention atomically"
    );
    assert!(state.input.chips.is_empty());
}

#[test]
fn two_picked_mentions_have_independent_chips() {
    let mut state = AppState::default();
    pick_first_file(&mut state);
    // Trigger the picker again after the first mention.
    for c in "and @".chars() {
        state.update(crate::Event::Input(c));
    }
    assert!(
        state.open_dialog.is_some(),
        "second @ must reopen the picker"
    );
    state.update(crate::Event::Submit);
    assert_eq!(
        state.input.chips.len(),
        2,
        "each picked mention keeps its own chip"
    );
    let second = state.input.chips[1].clone();
    assert_eq!(
        &state.input.input[second.start..second.end],
        "@README.md",
        "second chip must cover exactly the second mention"
    );

    // BS1 removes the trailing space; BS2 removes only the second mention.
    state.update(crate::Event::Backspace);
    state.update(crate::Event::Backspace);
    assert_eq!(
        state.input.input, "read @README.md and ",
        "BS2 must remove only the second mention, leaving the first intact"
    );
    assert_eq!(state.input.chips.len(), 1);
    assert_eq!(state.input.chips[0].start, 5);
    assert_eq!(state.input.chips[0].end, 15);
}

#[test]
fn typing_after_pick_appends_after_trailing_space() {
    let mut state = AppState::default();
    pick_first_file(&mut state);
    state.update(crate::Event::Input('X'));
    assert!(
        state.input.input.ends_with(" X"),
        "typing after a pick must append after the trailing space, got {:?}",
        state.input.input
    );
}
