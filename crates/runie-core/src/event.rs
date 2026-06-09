//! Centralized Event Types

#[derive(Debug, Clone, PartialEq)]
pub enum Event {

    Input(char),
    Backspace,
    Newline,      // Shift+Enter or Ctrl+J for multi-line input
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

    // Undo/redo
    Undo,            // Ctrl+Z
    Redo,            // Ctrl+Shift+Z

    // Word navigation
    CursorWordLeft,  // Alt+B - word backward
    CursorWordRight, // Alt+F - word forward

    // Bracketed paste
    Paste(String),   // Terminal paste event
    PasteImage,      // Ctrl+V paste image from clipboard

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
    SwitchTheme { name: String },
    CycleModelNext,
    CycleModelPrev,
    ToggleScopedModelsDialog,
    ScopedModelToggle { name: String },
    ScopedModelEnableAll,
    ScopedModelDisableAll,
    ScopedModelToggleProvider { provider: String },
    ToggleSettingsDialog,
    SettingsUp,
    SettingsDown,
    SettingsLeft,
    SettingsRight,
    SettingsSelect,
    SettingsClose,
    CycleThinkingLevel,
    SetThinkingLevel(crate::model::ThinkingLevel),
    ToggleReadOnly,
    TrustProject,
    UntrustProject,
    FollowUp,
    Abort,

    SpawnAgent,
    ToggleExpand,
    Dequeue,
    OpenExternalEditor,
    ExternalEditorDone { content: String },

    // Command palette
    ToggleCommandPalette,
    PaletteFilter(char),
    PaletteBackspace,
    PaletteUp,
    PaletteDown,
    PaletteSelect,
    PaletteClose,

    // Model selector
    ToggleModelSelector,
    ModelSelectorFilter(char),
    ModelSelectorBackspace,
    ModelSelectorUp,
    ModelSelectorDown,
    ModelSelectorSelect,
    ModelSelectorClose,

    // Edit preview / approval
    PendingEdit { path: String, original: String, proposed: String, diff: String },
    ApproveEdit,
    RejectEdit,

    // Config reload
    ReloadAll,

    // Diagnostics
    ShowDiagnostics,

    // Session tree
    ForkSession { message_index: usize },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,

    // Suspend to background (Unix only)
    Suspend,

    // Path completion
    TogglePathCompletion,
    PathCompletionUp,
    PathCompletionDown,
    PathCompletionSelect,
    PathCompletionClose,

    // Session sharing
    ShareSession,
    SystemMessage { content: String },
}
