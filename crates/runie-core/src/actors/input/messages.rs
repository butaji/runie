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
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,

    // ── State mutations ────────────────────────────────────────────────────
    /// Submit content — clears input and publishes InputChanged.
    Submit { content: String },
    /// Replace all input text and reset cursor.
    SetText { text: String },
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
impl InputMsg {
    pub fn apply_to(&self, state: &mut crate::model::InputState) {
        use crate::update::input::{
            find_word_boundary_left, find_word_boundary_right, next_grapheme_boundary,
            prev_grapheme_boundary,
        };
        match self {
            InputMsg::InsertChar(c) => {
                state.push_undo();
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
                    let new_pos = prev_grapheme_boundary(&state.input, state.cursor_pos);
                    state.input.drain(new_pos..state.cursor_pos);
                    state.cursor_pos = new_pos;
                    state.redo_stack.clear();
                }
            }
            InputMsg::Newline => {
                state.push_undo();
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
                state.input.drain(start..state.cursor_pos);
                state.cursor_pos = start;
                state.redo_stack.clear();
            }
            InputMsg::DeleteToEnd => {
                if state.cursor_pos < state.input.len() {
                    state.push_undo();
                    state.input.truncate(state.cursor_pos);
                    state.redo_stack.clear();
                }
            }
            InputMsg::DeleteToStart => {
                if state.cursor_pos > 0 {
                    state.push_undo();
                    state.input.drain(..state.cursor_pos);
                    state.cursor_pos = 0;
                    state.redo_stack.clear();
                }
            }
            InputMsg::KillChar => {
                if state.cursor_pos < state.input.len() {
                    let end = next_grapheme_boundary(&state.input, state.cursor_pos);
                    state.push_undo();
                    state.input.drain(state.cursor_pos..end);
                    state.redo_stack.clear();
                }
            }
            InputMsg::Paste(text) => {
                // Flatten line breaks to spaces (not "") so pasted multi-line
                // text stays readable — mirrors AppState::paste.
                let clean = text
                    .replace("\r\n", " ")
                    .replace(['\r', '\n'], " ")
                    .replace('\t', "    ");
                if clean.is_empty() {
                    return;
                }
                state.push_undo();
                state.input.insert_str(state.cursor_pos, &clean);
                state.cursor_pos += clean.len();
                state.redo_stack.clear();
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
                    Some(p) => p,
                    None => state.input_history.len() - 1,
                };
                state.history_pos = Some(pos);
                state.input = state.input_history[pos].clone();
                state.cursor_pos = state.input.len();
            }
            InputMsg::HistoryNext => {
                let pos = match state.history_pos {
                    Some(p) => p + 1,
                    None => return,
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
            }
            InputMsg::Undo => {
                if let Some((text, pos)) = state.undo_stack.pop() {
                    state
                        .redo_stack
                        .push((state.input.clone(), state.cursor_pos));
                    state.input = text;
                    state.cursor_pos = pos;
                }
            }
            InputMsg::Redo => {
                if let Some((text, pos)) = state.redo_stack.pop() {
                    state
                        .undo_stack
                        .push((state.input.clone(), state.cursor_pos));
                    state.input = text;
                    state.cursor_pos = pos;
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
            }
            InputMsg::SetText { .. } | InputMsg::SetPrompt { .. } | InputMsg::Clear => {
                // These all clear input state.
                state.input.clear();
                state.cursor_pos = 0;
                state.history_pos = None;
                state.undo_stack.clear();
                state.redo_stack.clear();
                state.input_scroll = 0;
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
                    if !state.input.is_empty() && !state.input.ends_with('\n') {
                        state.input.push('\n');
                    }
                    state.input.push_str(&combined);
                    state.cursor_pos = state.input.len();
                    state.redo_stack.clear();
                }
            }
            InputMsg::InsertAtRef { text } => {
                state.push_undo();
                state.input.insert_str(state.cursor_pos, text);
                state.cursor_pos += text.len();
                state.redo_stack.clear();
            }
            InputMsg::FilePickerAbort => {
                if let Some((input, cursor, _, _)) = state.file_picker_backup.take() {
                    state.input = input;
                    state.cursor_pos = cursor;
                }
            }
        }
    }
}
