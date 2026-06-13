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

    // Mouse events
    MouseClick { row: u16, col: u16, button: String },
    MouseRelease { row: u16, col: u16, button: String },
    MouseDrag { row: u16, col: u16, button: String },
    MouseMove { row: u16, col: u16 },

    // Focus events (terminal focus tracking)
    FocusGained,
    FocusLost,

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

    // Clipboard
    CopyToClipboard(String),
    CopyLastResponse,

    // Vim navigation
    GoToTop,
    GoToBottom,
    ToggleVimMode,

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

pub const EVENT_NAMES: &[(&str, fn() -> Event)] = &[
    ("Backspace", || Event::Backspace),
    ("Newline", || Event::Newline),
    ("Submit", || Event::Submit),
    ("ScrollUp", || Event::ScrollUp),
    ("ScrollDown", || Event::ScrollDown),
    ("PageUp", || Event::PageUp),
    ("PageDown", || Event::PageDown),
    ("CursorLeft", || Event::CursorLeft),
    ("CursorRight", || Event::CursorRight),
    ("CursorStart", || Event::CursorStart),
    ("CursorEnd", || Event::CursorEnd),
    ("DeleteWord", || Event::DeleteWord),
    ("DeleteToEnd", || Event::DeleteToEnd),
    ("DeleteToStart", || Event::DeleteToStart),
    ("KillChar", || Event::KillChar),
    ("HistoryPrev", || Event::HistoryPrev),
    ("HistoryNext", || Event::HistoryNext),
    ("Undo", || Event::Undo),
    ("Redo", || Event::Redo),
    ("CursorWordLeft", || Event::CursorWordLeft),
    ("CursorWordRight", || Event::CursorWordRight),
    ("PasteImage", || Event::PasteImage),
    ("Quit", || Event::Quit),
    ("Reset", || Event::Reset),
    ("CycleModelNext", || Event::CycleModelNext),
    ("CycleModelPrev", || Event::CycleModelPrev),
    ("ToggleScopedModelsDialog", || {
        Event::ToggleScopedModelsDialog
    }),
    ("ScopedModelEnableAll", || Event::ScopedModelEnableAll),
    ("ScopedModelDisableAll", || Event::ScopedModelDisableAll),
    ("ToggleSettingsDialog", || Event::ToggleSettingsDialog),
    ("SettingsUp", || Event::SettingsUp),
    ("SettingsDown", || Event::SettingsDown),
    ("SettingsLeft", || Event::SettingsLeft),
    ("SettingsRight", || Event::SettingsRight),
    ("SettingsSelect", || Event::SettingsSelect),
    ("SettingsClose", || Event::SettingsClose),
    ("CycleThinkingLevel", || Event::CycleThinkingLevel),
    ("ToggleReadOnly", || Event::ToggleReadOnly),
    ("TrustProject", || Event::TrustProject),
    ("UntrustProject", || Event::UntrustProject),
    ("FollowUp", || Event::FollowUp),
    ("Abort", || Event::Abort),
    ("ToggleExpand", || Event::ToggleExpand),
    ("Dequeue", || Event::Dequeue),
    ("OpenExternalEditor", || Event::OpenExternalEditor),
    ("ToggleCommandPalette", || Event::ToggleCommandPalette),
    ("PaletteBackspace", || Event::PaletteBackspace),
    ("PaletteUp", || Event::PaletteUp),
    ("PaletteDown", || Event::PaletteDown),
    ("PaletteSelect", || Event::PaletteSelect),
    ("PaletteClose", || Event::PaletteClose),
    ("ToggleModelSelector", || Event::ToggleModelSelector),
    ("ModelSelectorBackspace", || Event::ModelSelectorBackspace),
    ("ModelSelectorUp", || Event::ModelSelectorUp),
    ("ModelSelectorDown", || Event::ModelSelectorDown),
    ("ModelSelectorSelect", || Event::ModelSelectorSelect),
    ("ModelSelectorClose", || Event::ModelSelectorClose),
    ("ApproveEdit", || Event::ApproveEdit),
    ("RejectEdit", || Event::RejectEdit),
    ("ReloadAll", || Event::ReloadAll),
    ("ShowDiagnostics", || Event::ShowDiagnostics),
    ("CloneSession", || Event::CloneSession),
    ("ToggleSessionTree", || Event::ToggleSessionTree),
    ("SessionTreeFilterCycle", || Event::SessionTreeFilterCycle),
    ("Suspend", || Event::Suspend),
    ("TogglePathCompletion", || Event::TogglePathCompletion),
    ("PathCompletionUp", || Event::PathCompletionUp),
    ("PathCompletionDown", || Event::PathCompletionDown),
    ("PathCompletionSelect", || Event::PathCompletionSelect),
    ("PathCompletionClose", || Event::PathCompletionClose),
    ("ShareSession", || Event::ShareSession),
    ("AtFilePicker", || Event::AtFilePicker),
    ("CommandFormBackspace", || Event::CommandFormBackspace),
    ("CommandFormUp", || Event::CommandFormUp),
    ("CommandFormDown", || Event::CommandFormDown),
    ("CommandFormSubmit", || Event::CommandFormSubmit),
    ("CommandFormClose", || Event::CommandFormClose),
    ("DialogBack", || Event::DialogBack),
    ("ProvidersDialog", || Event::ProvidersDialog),
    ("ProvidersAdd", || Event::ProvidersAdd),
    ("LoginFlowStart", || Event::LoginFlowStart),
    ("LoginFlowSave", || Event::LoginFlowSave),
    ("LoginFlowCancel", || Event::LoginFlowCancel),
    ("ClearTransient", || Event::ClearTransient),
    ("CopyLastResponse", || Event::CopyLastResponse),
    ("GoToTop", || Event::GoToTop),
    ("GoToBottom", || Event::GoToBottom),
    ("ToggleVimMode", || Event::ToggleVimMode),
];

impl Event {
    /// Canonical name for bindable (unit) variants.
    pub const fn name(&self) -> Option<&'static str> {
        Some(match self {
            Event::Input(_) => return None,
            Event::Backspace => "Backspace",
            Event::Newline => "Newline",
            Event::Submit => "Submit",
            Event::ScrollUp => "ScrollUp",
            Event::ScrollDown => "ScrollDown",
            Event::PageUp => "PageUp",
            Event::PageDown => "PageDown",
            Event::CursorLeft => "CursorLeft",
            Event::CursorRight => "CursorRight",
            Event::CursorStart => "CursorStart",
            Event::CursorEnd => "CursorEnd",
            Event::DeleteWord => "DeleteWord",
            Event::DeleteToEnd => "DeleteToEnd",
            Event::DeleteToStart => "DeleteToStart",
            Event::KillChar => "KillChar",
            Event::HistoryPrev => "HistoryPrev",
            Event::HistoryNext => "HistoryNext",
            Event::Undo => "Undo",
            Event::Redo => "Redo",
            Event::CursorWordLeft => "CursorWordLeft",
            Event::CursorWordRight => "CursorWordRight",
            Event::Paste(_) => return None,
            Event::PasteImage => "PasteImage",
            Event::Quit => "Quit",
            Event::Reset => "Reset",
            Event::AgentThinking { .. } => return None,
            Event::AgentThoughtDone { .. } => return None,
            Event::AgentToolStart { .. } => return None,
            Event::AgentToolEnd { .. } => return None,
            Event::AgentResponse { .. } => return None,
            Event::AgentTurnComplete { .. } => return None,
            Event::AgentDone { .. } => return None,
            Event::AgentError { .. } => return None,
            Event::SwitchModel { .. } => return None,
            Event::SwitchTheme { .. } => return None,
            Event::CycleModelNext => "CycleModelNext",
            Event::CycleModelPrev => "CycleModelPrev",
            Event::ToggleScopedModelsDialog => "ToggleScopedModelsDialog",
            Event::ScopedModelToggle { .. } => return None,
            Event::ScopedModelEnableAll => "ScopedModelEnableAll",
            Event::ScopedModelDisableAll => "ScopedModelDisableAll",
            Event::ScopedModelToggleProvider { .. } => return None,
            Event::ToggleSettingsDialog => "ToggleSettingsDialog",
            Event::SettingsUp => "SettingsUp",
            Event::SettingsDown => "SettingsDown",
            Event::SettingsLeft => "SettingsLeft",
            Event::SettingsRight => "SettingsRight",
            Event::SettingsSelect => "SettingsSelect",
            Event::SettingsClose => "SettingsClose",
            Event::CycleThinkingLevel => "CycleThinkingLevel",
            Event::SetThinkingLevel(_) => return None,
            Event::ToggleReadOnly => "ToggleReadOnly",
            Event::TrustProject => "TrustProject",
            Event::UntrustProject => "UntrustProject",
            Event::FollowUp => "FollowUp",
            Event::Abort => "Abort",
            Event::SpawnAgent { .. } => return None,
            Event::ToggleExpand => "ToggleExpand",
            Event::Dequeue => "Dequeue",
            Event::OpenExternalEditor => "OpenExternalEditor",
            Event::ExternalEditorDone { .. } => return None,
            Event::ToggleCommandPalette => "ToggleCommandPalette",
            Event::PaletteFilter(_) => return None,
            Event::PaletteBackspace => "PaletteBackspace",
            Event::PaletteUp => "PaletteUp",
            Event::PaletteDown => "PaletteDown",
            Event::PaletteSelect => "PaletteSelect",
            Event::PaletteClose => "PaletteClose",
            Event::ToggleModelSelector => "ToggleModelSelector",
            Event::ModelSelectorFilter(_) => return None,
            Event::ModelSelectorBackspace => "ModelSelectorBackspace",
            Event::ModelSelectorUp => "ModelSelectorUp",
            Event::ModelSelectorDown => "ModelSelectorDown",
            Event::ModelSelectorSelect => "ModelSelectorSelect",
            Event::ModelSelectorClose => "ModelSelectorClose",
            Event::PendingEdit { .. } => return None,
            Event::ApproveEdit => "ApproveEdit",
            Event::RejectEdit => "RejectEdit",
            Event::ReloadAll => "ReloadAll",
            Event::ShowDiagnostics => "ShowDiagnostics",
            Event::ForkSession { .. } => return None,
            Event::CloneSession => "CloneSession",
            Event::ToggleSessionTree => "ToggleSessionTree",
            Event::SessionTreeFilterCycle => "SessionTreeFilterCycle",
            Event::SessionTreeSelect { .. } => return None,
            Event::Suspend => "Suspend",
            Event::TogglePathCompletion => "TogglePathCompletion",
            Event::PathCompletionUp => "PathCompletionUp",
            Event::PathCompletionDown => "PathCompletionDown",
            Event::PathCompletionSelect => "PathCompletionSelect",
            Event::PathCompletionClose => "PathCompletionClose",
            Event::ShareSession => "ShareSession",
            Event::SystemMessage { .. } => return None,
            Event::AtFilePicker => "AtFilePicker",
            Event::InsertAtRef(_) => return None,
            Event::CopyToClipboard(_) => return None,
            Event::CopyLastResponse => return None,
            Event::GoToTop => return None,
            Event::GoToBottom => return None,
            Event::ToggleVimMode => return None,
            Event::MouseClick { .. } => return None,
            Event::MouseRelease { .. } => return None,
            Event::MouseDrag { .. } => return None,
            Event::MouseMove { .. } => return None,
            Event::FocusGained => return None,
            Event::FocusLost => return None,
            Event::CommandFormInput(_) => return None,
            Event::CommandFormBackspace => "CommandFormBackspace",
            Event::CommandFormUp => "CommandFormUp",
            Event::CommandFormDown => "CommandFormDown",
            Event::CommandFormSubmit => "CommandFormSubmit",
            Event::CommandFormClose => "CommandFormClose",
            Event::DialogBack => "DialogBack",
            Event::RunSaveCommand { .. } => return None,
            Event::RunLoadCommand { .. } => return None,
            Event::RunDeleteCommand { .. } => return None,
            Event::RunImportCommand { .. } => return None,
            Event::RunExportCommand { .. } => return None,
            Event::RunSkillCommand { .. } => return None,
            Event::RunLoginCommand { .. } => return None,
            Event::RunLogoutCommand { .. } => return None,
            Event::RunNameCommand { .. } => return None,
            Event::RunForkCommand { .. } => return None,
            Event::RunCompactCommand { .. } => return None,
            Event::RunPromptCommand { .. } => return None,
            Event::RunThinkingCommand { .. } => return None,
            Event::RunPaletteCommand { .. } => return None,
            Event::SettingsSwitchCategory { .. } => return None,
            Event::ProvidersDialog => "ProvidersDialog",
            Event::ProvidersSelectModel { .. } => return None,
            Event::ProvidersDisconnect { .. } => return None,
            Event::ProvidersAdd => "ProvidersAdd",
            Event::LoginFlowStart => "LoginFlowStart",
            Event::LoginFlowSelectProvider { .. } => return None,
            Event::LoginFlowSubmitKey { .. } => return None,
            Event::LoginFlowValidationDone { .. } => return None,
            Event::LoginFlowValidationFailed { .. } => return None,
            Event::LoginFlowModelsFetched { .. } => return None,
            Event::LoginFlowToggleModel { .. } => return None,
            Event::LoginFlowSave => "LoginFlowSave",
            Event::LoginFlowCancel => "LoginFlowCancel",
            Event::TransientMessage { .. } => return None,
            Event::TransientError { .. } => return None,
            Event::ClearTransient => "ClearTransient",
            Event::GoToTop => "GoToTop",
            Event::GoToBottom => "GoToBottom",
            Event::ToggleVimMode => "ToggleVimMode",
            Event::MouseClick { .. } => return None,
            Event::MouseRelease { .. } => return None,
            Event::MouseDrag { .. } => return None,
            Event::MouseMove { .. } => return None,
            Event::FocusGained => "FocusGained",
            Event::FocusLost => "FocusLost",
        })
    }

    /// Build an Event from its canonical name. Supports the special
    /// `Input:<char>` prefix for character input bindings.
    pub fn from_name(name: &str) -> Option<Event> {
        if let Some(rest) = name.strip_prefix("Input:") {
            let c = rest.chars().next()?;
            return Some(Event::Input(c));
        }
        EVENT_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, ctor)| ctor())
    }
}

/// Compile-time check: `Event::name` must be exhaustive. If a variant is
/// added without updating the match above, this block fails to compile.
const _: () = {
    fn _exhaustive(e: &Event) {
        let _ = e.name();
    }
};

/// Severity level for transient notifications shown in the hints line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientLevel {
    Info,
    Success,
    Warning,
    Error,
}
