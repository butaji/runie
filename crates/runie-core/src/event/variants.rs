//! Top-level Event enum. See [`kind.rs`](kind) for taxonomy.

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::model::ThinkingLevel;
use crate::settings::SettingsCategory;

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
    Thinking { id: String },
    ThoughtDone { id: String },
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
    /// Tool call failed due to constraint violation at turn build time.
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
    // LLM lifecycle
    TextStart { id: String },
    TextEnd { id: String },
    ThinkingStart { id: String },
    ThinkingEnd { id: String },
    Response { id: String, content: String },
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

    TurnStarted { id: String, request_id: String, content: String },
    TurnAborted,
    TurnCompleted,
    TurnErrored { id: String, message: String },
    /// Turn failed due to tool constraint violation before provider call.
    TurnConstraintError {
        id: String,
        tool: String,
        message: String,
    },
    TokenStatsUpdated { tokens_in: usize, tokens_out: usize, speed_tps: f64 },
    StreamStarted { id: String },
    UserMessageSubmitted { id: String, content: String },
    QueueAborted { content: String },
    QueuesCleared,
    SteeringDelivered { content: String, id: String },
    FollowUpDelivered { content: String, id: String },
    MessageDequeued { content: String },
    IdGenerated(crate::actors::turn::NextIdResponse),

    PermissionRequest {
        request_id: String,
        tool: String,
        input: serde_json::Value,
    },
    PermissionResponse {
        request_id: String,
        action: crate::permissions::PermissionAction,
    },
    PermissionRequestDismissed,
    AssistantMessageReady { message: crate::message::ChatMessage },

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
    /// Input state changed — emitted by InputActor.
    InputChanged {
        state: Box<crate::model::InputState>,
    },
    /// View/cache state changed — emitted by ViewActor.
    /// Note: skipped for serialization since ViewState contains runtime-only
    /// cache data that cannot be serialized.
    #[serde(skip)]
    ViewChanged {
        state: Box<crate::model::ViewState>,
    },
    /// Completion state changed — emitted by CompletionActor.
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
    /// Read-only flag changed — emitted by TrustActor.
    ReadOnlyChanged {
        enabled: bool,
    },
    HistoryLoaded {
        entries: Vec<String>,
    },
    HistoryAppend {
        entry: String,
    },

    // IO effects (results)
    /// Gist URL or error from sharing session.
    GistShared { result: Result<String, String> },
    /// Text from external editor or error.
    ExternalEditorClosed { result: Result<String, String> },
    /// Clipboard write result.
    ClipboardWritten { success: bool },
    /// Clipboard read text.
    ClipboardRead { result: Result<String, String> },
    /// Process suspended/resumed.
    ProcessResumed,

    // Session persistence results
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
    /// Emitted when session state changes (messages, tree, pending edits).
    SessionChanged {
        state: Box<crate::model::SessionState>,
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
    /// Environment info detected at startup (cwd name, git info).
    EnvDetected {
        git_info: Option<crate::snapshot::GitInfo>,
        cwd_name: String,
    },
    /// FFF search results from FffIndexerActor.
    FffSearchResult {
        request_id: u64,
        entries: Vec<crate::model::FffFileEntry>,
        query: String,
        indexed: bool,
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
