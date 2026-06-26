//! Typed messages for `InputActor`.

use crate::actors::GenericActorHandle;

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
    HistoryPrev,
    /// Navigate to the next history entry.
    HistoryNext,
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,

    // ── State mutations ────────────────────────────────────────────────────
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

/// Handle for sending messages to `InputActor`.
pub type InputActorHandle = GenericActorHandle<InputMsg>;
