//! Flat `Event` enum — every leaf variant lives at the top level.
//!
//! Sub-enums are reduced to type aliases for backward compatibility, so old
//! code such as `InputEvent::Submit` still resolves to `Event::Submit`.

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::model::ThinkingLevel;
use crate::settings::SettingsCategory;

mod constructors;
mod name;
mod to_durable;

/// The top-level event type for the entire application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    // Input
    Input(char),
    Backspace,
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
    Paste(String),
    PasteImage,
    MouseClick {
        row: u16,
        col: u16,
        button: String,
    },
    MouseRelease {
        row: u16,
        col: u16,
        button: String,
    },
    MouseDrag {
        row: u16,
        col: u16,
        button: String,
    },
    MouseMove {
        row: u16,
        col: u16,
    },
    MouseScrollUp,
    MouseScrollDown,
    FocusGained,
    FocusLost,
    TerminalSize {
        width: u16,
        height: u16,
    },

    // Agent
    Thinking {
        id: String,
    },
    ThoughtDone {
        id: String,
    },
    ToolStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolEnd {
        id: String,
        duration_secs: f64,
        output: String,
    },
    ResponseDelta {
        id: String,
        content: String,
    },
    /// A delta of thinking/reasoning content (from think filter).
    ThinkingDelta {
        id: String,
        content: String,
    },
    // LLM lifecycle events (populated from LLMEvent via LifecycleState)
    TextStart { id: String },
    TextEnd { id: String },
    ThinkingStart { id: String },
    ThinkingEnd { id: String },
    Response {
        id: String,
        content: String,
    },
    TurnComplete {
        id: String,
        duration_secs: f64,
    },
    Done {
        id: String,
    },
    Error {
        id: String,
        message: String,
    },
    PermissionRequest {
        request_id: String,
        tool: String,
        input: serde_json::Value,
    },
    PermissionResponse {
        request_id: String,
        action: crate::permissions::PermissionAction,
    },
    /// Assistant message ready with full content (used by agent to update
    /// AppState with the final message including tool calls after streaming).
    AssistantMessageReady {
        message: crate::message::ChatMessage,
    },

    // Replay
    MessageReplayed {
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        provider: String,
    },

    // Scroll
    Up,
    Down,

    // Control
    Quit,
    ForceQuit,
    Reset,
    Abort,
    FollowUp,
    ToggleExpand,
    Dequeue,
    OpenExternalEditor,
    ExternalEditorDone {
        content: String,
    },
    ShareSession,
    Suspend,
    ToggleVimMode,
    CopyLastResponse,
    OpenSessionList,
    NewSession,
    ResumeSession,
    SelectSession {
        id: String,
    },
    StarSession {
        id: String,
    },
    RenameSession {
        id: String,
        name: String,
    },
    DeleteSession {
        id: String,
    },

    // ModelConfig
    SwitchModel {
        provider: String,
        model: String,
        explicit: bool,
    },
    SwitchTheme {
        name: String,
    },
    CycleModelNext,
    CycleModelPrev,
    ToggleScopedModelsDialog,
    ScopedModelToggle {
        provider: String,
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
    SettingsSwitchCategory {
        category: SettingsCategory,
    },
    CycleThinkingLevel,
    SetThinkingLevel(ThinkingLevel),
    ToggleReadOnly,
    TrustProject,
    UntrustProject,
    ReloadAll,
    KeybindingsReloaded,

    // Dialog
    ToggleWelcome,
    ToggleCommandPalette,
    PaletteFilter(char),
    PaletteBackspace,
    PaletteUp,
    PaletteDown,
    PaletteSelect,
    PaletteClose,
    ToggleModelSelector,
    ModelSelectorFilter(char),
    ModelSelectorBackspace,
    ModelSelectorUp,
    ModelSelectorDown,
    ModelSelectorSelect,
    ModelSelectorClose,
    TogglePathCompletion,
    PathCompletionUp,
    PathCompletionDown,
    PathCompletionSelect,
    PathCompletionClose,
    CommandFormInput(char),
    CommandFormBackspace,
    CommandFormUp,
    CommandFormDown,
    CommandFormSubmit,
    CommandFormClose,
    DialogBack,
    ProvidersDialog,
    ProvidersSelectModel {
        provider: String,
        model: String,
    },
    ProvidersDisconnect {
        provider: String,
    },
    ProvidersAdd,
    ProvidersEditModels {
        provider: String,
    },
    CopyToClipboard(String),
    CopySelectedBlock,
    CopyBlockMetadata,
    AtFilePicker,
    InsertAtRef(String),

    // Edit
    PendingEdit {
        path: String,
        original: String,
        proposed: String,
    },
    ApproveEdit,
    RejectEdit,

    // System
    SystemMessage {
        content: String,
    },
    TransientMessage {
        content: String,
        level: super::TransientLevel,
    },
    TransientError {
        content: String,
    },
    ClearTransient,
    ShowDiagnostics,

    // Config
    ConfigLoaded {
        config: Box<crate::config::Config>,
    },

    // Persistence
    TrustLoaded {
        decisions: std::collections::HashMap<std::path::PathBuf, crate::trust::TrustDecision>,
    },
    TrustChanged {
        path: std::path::PathBuf,
        decision: crate::trust::TrustDecision,
    },
    TrustSet {
        path: std::path::PathBuf,
        decision: crate::trust::TrustDecision,
    },
    HistoryLoaded {
        entries: Vec<String>,
    },
    HistoryAppend {
        entry: String,
    },

    // Session persistence results
    SessionLoaded {
        name: String,
        events: Box<Vec<crate::event::DurableCoreEvent>>,
        metadata: Option<Box<crate::session_index::SessionMetadata>>,
    },
    SessionSaved {
        name: String,
    },
    SessionDeleted {
        name: String,
    },
    SessionImported {
        session: Box<crate::session::Session>,
    },
    SessionExported {
        path: String,
    },
    SessionList {
        sessions: Box<Vec<String>>,
    },
    SessionOperationFailed {
        operation: String,
        error: String,
    },

    // IO effects
    BashOutput {
        command: String,
        output: String,
    },
    FilesWritten {
        count: usize,
        errors: Vec<String>,
    },

    // Session
    ForkSession {
        message_index: usize,
    },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,
    SessionTreeSelect {
        id: String,
    },

    // Command
    RunLoadCommand {
        name: String,
    },
    RunSaveCommand {
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
        level: ThinkingLevel,
    },
    RunPaletteCommand {
        name: String,
        args: String,
    },

    // LoginFlow
    Start,
    SelectProvider {
        provider: String,
    },
    SubmitKey {
        provider: String,
        key: String,
    },
    ValidationFailed {
        provider: String,
        key: String,
        error: String,
    },
    ModelsFetched {
        provider: String,
        key: String,
        models: Vec<String>,
    },
    ToggleModel {
        model: String,
    },
    Save,
    Cancel,
}
