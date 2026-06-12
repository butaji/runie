//! Centralized Event Types

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Input(char),
    Backspace,
    Newline, // Shift+Enter or Ctrl+J for multi-line input
    Submit,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,

    // Cursor movement (Emacs-style)
    CursorLeft,
    CursorRight,
    CursorStart,
    CursorEnd,

    // Text editing (Emacs-style)
    DeleteWord,    // Ctrl+W - delete word before cursor
    DeleteToEnd,   // Ctrl+K - delete from cursor to end
    DeleteToStart, // Ctrl+U - delete from start to cursor
    KillChar,      // Ctrl+D - delete char at cursor (if not empty)

    // Input history
    HistoryPrev, // Up arrow - previous history item
    HistoryNext, // Down arrow - next history item

    // Undo/redo
    Undo, // Ctrl+Z
    Redo, // Ctrl+Shift+Z

    // Word navigation
    CursorWordLeft,  // Alt+B - word backward
    CursorWordRight, // Alt+F - word forward

    // Bracketed paste
    Paste(String), // Terminal paste event
    PasteImage,    // Ctrl+V paste image from clipboard

    Quit,
    Reset,

    AgentThinking {
        id: String,
    },
    AgentThoughtDone {
        id: String,
    },
    AgentToolStart {
        id: String,
        name: String,
    },
    AgentToolEnd {
        duration_secs: f64,
        output: String,
    },
    AgentResponse {
        id: String,
        content: String,
    },
    AgentTurnComplete {
        id: String,
        duration_secs: f64,
    },
    AgentDone {
        id: String,
    },
    AgentError {
        id: String,
        message: String,
    },

    SwitchModel {
        provider: String,
        model: String,
    },
    SwitchTheme {
        name: String,
    },
    CycleModelNext,
    CycleModelPrev,
    ToggleScopedModelsDialog,
    ScopedModelToggle {
        name: String,
    },
    ScopedModelEnableAll,
    ScopedModelDisableAll,
    ScopedModelToggleProvider {
        provider: String,
    },
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

    SpawnAgent {
        prompt: String,
    },
    ToggleExpand,
    Dequeue,
    OpenExternalEditor,
    ExternalEditorDone {
        content: String,
    },

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
    PendingEdit {
        path: String,
        original: String,
        proposed: String,
        diff: String,
    },
    ApproveEdit,
    RejectEdit,

    // Config reload
    ReloadAll,

    // Diagnostics
    ShowDiagnostics,

    // Session tree
    ForkSession {
        message_index: usize,
    },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,
    SessionTreeSelect {
        id: String,
    },

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
    SystemMessage {
        content: String,
    },

    // @-file picker dialog
    AtFilePicker,
    InsertAtRef(String),

    // Command form dialog
    CommandFormInput(char),
    CommandFormBackspace,
    CommandFormUp,
    CommandFormDown,
    CommandFormSubmit,
    CommandFormClose,
    /// ESC / dialog back: pop one panel in a dialog, close at root.
    /// Distinct from `Abort` (force-close, bypasses stack).
    DialogBack,
    RunSaveCommand {
        name: String,
    },
    RunLoadCommand {
        name: String,
    },
    RunDeleteCommand {
        name: String,
    },
    RunImportCommand {
        path: String,
    },
    RunExportCommand {
        path: String,
    },
    RunSkillCommand {
        name: String,
    },
    RunLoginCommand {
        provider: String,
        token: String,
    },
    RunLogoutCommand {
        provider: String,
    },
    RunNameCommand {
        name: String,
    },
    RunForkCommand {
        message_index: String,
    },
    RunCompactCommand {
        keep: String,
        focus: String,
    },
    RunPromptCommand {
        name: String,
    },
    RunThinkingCommand {
        level: crate::model::ThinkingLevel,
    },

    // Unified palette command execution
    RunPaletteCommand {
        name: String,
        args: String,
    },

    // Settings category switching
    SettingsSwitchCategory {
        category: crate::settings::SettingsCategory,
    },

    // Providers dialog (unified login/logout/select)
    ProvidersDialog,
    ProvidersSelectModel {
        provider: String,
        model: String,
    },
    ProvidersDisconnect {
        provider: String,
    },
    ProvidersAdd,

    // Login dialog flow
    LoginFlowStart,
    LoginFlowSelectProvider {
        provider: String,
    },
    LoginFlowSubmitKey {
        provider: String,
        key: String,
    },
    LoginFlowValidate {
        provider: String,
        key: String,
    },
    LoginFlowValidationDone {
        provider: String,
        key: String,
        models: Vec<String>,
    },
    LoginFlowValidationFailed {
        provider: String,
        key: String,
        error: String,
    },
    LoginFlowModelsFetched {
        provider: String,
        key: String,
        models: Vec<String>,
    },
    LoginFlowToggleModel {
        model: String,
    },
    LoginFlowSave,
    LoginFlowCancel,

    // Transient notifications (shown in hints line)
    TransientMessage {
        content: String,
        level: TransientLevel,
    },
    TransientError {
        content: String,
    },
    ClearTransient,
}

/// Severity level for transient notifications shown in the hints line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Routing category for [`Event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    Input,
    Agent,
    Scroll,
    Control,
    ModelConfig,
    DialogToggle,
    Settings,
    Edit,
    System,
    Transient,
}

impl Event {
    /// Whether this event belongs to the login flow.
    pub fn is_login(&self) -> bool {
        matches!(
            self,
            Event::LoginFlowStart
                | Event::LoginFlowSelectProvider { .. }
                | Event::LoginFlowSubmitKey { .. }
                | Event::LoginFlowValidate { .. }
                | Event::LoginFlowValidationDone { .. }
                | Event::LoginFlowValidationFailed { .. }
                | Event::LoginFlowModelsFetched { .. }
                | Event::LoginFlowToggleModel { .. }
                | Event::LoginFlowSave
                | Event::LoginFlowCancel
        )
    }

    /// Categorizes an event for dispatch.
    pub fn category(&self) -> EventCategory {
        use Event::*;
        match self {
            Input(_) | Backspace | Newline | Submit | CursorLeft | CursorRight | CursorStart
            | CursorEnd | DeleteWord | DeleteToEnd | DeleteToStart | KillChar | Undo | Redo
            | CursorWordLeft | CursorWordRight | Paste(_) | PasteImage | HistoryPrev
            | HistoryNext => EventCategory::Input,
            AgentThinking { .. }
            | AgentThoughtDone { .. }
            | AgentToolStart { .. }
            | AgentToolEnd { .. }
            | AgentResponse { .. }
            | AgentTurnComplete { .. }
            | AgentDone { .. }
            | AgentError { .. } => EventCategory::Agent,
            ScrollUp | ScrollDown | PageUp | PageDown => EventCategory::Scroll,
            Quit | Reset | Abort | SpawnAgent { .. } | ToggleExpand | OpenExternalEditor
            | ExternalEditorDone { .. } | Suspend | ShareSession | ForkSession { .. }
            | CloneSession | ToggleSessionTree | SessionTreeFilterCycle | SessionTreeSelect { .. }
            | AtFilePicker | InsertAtRef(_) => EventCategory::Control,
            SwitchModel { .. } | SwitchTheme { .. } | CycleModelNext | CycleModelPrev
            | ToggleScopedModelsDialog | ScopedModelToggle { .. } | ScopedModelEnableAll
            | ScopedModelDisableAll | ScopedModelToggleProvider { .. } | CycleThinkingLevel
            | SetThinkingLevel(_) | ToggleReadOnly | TrustProject | UntrustProject | FollowUp
            | Dequeue => EventCategory::ModelConfig,
            ToggleCommandPalette | PaletteFilter(_) | PaletteBackspace | PaletteUp | PaletteDown
            | PaletteSelect | PaletteClose | ToggleModelSelector | ModelSelectorFilter(_)
            | ModelSelectorBackspace | ModelSelectorUp | ModelSelectorDown | ModelSelectorSelect
            | ModelSelectorClose | CommandFormInput(_) | CommandFormBackspace | CommandFormUp
            | CommandFormDown | CommandFormSubmit | CommandFormClose => EventCategory::DialogToggle,
            ToggleSettingsDialog | SettingsUp | SettingsDown | SettingsLeft | SettingsRight
            | SettingsSelect | SettingsClose | SettingsSwitchCategory { .. } => {
                EventCategory::Settings
            }
            PendingEdit { .. } | ApproveEdit | RejectEdit | ReloadAll | ShowDiagnostics
            | TogglePathCompletion | PathCompletionUp | PathCompletionDown | PathCompletionSelect
            | PathCompletionClose | RunSaveCommand { .. } | RunLoadCommand { .. }
            | RunDeleteCommand { .. } | RunImportCommand { .. } | RunExportCommand { .. }
            | RunSkillCommand { .. } | RunLoginCommand { .. } | RunLogoutCommand { .. }
            | RunNameCommand { .. } | RunForkCommand { .. } | RunCompactCommand { .. }
            | RunPromptCommand { .. } | RunThinkingCommand { .. } | RunPaletteCommand { .. } => {
                EventCategory::Edit
            }
            SystemMessage { .. } => EventCategory::System,
            TransientMessage { .. } | TransientError { .. } | ClearTransient => {
                EventCategory::Transient
            }
            LoginFlowStart
            | LoginFlowSelectProvider { .. }
            | LoginFlowSubmitKey { .. }
            | LoginFlowValidate { .. }
            | LoginFlowValidationDone { .. }
            | LoginFlowValidationFailed { .. }
            | LoginFlowModelsFetched { .. }
            | LoginFlowToggleModel { .. }
            | LoginFlowSave
            | LoginFlowCancel => EventCategory::Control,
        }
    }
}
