//! Chip (atomic region) behavior for pasted blocks.
//!
//! Grok parity (see GROK.md §23 in runie-tests):
//! - Pasting more than 3 lines collapses to a `[Pasted: N lines]` chip.
//! - A chip is atomic: one Backspace at its end deletes the whole chip.
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
    state.update(crate::Event::InputChanged { state: Box::new(crate::model::InputState::default()) });
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
    state.update(crate::Event::InputChanged { state: Box::new(echoed) });
    assert_eq!(state.input.chips.len(), 1);
    assert_eq!(
        state.input.chips[0].label.as_deref(),
        Some("[Pasted: 4 lines]")
    );
}
