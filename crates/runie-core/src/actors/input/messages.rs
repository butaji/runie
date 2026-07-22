//! Typed messages for `InputActor`.
//!
//! Each message variant carries the data needed to apply the mutation either:
//! - Asynchronously in `InputActor` (production)
//! - Synchronously via `apply_to()` (tests without a spawned actor)

/// All messages accepted by `InputActor`.
///
/// Covers text editing, cursor navigation, history, undo/redo, and clipboard.
#[derive(Debug, Clone)]
pub enum InputMsg {
    // ── Text editing ───────────────────────────────────────────────────────
    /// Insert a character at the cursor.
    InsertChar(char),
    /// Delete the character before the cursor.
    Backspace,
    /// Insert a newline at the cursor.
    Newline,
    /// Delete the word before the cursor.
    DeleteWord,
    /// Delete from cursor to end of line.
    DeleteToEnd,
    /// Delete from start of line to cursor.
    DeleteToStart,
    /// Delete the character after the cursor.
    KillChar,
    /// Paste text at the cursor.
    Paste(String),
    /// Paste image (placeholder — image paste was removed).
    PasteImage,

    // ── Cursor navigation ──────────────────────────────────────────────────
    /// Move cursor one character left.
    CursorLeft,
    /// Move cursor one character right.
    CursorRight,
    /// Move cursor to the start of the input.
    CursorStart,
    /// Move cursor to the end of the input.
    CursorEnd,
    /// Move cursor one word left.
    CursorWordLeft,
    /// Move cursor one word right.
    CursorWordRight,
    /// Set cursor to an absolute position (for line-up/down navigation).
    MoveCursor { pos: usize },

    // ── History & undo/redo ────────────────────────────────────────────────
    /// Navigate to the previous history entry.
    /// When the input box is empty this recalls the most recently persisted
    /// command; the history is loaded from disk on startup and is independent
    /// of modal dialog navigation (dialogs consume HistoryPrev before it reaches
    /// the chat input actor).
    HistoryPrev,
    /// Navigate to the next history entry.
    HistoryNext,
    /// Move the cursor one visual line up in a multi-line input.
    ///
    /// Column is preserved (clamped to the target line's length); on the
    /// first line — and for single-line input — the cursor goes to the very
    /// start of the input. grok parity: arrows with text in the box move the
    /// cursor, they never recall history.
    CursorLineUp,
    /// Move the cursor one visual line down in a multi-line input.
    ///
    /// Column is preserved (clamped to the target line's length); on the
    /// last line — and for single-line input — the cursor goes to the very
    /// end of the input.
    CursorLineDown,
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,

    // ── State mutations ────────────────────────────────────────────────────
    /// Submit content — clears input and publishes InputChanged.
    Submit { content: String },
    /// Replace all input text and reset cursor.
    SetText { text: String, chips: Vec<crate::model::InputChip> },
    /// Set the current prompt name.
    SetPrompt { name: String },
    /// Clear the input (reset text, cursor, undo/redo).
    Clear,
    /// Load history entries from disk.
    HistoryLoaded { entries: Vec<String> },
    /// Drain queued follow-up messages into input.
    DrainQueue { messages: Vec<String> },
    /// Insert text at the file reference position.
    InsertAtRef { text: String },
    /// Abort file picker — restore backup.
    FilePickerAbort,
}

/// Synchronous application of an `InputMsg` to `InputState`.
///
/// Mirrors `InputActor::handle_msg` for use in tests where the actor is not
/// spawned. This lets synchronous tests exercise the same mutation logic that
/// `InputActor` runs asynchronously in production.
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::too_many_lines)]
impl InputMsg {
    pub fn apply_to(&self, state: &mut crate::model::InputState) {
        use crate::update::input::{
            find_word_boundary_left, find_word_boundary_right, next_grapheme_boundary, prev_grapheme_boundary,
        };
        match self {
            InputMsg::InsertChar(c) => {
                state.push_undo();
                state.history_pos = None;
                state.adjust_chips_for_replace(state.cursor_pos, state.cursor_pos, c.len_utf8());
                if state.cursor_pos == state.input.len() {
                    state.input.push(*c);
                } else {
                    state.input.insert(state.cursor_pos, *c);
                }
                state.cursor_pos += c.len_utf8();
                state.redo_stack.clear();
            }
            InputMsg::Backspace => {
                if state.cursor_pos > 0 {
                    state.push_undo();
                    state.history_pos = None;
                    if let Some(chip) = state.chip_at_cursor_end() {
                        // Atomic delete: the whole chip goes in one press.
                        state.input.drain(chip.start..chip.end);
                        state.cursor_pos = chip.start;
                        state.adjust_chips_for_replace(chip.start, chip.end, 0);
                    } else {
                        let new_pos = prev_grapheme_boundary(&state.input, state.cursor_pos);
                        state.input.drain(new_pos..state.cursor_pos);
                        state.adjust_chips_for_replace(new_pos, state.cursor_pos, 0);
                        state.cursor_pos = new_pos;
                    }
                    state.redo_stack.clear();
                }
            }
            InputMsg::Newline => {
                state.push_undo();
                state.history_pos = None;
                state.adjust_chips_for_replace(state.cursor_pos, state.cursor_pos, 1);
                if state.cursor_pos == state.input.len() {
                    state.input.push('\n');
                } else {
                    state.input.insert(state.cursor_pos, '\n');
                }
                state.cursor_pos += 1;
                state.redo_stack.clear();
            }
            InputMsg::DeleteWord => {
                if state.cursor_pos == 0 {
                    return;
                }
                let start = find_word_boundary_left(&state.input, state.cursor_pos);
                state.push_undo();
                state.history_pos = None;
                state.input.drain(start..state.cursor_pos);
                state.adjust_chips_for_replace(start, state.cursor_pos, 0);
                state.cursor_pos = start;
                state.redo_stack.clear();
            }
            InputMsg::DeleteToEnd => {
                if state.cursor_pos < state.input.len() {
                    state.push_undo();
                    state.history_pos = None;
                    let end = state.input.len();
                    state.input.truncate(state.cursor_pos);
                    state.adjust_chips_for_replace(state.cursor_pos, end, 0);
                    state.redo_stack.clear();
                }
            }
            InputMsg::DeleteToStart => {
                if state.cursor_pos > 0 {
                    state.push_undo();
                    state.history_pos = None;
                    state.input.drain(..state.cursor_pos);
                    state.adjust_chips_for_replace(0, state.cursor_pos, 0);
                    state.cursor_pos = 0;
                    state.redo_stack.clear();
                }
            }
            InputMsg::KillChar => {
                if state.cursor_pos < state.input.len() {
                    let end = next_grapheme_boundary(&state.input, state.cursor_pos);
                    state.push_undo();
                    state.history_pos = None;
                    state.input.drain(state.cursor_pos..end);
                    state.adjust_chips_for_replace(state.cursor_pos, end, 0);
                    state.redo_stack.clear();
                }
            }
            InputMsg::Paste(text) => {
                // Preserves newlines (multi-line input); >3 lines becomes a
                // `[Pasted: N lines]` chip — grok parity.
                state.insert_paste(text);
                state.history_pos = None;
            }
            InputMsg::PasteImage => {
                state.input_flash = 3;
            }
            InputMsg::CursorLeft => {
                if state.cursor_pos > 0 {
                    state.cursor_pos = prev_grapheme_boundary(&state.input, state.cursor_pos);
                }
            }
            InputMsg::CursorRight => {
                if state.cursor_pos < state.input.len() {
                    state.cursor_pos = next_grapheme_boundary(&state.input, state.cursor_pos);
                }
            }
            InputMsg::CursorStart => {
                state.cursor_pos = 0;
            }
            InputMsg::CursorEnd => {
                state.cursor_pos = state.input.len();
            }
            InputMsg::CursorWordLeft => {
                if state.cursor_pos > 0 {
                    state.cursor_pos = find_word_boundary_left(&state.input, state.cursor_pos);
                }
            }
            InputMsg::CursorWordRight => {
                if state.cursor_pos < state.input.len() {
                    state.cursor_pos = find_word_boundary_right(&state.input, state.cursor_pos);
                }
            }
            InputMsg::MoveCursor { pos } => {
                state.cursor_pos = (*pos).min(state.input.len());
            }
            InputMsg::HistoryPrev => {
                if state.input_history.is_empty() {
                    state.input_flash = 3;
                    return;
                }
                let pos = match state.history_pos {
                    Some(p) if p > 0 => p - 1,
                    Some(_) => {
                        // Already at the oldest entry: stay, no wrap-around.
                        state.input_flash = 3;
                        return;
                    }
                    None => state.input_history.len() - 1,
                };
                state.history_pos = Some(pos);
                state.input = state.input_history[pos].clone();
                state.cursor_pos = state.input.len();
                state.chips.clear();
            }
            InputMsg::HistoryNext => {
                let pos = match state.history_pos {
                    Some(p) => p + 1,
                    None => {
                        state.input_flash = 3;
                        return;
                    }
                };
                if pos >= state.input_history.len() {
                    state.history_pos = None;
                    state.input.clear();
                    state.cursor_pos = 0;
                } else {
                    state.history_pos = Some(pos);
                    state.input = state.input_history[pos].clone();
                    state.cursor_pos = state.input.len();
                }
                state.chips.clear();
            }
            InputMsg::CursorLineUp => {
                let input = &state.input;
                if !input.contains('\n') {
                    // Single-line draft: Up goes to the start of the text.
                    state.cursor_pos = 0;
                    return;
                }
                let cursor = state.cursor_pos.min(input.len());
                let cur_start = input[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
                if cur_start == 0 {
                    // First line: Up goes to the very start of the input.
                    state.cursor_pos = 0;
                    return;
                }
                let prev_ls = input[..cur_start - 1]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                // Previous line spans [prev_ls, cur_start - 1); clamp the column
                // to its length (not to prev_ls — wrong from line 3 onward).
                let prev_line_len = cur_start - 1 - prev_ls;
                state.cursor_pos = prev_ls + (cursor - cur_start).min(prev_line_len);
            }
            InputMsg::CursorLineDown => {
                let input = &state.input;
                let input_len = input.len();
                let cursor = state.cursor_pos.min(input_len);
                let cur_start = input[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = input[cur_start..]
                    .find('\n')
                    .map(|i| cur_start + i)
                    .unwrap_or(input_len);
                if line_end >= input_len {
                    // Last line (or single-line): Down goes to the very end.
                    state.cursor_pos = input_len;
                    return;
                }
                let next_ls = line_end + 1;
                let next_le = input[next_ls..]
                    .find('\n')
                    .map(|i| next_ls + i)
                    .unwrap_or(input_len);
                state.cursor_pos = next_ls + (cursor - cur_start).min(next_le - next_ls);
            }
            InputMsg::Undo => {
                if let Some((text, pos)) = state.undo_stack.pop() {
                    state.history_pos = None;
                    state
                        .redo_stack
                        .push((state.input.clone(), state.cursor_pos));
                    state.input = text;
                    state.cursor_pos = pos;
                    state.chips.clear();
                }
            }
            InputMsg::Redo => {
                if let Some((text, pos)) = state.redo_stack.pop() {
                    state.history_pos = None;
                    state
                        .undo_stack
                        .push((state.input.clone(), state.cursor_pos));
                    state.input = text;
                    state.cursor_pos = pos;
                    state.chips.clear();
                }
            }
            InputMsg::Submit { content } => {
                // The content is dispatched by UiActor directly (captured
                // before sending); here we record it in the input history so
                // HistoryPrev can recall messages sent in this session, then
                // clear the input box like the other reset variants.
                if !content.is_empty() && state.input_history.last() != Some(content) {
                    state.input_history.push(content.clone());
                }
                state.input.clear();
                state.cursor_pos = 0;
                state.history_pos = None;
                state.undo_stack.clear();
                state.redo_stack.clear();
                state.input_scroll = 0;
                state.chips.clear();
            }
            InputMsg::SetText { text, chips } => {
                // Full-content sync (e.g. after an @-ref pick): replace the
                // text wholesale and put the cursor at the end.
                state.input = text.clone();
                state.cursor_pos = state.input.len();
                state.history_pos = None;
                state.undo_stack.clear();
                state.redo_stack.clear();
                state.input_scroll = 0;
                state.chips = chips.clone();
            }
            InputMsg::SetPrompt { .. } | InputMsg::Clear => {
                // These all clear input state.
                state.input.clear();
                state.cursor_pos = 0;
                state.history_pos = None;
                state.undo_stack.clear();
                state.redo_stack.clear();
                state.input_scroll = 0;
                state.chips.clear();
            }
            InputMsg::HistoryLoaded { entries } => {
                state.input_history = entries.clone();
            }
            InputMsg::DrainQueue { messages } => {
                let mut combined = String::new();
                for msg in messages {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(msg);
                }
                if !combined.is_empty() {
                    state.push_undo();
                    state.history_pos = None;
                    let append_at =
                        state.input.len() + usize::from(!state.input.is_empty() && !state.input.ends_with('\n'));
                    if !state.input.is_empty() && !state.input.ends_with('\n') {
                        state.input.push('\n');
                    }
                    state.input.push_str(&combined);
                    state.adjust_chips_for_replace(append_at, append_at, state.input.len() - append_at);
                    state.cursor_pos = state.input.len();
                    state.redo_stack.clear();
                }
            }
            InputMsg::InsertAtRef { text } => {
                state.push_undo();
                state.history_pos = None;
                state.adjust_chips_for_replace(state.cursor_pos, state.cursor_pos, text.len());
                state.input.insert_str(state.cursor_pos, text);
                state.cursor_pos += text.len();
                state.redo_stack.clear();
            }
            InputMsg::FilePickerAbort => {
                if let Some((input, cursor, _, _)) = state.file_picker_backup.take() {
                    state.input = input;
                    state.cursor_pos = cursor;
                    state.chips.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SetText` must replace the input with the given text (its documented
    /// contract: "Replace all input text and reset cursor"). Regression: the
    /// implementation ignored `text` and cleared the input, so syncing the
    /// picked @-ref to the InputActor wiped the box instead of filling it.
    #[test]
    fn set_text_replaces_input_content() {
        let mut state = crate::model::InputState::default();
        InputMsg::apply_to(
            &InputMsg::SetText { text: "read @a.rs".to_owned(), chips: Vec::new() },
            &mut state,
        );
        assert_eq!(state.input, "read @a.rs");
        assert_eq!(state.cursor_pos, "read @a.rs".len());
    }

    /// `SetText` replaces whatever was there before (it is a full sync, not
    /// an append).
    #[test]
    fn set_text_replaces_existing_content() {
        let mut state = crate::model::InputState::default();
        InputMsg::apply_to(&InputMsg::InsertChar('x'), &mut state);
        InputMsg::apply_to(
            &InputMsg::SetText { text: "new".to_owned(), chips: Vec::new() },
            &mut state,
        );
        assert_eq!(state.input, "new");
        assert_eq!(state.cursor_pos, 3);
    }

    /// `SetText` installs the carried chip spans (picked @-mentions sync
    /// their atomic region to the actor this way).
    #[test]
    fn set_text_installs_chips() {
        let mut state = crate::model::InputState::default();
        let chip = crate::model::InputChip { start: 5, end: 10, label: None };
        InputMsg::apply_to(
            &InputMsg::SetText { text: "read @a.rs ".to_owned(), chips: vec![chip.clone()] },
            &mut state,
        );
        assert_eq!(state.chips, vec![chip]);
    }

    /// Backspace exactly at a chip's end deletes the whole chip atomically;
    /// anywhere else it is a normal char-wise delete.
    #[test]
    fn backspace_at_chip_end_is_atomic() {
        let mut state = crate::model::InputState::default();
        InputMsg::apply_to(&InputMsg::Paste("l1\nl2\nl3\nl4".to_owned()), &mut state);
        assert_eq!(state.chips.len(), 1);
        InputMsg::apply_to(&InputMsg::Backspace, &mut state);
        assert_eq!(state.input, "");
        assert!(state.chips.is_empty());
        assert_eq!(state.cursor_pos, 0);
    }

    /// Editing inside a chip dissolves it: the delete is char-wise and the
    /// remaining text is no longer atomic.
    #[test]
    fn backspace_inside_chip_dissolves_it() {
        let mut state = crate::model::InputState::default();
        InputMsg::apply_to(&InputMsg::Paste("l1\nl2\nl3\nl4".to_owned()), &mut state);
        InputMsg::apply_to(&InputMsg::MoveCursor { pos: 2 }, &mut state);
        InputMsg::apply_to(&InputMsg::Backspace, &mut state);
        assert_eq!(state.input, "l\nl2\nl3\nl4");
        assert!(state.chips.is_empty());
    }
}
