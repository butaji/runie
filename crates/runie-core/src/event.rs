//! Centralized Event Types

#[derive(Debug, Clone)]
pub enum Event {

    Input(char),
    Backspace,
    Submit,
    ScrollUp,
    ScrollDown,

    // Cursor movement (Emacs-style)
    CursorLeft,
    CursorRight,
    CursorStart,
    CursorEnd,

    // Text editing (Emacs-style)
    DeleteWord,      // Ctrl+W - delete word before cursor
    DeleteToEnd,     // Ctrl+K - delete from cursor to end
    DeleteToStart,   // Ctrl+U - delete from start to cursor
    KillChar,        // Ctrl+D - delete char at cursor (if not empty)

    // Input history
    HistoryPrev,     // Up arrow - previous history item
    HistoryNext,     // Down arrow - next history item

    Quit,
    Reset,

    AgentThinking { id: String },
    AgentThoughtDone { id: String },
    AgentToolStart { id: String, name: String },
    AgentToolEnd { duration_secs: f64, output: String },
    AgentResponse { id: String, content: String },
    AgentTurnComplete { id: String, duration_secs: f64 },
    AgentDone { id: String },
    AgentError { id: String, message: String },

    SwitchModel { provider: String, model: String },
    FollowUp,
    Abort,

    SpawnAgent,
    ToggleExpand,
}
