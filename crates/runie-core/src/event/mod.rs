//! Centralized Event Types
//!
//! ## Architecture
//!
//! `Event` is a single flat enum with all leaf variants at the top level.
//!
//! Durable events for JSONL persistence: [`DurableCoreEvent`](durable::DurableCoreEvent)
//!
//! ## Taxonomy
//!
//! Every `Event` variant is classified along two axes:
//! - [`EventKind`] — Intent / Fact / Control (routing)
//! - [`EventCategory`] — Agent / Command / Control / Dialog / Edit / IO / Input / ...
//!
//! Use `Event::kind()` and `Event::category()` to classify variants.

pub use durable::DurableCoreEvent;
pub use kind::EventKind;
pub use level::TransientLevel;

pub mod constructors;
pub mod durable;
pub mod from_provider_event;
pub mod headless;
pub mod intent;
pub mod kind;
mod level;
pub mod name;
pub mod to_durable;

#[cfg(test)]
mod tests;

// ─────────────────────────────────────────────────────────────────────────────
// Event enum
// ─────────────────────────────────────────────────────────────────────────────

use camino::Utf8PathBuf;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use strum::{Display, IntoStaticStr, VariantNames};

use crate::event::TransientLevel as TLevel;
use crate::model::ThinkingLevel;
use crate::settings::SettingsCategory;

/// All application events — a single flat enum.
///
/// Variants are classified by [`EventKind`](kind::EventKind) and
/// [`EventCategory`](EventCategory) for routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display, IntoStaticStr, VariantNames)]
#[serde(tag = "type", content = "data")]
#[strum(serialize_all = "PascalCase")]
pub enum Event {
    // ── Agent / Fact variants ───────────────────────────────────────────────
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
    /// Tool end with optional input for replay reconstruction.
    ToolEnd {
        id: String,
        #[serde(skip)]
        input: Option<serde_json::Value>,
        duration_secs: f64,
        output: String,
    },
    ToolConstraintError {
        id: String,
        tool: String,
        violations: Vec<crate::tool::ConstraintViolation>,
    },
    /// Turn journal phase: tool requests have been recorded from the LLM.
    /// Used for crash recovery.
    ToolRequestsRecorded {
        request_id: String,
    },
    /// Turn journal phase: response streaming has started.
    /// Used for crash recovery.
    ResponseDeltaStarted {
        request_id: String,
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
    /// Assistant response with optional durable metadata.
    Response {
        id: String,
        content: String,
        #[serde(skip)]
        role: String,
        #[serde(skip)]
        timestamp: f64,
        #[serde(skip)]
        provider: String,
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
    },
    CompactionTriggered {
        ratio: f64,
        tokens_in: usize,
        context_window: usize,
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
    QueueFollowUpAdded {
        id: String,
        content: String,
    },
    QueueSteeringAdded {
        id: String,
        content: String,
    },
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
    /// Swarm pattern worker spawned — transient feed row (GROK.md §26).
    PatternWorkerSpawned {
        id: String,
        description: String,
        model: String,
    },
    /// Swarm pattern worker finished — updates its feed row in place.
    /// `status` is "completed" or any other string (treated as failed).
    PatternWorkerFinished {
        id: String,
        status: String,
        duration_ms: u64,
        output: String,
    },

    // ── Command variants ─────────────────────────────────────────────────────
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

    // ── Control variants ─────────────────────────────────────────────────────
    Quit,
    ForceQuit,
    Reset,
    Abort,
    ClearQueues,
    FollowUp,
    ToggleExpand,
    ToggleTasksPane,
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

    // ── Plan mode variants ────────────────────────────────────────────────────
    /// Enable plan mode with optional initial content.
    PlanModeEnabled {
        content: String,
    },
    /// Disable plan mode.
    PlanModeDisabled,

    // ── Goal variants ────────────────────────────────────────────────────────
    /// Create a new goal with the given objective text.
    GoalCreate {
        objective: String,
    },
    /// Mark the goal as completed.
    GoalComplete {
        objective: String,
    },
    /// Pause the active goal.
    GoalPause,
    /// Resume a paused goal.
    GoalResume,
    /// Cancel the active goal.
    GoalCancel,
    /// Emit current goal status as a fact (used for UI updates).
    GoalStatus {
        status: String,
    },

    // ── Dialog variants ──────────────────────────────────────────────────────
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

    // ── Edit variants ────────────────────────────────────────────────────────
    PendingEdit {
        path: String,
        original: String,
        proposed: String,
    },
    ApproveEdit,
    RejectEdit,

    // ── IO / Fact variants ───────────────────────────────────────────────────
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
    SkillsLoaded {
        #[serde(skip)]
        skills: Vec<crate::skills::Skill>,
    },
    SkillCreated {
        name: String,
    },
    SkillDeleted {
        name: String,
    },
    SkillError {
        name: String,
        message: String,
    },
    AuthLoaded {
        providers: Vec<String>,
    },

    // ── Input variants ───────────────────────────────────────────────────────
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
    MouseScrollUp,
    MouseScrollDown,
    FocusGained,
    FocusLost,
    TerminalSize {
        width: u16,
        height: u16,
    },

    // ── LoginFlow variants ───────────────────────────────────────────────────
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

    // ── ModelConfig variants ─────────────────────────────────────────────────
    SwitchModel {
        provider: String,
        model: String,
        explicit: bool,
    },
    /// Model row picked in the `/model` selector: opens the per-model
    /// reasoning-level panel for this model (the switch itself happens via
    /// `SwitchModelWithLevel` once a level is chosen).
    SelectModel {
        provider: String,
        model: String,
    },
    /// Reasoning level chosen for a model in the selector: stores the
    /// per-model thinking override (`None` = inherit the global level) and
    /// switches to the model.
    SwitchModelWithLevel {
        provider: String,
        model: String,
        level: Option<ThinkingLevel>,
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
    /// Switch the agent orchestration pattern (`/mode`); `workers` overrides
    /// the configured worker count when present.
    SetMode {
        active: String,
        workers: Option<usize>,
    },
    /// Set the swarm execution variant (`parallel`, `delegation`, `dag`).
    SetSwarmVariant { variant: String },
    /// Switch to swarm and set its variant in one action (used by the mode picker).
    SetModeAndSwarmVariant {
        active: String,
        swarm_variant: String,
    },
    ToggleReadOnly,
    TrustProject,
    UntrustProject,
    ReloadAll,
    KeybindingsReloaded,

    // ── Other / Fact variants ────────────────────────────────────────────────
    MessageReplayed {
        id: String,
        role: String,
        content: String,
        timestamp: f64,
        provider: String,
    },

    // ── Permission variants ────────────────────────────────────────────────────
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
    PermissionAllow {
        request_id: String,
    },
    PermissionDeny {
        request_id: String,
    },
    /// Always allow — persists across sessions (stored in config).
    PermissionAlwaysAllow {
        request_id: String,
        tool: String,
    },
    /// This session — persists for the current session only.
    PermissionSessionAllow {
        request_id: String,
        tool: String,
    },
    /// Once — single use, will ask again next time.
    PermissionOnce {
        request_id: String,
    },

    // ── AskUserQuestion variants ─────────────────────────────────────────────────
    /// Open an interactive multi-question dialog.
    AskUserQuestion {
        request_id: String,
        questions: Vec<crate::model::Question>,
    },
    /// User answered the current question with an option.
    QuestionAnswer {
        request_id: String,
        option_id: String,
    },
    /// User skipped the current question.
    QuestionSkip {
        request_id: String,
    },
    /// All questions answered or skipped — submit the collected answers.
    QuestionSubmit {
        request_id: String,
    },

    // ── Persistence / Fact variants ──────────────────────────────────────────
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
        decisions: IndexMap<Utf8PathBuf, crate::trust::TrustDecision>,
    },
    TrustChanged {
        path: Utf8PathBuf,
        decision: crate::trust::TrustDecision,
    },
    TrustSet {
        path: Utf8PathBuf,
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

    // ── Session variants ──────────────────────────────────────────────────────
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
        metadata: Option<Box<crate::session::SessionMetadata>>,
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
    /// Fine-grained session change events (replacing whole-state SessionChanged).
    SessionMessageAdded {
        id: String,
        role: String,
        content: String,
    },
    SessionMessageUpdated {
        id: String,
        content: String,
    },
    SessionMetadataUpdated {
        name: Option<String>,
    },
    /// Legacy whole-state event (deprecated in favor of fine-grained events).
    SessionChanged {
        state: Box<crate::model::SessionState>,
    },
    /// Session tree snapshot (branching history) for durable persistence.
    SessionTreeSnapshot {
        snapshot: crate::session::tree::SessionTreeSnapshot,
    },

    // ── System / Fact variants ───────────────────────────────────────────────
    TransientMessage {
        content: String,
        level: TLevel,
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

    // ── MCP variants ─────────────────────────────────────────────────────────
    McpServerAction {
        name: String,
        action: crate::dialog::builders::mcp::McpServerActionKind,
    },
    ToggleMcpServersDialog,

    // ── Skills variants ────────────────────────────────────────────────────────
    SkillAction {
        name: String,
        action: crate::dialog::builders::skills::SkillActionKind,
    },
    ToggleSkillsDialog,
}

// ─────────────────────────────────────────────────────────────────────────────
// EventCategory
// ─────────────────────────────────────────────────────────────────────────────

/// Event category — routing taxonomy for the dispatcher.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    strum::Display,
    strum::IntoStaticStr,
    strum::VariantNames,
)]
pub enum EventCategory {
    Agent,
    Command,
    Control,
    Dialog,
    Edit,
    Goal,
    IO,
    Input,
    LoginFlow,
    ModelConfig,
    Other,
    Permission,
    Persistence,
    PlanMode,
    Question,
    Scroll,
    Session,
    System,
    #[default]
    Unknown,
}

// Event taxonomy — generated from taxonomy.json
pub mod generated;

// Re-export generated items so the public API is unchanged.
pub use generated::is_fact_variant;
pub use generated::EventCtor;
pub use generated::EVENT_NAMES;
