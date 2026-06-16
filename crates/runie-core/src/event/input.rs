//! Input event variants (keyboard, mouse, clipboard, terminal).

use std::fmt;
use strum::IntoStaticStr;

/// Keyboard, mouse, clipboard, and terminal-focus input events.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum InputEvent {
    /// A single character input.
    Input(char),
    Backspace,
    /// Shift+Enter or Ctrl+J for multi-line input.
    Newline,
    Submit,
    Escape,
    CursorLeft,
    CursorRight,
    CursorStart,
    CursorEnd,
    DeleteWord,
    DeleteToEnd,
    DeleteToStart,
    KillChar,
    HistoryPrev,
    HistoryNext,
    Undo,
    Redo,
    CursorWordLeft,
    CursorWordRight,
    PageUp,
    PageDown,
    GoToTop,
    GoToBottom,
    /// Terminal paste event with content.
    Paste(String),
    /// Ctrl+V paste image from clipboard.
    PasteImage,
    MouseClick { row: u16, col: u16, button: String },
    MouseRelease { row: u16, col: u16, button: String },
    MouseDrag { row: u16, col: u16, button: String },
    MouseMove { row: u16, col: u16 },
    MouseScrollUp,
    MouseScrollDown,
    FocusGained,
    FocusLost,
    /// Terminal resize event.
    TerminalSize { width: u16, height: u16 },
}

impl InputEvent {
    /// Canonical name for bindable key events. Returns `None` for parameterized variants.
    pub fn variant_name(&self) -> Option<&'static str> {
        match self {
            InputEvent::Input(_) => None,
            InputEvent::Backspace => Some("Backspace"),
            InputEvent::Newline => Some("Newline"),
            InputEvent::Submit => Some("Submit"),
            InputEvent::Escape => Some("Escape"),
            InputEvent::CursorLeft => Some("CursorLeft"),
            InputEvent::CursorRight => Some("CursorRight"),
            InputEvent::CursorStart => Some("CursorStart"),
            InputEvent::CursorEnd => Some("CursorEnd"),
            InputEvent::DeleteWord => Some("DeleteWord"),
            InputEvent::DeleteToEnd => Some("DeleteToEnd"),
            InputEvent::DeleteToStart => Some("DeleteToStart"),
            InputEvent::KillChar => Some("KillChar"),
            InputEvent::HistoryPrev => Some("HistoryPrev"),
            InputEvent::HistoryNext => Some("HistoryNext"),
            InputEvent::Undo => Some("Undo"),
            InputEvent::Redo => Some("Redo"),
            InputEvent::CursorWordLeft => Some("CursorWordLeft"),
            InputEvent::CursorWordRight => Some("CursorWordRight"),
            InputEvent::PageUp => Some("PageUp"),
            InputEvent::PageDown => Some("PageDown"),
            InputEvent::GoToTop => Some("GoToTop"),
            InputEvent::GoToBottom => Some("GoToBottom"),
            InputEvent::Paste(_) => None,
            InputEvent::PasteImage => Some("PasteImage"),
            InputEvent::MouseClick { .. } => None,
            InputEvent::MouseRelease { .. } => None,
            InputEvent::MouseDrag { .. } => None,
            InputEvent::MouseMove { .. } => None,
            InputEvent::MouseScrollUp => None,
            InputEvent::MouseScrollDown => None,
            InputEvent::FocusGained => Some("FocusGained"),
            InputEvent::FocusLost => Some("FocusLost"),
            InputEvent::TerminalSize { .. } => None,
        }
    }
}

impl fmt::Display for InputEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputEvent::Input(c) => write!(f, "Input:{}", c),
            InputEvent::Backspace => write!(f, "Backspace"),
            InputEvent::Newline => write!(f, "Newline"),
            InputEvent::Submit => write!(f, "Submit"),
            InputEvent::Escape => write!(f, "Escape"),
            InputEvent::CursorLeft => write!(f, "CursorLeft"),
            InputEvent::CursorRight => write!(f, "CursorRight"),
            InputEvent::CursorStart => write!(f, "CursorStart"),
            InputEvent::CursorEnd => write!(f, "CursorEnd"),
            InputEvent::DeleteWord => write!(f, "DeleteWord"),
            InputEvent::DeleteToEnd => write!(f, "DeleteToEnd"),
            InputEvent::DeleteToStart => write!(f, "DeleteToStart"),
            InputEvent::KillChar => write!(f, "KillChar"),
            InputEvent::HistoryPrev => write!(f, "HistoryPrev"),
            InputEvent::HistoryNext => write!(f, "HistoryNext"),
            InputEvent::Undo => write!(f, "Undo"),
            InputEvent::Redo => write!(f, "Redo"),
            InputEvent::CursorWordLeft => write!(f, "CursorWordLeft"),
            InputEvent::CursorWordRight => write!(f, "CursorWordRight"),
            InputEvent::PageUp => write!(f, "PageUp"),
            InputEvent::PageDown => write!(f, "PageDown"),
            InputEvent::GoToTop => write!(f, "GoToTop"),
            InputEvent::GoToBottom => write!(f, "GoToBottom"),
            InputEvent::Paste(_) => write!(f, "Paste"),
            InputEvent::PasteImage => write!(f, "PasteImage"),
            InputEvent::MouseClick { .. } => write!(f, "MouseClick"),
            InputEvent::MouseRelease { .. } => write!(f, "MouseRelease"),
            InputEvent::MouseDrag { .. } => write!(f, "MouseDrag"),
            InputEvent::MouseMove { .. } => write!(f, "MouseMove"),
            InputEvent::MouseScrollUp => write!(f, "MouseScrollUp"),
            InputEvent::MouseScrollDown => write!(f, "MouseScrollDown"),
            InputEvent::FocusGained => write!(f, "FocusGained"),
            InputEvent::FocusLost => write!(f, "FocusLost"),
            InputEvent::TerminalSize { .. } => write!(f, "TerminalSize"),
        }
    }
}
