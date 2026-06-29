//! Event enum — generated from `taxonomy.json`. DO NOT EDIT.

use serde::{Deserialize, Serialize};
use strum::{Display, IntoStaticStr, VariantNames};

use crate::event::TransientLevel;
use crate::model::ThinkingLevel;
use crate::settings::SettingsCategory;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display, IntoStaticStr, VariantNames)]
#[serde(tag = "type", content = "data")]
#[strum(serialize_all = "PascalCase")]
pub enum Event {
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
    ToolInputDelta {
        id: String,
        content: String,
    },
    ToolEnd {
        id: String,
        duration_secs: f64,
        output: String,
    },
    ToolConstraintError {
        id: String,
        tool: String,
        violations: Vec<crate::tool::ConstraintViolation>,
    },
    ResponseDelta {
        id: String,
        content: String,
    },
    ThinkingDelta {
        id: String,
        content: String,
    },
    TextStart {
        id: String,
    },
    TextEnd {
        id: String,
    },
    ThinkingStart {
        id: String,
    },
    ThinkingEnd {
        id: String,
    },
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
    TurnStarted {
        id: String,
        request_id: String,
        content: String,
    },
    TurnAborted,
    TurnCompleted,
    TurnErrored {
        id: String,
        message: String,
    },
    TurnConstraintError {
        id: String,
        tool: String,
        message: String,
    },
    TokenStatsUpdated {
        tokens_in: usize,
        tokens_out: usize,
        speed_tps: f64,
    },
    StreamStarted {
        id: String,
    },
    UserMessageSubmitted {
        id: String,
        content: String,
    },
    QueueAborted {
        content: String,
    },
    QueuesCleared,
    SteeringDelivered {
        content: String,
        id: String,
    },
    FollowUpDelivered {
        content: String,
        id: String,
    },
    MessageDequeued {
        content: String,
    },
    IdGenerated(crate::actors::turn::NextIdResponse),
    AssistantMessageReady {
        message: crate::message::ChatMessage,
    },
    SetPrompt {
        name: String,
    },
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
    Quit,
    ForceQuit,
    Reset,
    Abort,
    ClearQueues,
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
    PendingEdit {
        path: String,
        original: String,
        proposed: String,
    },
    ApproveEdit,
    RejectEdit,
    GistShared {
        result: Result<String, String>,
    },
    ExternalEditorClosed {
        result: Result<String, String>,
    },
    ClipboardWritten {
        success: bool,
    },
    ClipboardRead {
        result: Result<String, String>,
    },
    ProcessResumed,
    BashOutput {
        command: String,
        output: String,
    },
    FilesWritten {
        count: usize,
        errors: Vec<String>,
    },
    EnvDetected {
        git_info: Option<crate::snapshot::GitInfo>,
        cwd_name: String,
    },
    FffSearchResult {
        request_id: u64,
        entries: Vec<crate::model::FffFileEntry>,
        query: String,
        indexed: bool,
    },
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
    Start,
    SelectProvider {
        provider: String,
    },
    SubmitKey {
        provider: String,
        key: String,
    },
    ToggleModel {
        model: String,
    },
    Save,
    Cancel,
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
    MessageReplayed {
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        provider: String,
    },
    PermissionResponse {
        request_id: String,
        action: crate::permissions::PermissionAction,
    },
    PermissionRequest {
        request_id: String,
        tool: String,
        input: serde_json::Value,
    },
    PermissionRequestDismissed,
    InputChanged {
        state: Box<crate::model::InputState>,
    },
    #[serde(skip)]
    ViewChanged {
        state: Box<crate::model::ViewState>,
    },
    CompletionChanged {
        state: Box<crate::model::CompletionState>,
    },
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
    ReadOnlyChanged {
        enabled: bool,
    },
    HistoryLoaded {
        entries: Vec<String>,
    },
    HistoryAppend {
        entry: String,
    },
    Up,
    Down,
    ForkSession {
        message_index: usize,
    },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,
    SessionTreeSelect {
        id: String,
    },
    SessionLoaded {
        name: String,
        events: Box<Vec<crate::event::DurableCoreEvent>>,
        metadata: Option<Box<crate::session::index::SessionMetadata>>,
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
    SessionChanged {
        state: Box<crate::model::SessionState>,
    },
    TransientMessage {
        content: String,
        level: TransientLevel,
    },
    TransientError {
        content: String,
    },
    ClearTransient,
    ShowDiagnostics,
    SystemMessage {
        content: String,
    },
    ConfigLoaded {
        config: Box<crate::config::Config>,
    },
}
