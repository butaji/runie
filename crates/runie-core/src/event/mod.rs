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
    Thinking { id: String },
    ThoughtDone { id: String },
    ToolStart { id: String, name: String, input: serde_json::Value },
    ToolInputDelta { id: String, content: String },
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
    ResponseDelta { id: String, content: String },
    ThinkingDelta { id: String, content: String },
    TextStart { id: String },
    TextEnd { id: String },
    ThinkingStart { id: String },
    ThinkingEnd { id: String },
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
    TurnComplete { id: String, duration_secs: f64 },
    Done { id: String },
    Error { id: String, message: String },
    TurnStarted { id: String, request_id: String, content: String },
    TurnAborted,
    TurnCompleted,
    TurnErrored { id: String, message: String },
    TurnConstraintError { id: String, tool: String, message: String },
    TokenStatsUpdated { tokens_in: usize, tokens_out: usize, speed_tps: f64 },
    CompactionTriggered { ratio: f64, tokens_in: usize, context_window: usize },
    StreamStarted { id: String },
    UserMessageSubmitted { id: String, content: String },
    QueueAborted { content: String },
    QueuesCleared,
    SteeringDelivered { content: String, id: String },
    FollowUpDelivered { content: String, id: String },
    MessageDequeued { content: String },
    IdGenerated(crate::actors::turn::NextIdResponse),
    AssistantMessageReady { message: crate::message::ChatMessage },

    // ── Command variants ─────────────────────────────────────────────────────
    SetPrompt { name: String },
    RunLoadCommand { name: String },
    RunSaveCommand { name: String },
    RunDeleteCommand { name: String },
    RunImportCommand { path: String },
    RunExportCommand { path: String },
    RunSkillCommand { name: String },
    RunLoginCommand { provider: String, token: String },
    RunLogoutCommand { provider: String },
    RunNameCommand { name: String },
    RunForkCommand { message_index: String },
    RunCompactCommand { keep: String, focus: String },
    RunPromptCommand { name: String },
    RunThinkingCommand { level: ThinkingLevel },
    RunPaletteCommand { name: String, args: String },

    // ── Control variants ─────────────────────────────────────────────────────
    Quit,
    ForceQuit,
    Reset,
    Abort,
    ClearQueues,
    FollowUp,
    ToggleExpand,
    Dequeue,
    OpenExternalEditor,
    ExternalEditorDone { content: String },
    ShareSession,
    Suspend,
    ToggleVimMode,
    CopyLastResponse,
    OpenSessionList,
    NewSession,
    ResumeSession,
    SelectSession { id: String },
    StarSession { id: String },
    RenameSession { id: String, name: String },
    DeleteSession { id: String },

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
    ProvidersSelectModel { provider: String, model: String },
    ProvidersDisconnect { provider: String },
    ProvidersAdd,
    ProvidersEditModels { provider: String },
    CopyToClipboard(String),
    CopySelectedBlock,
    CopyBlockMetadata,
    AtFilePicker,
    InsertAtRef(String),

    // ── Edit variants ────────────────────────────────────────────────────────
    PendingEdit { path: String, original: String, proposed: String },
    ApproveEdit,
    RejectEdit,

    // ── IO / Fact variants ───────────────────────────────────────────────────
    GistShared { result: Result<String, String> },
    ExternalEditorClosed { result: Result<String, String> },
    ClipboardWritten { success: bool },
    ClipboardRead { result: Result<String, String> },
    ProcessResumed,
    BashOutput { command: String, output: String },
    FilesWritten { count: usize, errors: Vec<String> },
    EnvDetected { git_info: Option<crate::snapshot::GitInfo>, cwd_name: String },
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
    AuthLoaded { providers: Vec<String> },

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
    MouseClick { row: u16, col: u16, button: String },
    MouseRelease { row: u16, col: u16, button: String },
    MouseDrag { row: u16, col: u16, button: String },
    MouseMove { row: u16, col: u16 },
    MouseScrollUp,
    MouseScrollDown,
    FocusGained,
    FocusLost,
    TerminalSize { width: u16, height: u16 },

    // ── LoginFlow variants ───────────────────────────────────────────────────
    Start,
    SelectProvider { provider: String },
    SubmitKey { provider: String, key: String },
    ToggleModel { model: String },
    Save,
    Cancel,
    ValidationFailed { provider: String, key: String, error: String },
    ModelsFetched { provider: String, key: String, models: Vec<String> },

    // ── ModelConfig variants ─────────────────────────────────────────────────
    SwitchModel { provider: String, model: String, explicit: bool },
    SwitchTheme { name: String },
    CycleModelNext,
    CycleModelPrev,
    ToggleScopedModelsDialog,
    ScopedModelToggle { provider: String, name: String },
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
    SettingsSwitchCategory { category: SettingsCategory },
    CycleThinkingLevel,
    SetThinkingLevel(ThinkingLevel),
    ToggleReadOnly,
    TrustProject,
    UntrustProject,
    ReloadAll,
    KeybindingsReloaded,

    // ── Other / Fact variants ────────────────────────────────────────────────
    MessageReplayed { id: String, role: String, content: String, timestamp: f64, provider: String },

    // ── Permission variants ────────────────────────────────────────────────────
    PermissionResponse { request_id: String, action: crate::permissions::PermissionAction },
    PermissionRequest { request_id: String, tool: String, input: serde_json::Value },
    PermissionRequestDismissed,

    // ── Persistence / Fact variants ──────────────────────────────────────────
    InputChanged { state: Box<crate::model::InputState> },
    #[serde(skip)]
    ViewChanged { state: Box<crate::model::ViewState> },
    CompletionChanged { state: Box<crate::model::CompletionState> },
    TrustLoaded { decisions: IndexMap<Utf8PathBuf, crate::trust::TrustDecision> },
    TrustChanged { path: Utf8PathBuf, decision: crate::trust::TrustDecision },
    TrustSet { path: Utf8PathBuf, decision: crate::trust::TrustDecision },
    ReadOnlyChanged { enabled: bool },
    HistoryLoaded { entries: Vec<String> },
    HistoryAppend { entry: String },

    // ── Session variants ──────────────────────────────────────────────────────
    Up,
    Down,
    ForkSession { message_index: usize },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,
    SessionTreeSelect { id: String },
    SessionLoaded {
        name: String,
        events: Box<Vec<crate::event::DurableCoreEvent>>,
        metadata: Option<Box<crate::session::SessionMetadata>>,
    },
    SessionSaved { name: String },
    SessionDeleted { name: String },
    SessionImported { session: Box<crate::session::Session> },
    SessionExported { path: String },
    SessionList { sessions: Box<Vec<String>> },
    SessionOperationFailed { operation: String, error: String },
    SessionChanged { state: Box<crate::model::SessionState> },
    /// Session tree snapshot (branching history) for durable persistence.
    SessionTreeSnapshot { snapshot: crate::session::tree::SessionTreeSnapshot },

    // ── System / Fact variants ───────────────────────────────────────────────
    TransientMessage { content: String, level: TLevel },
    TransientError { content: String },
    ClearTransient,
    ShowDiagnostics,
    SystemMessage { content: String },
    ConfigLoaded { config: Box<crate::config::Config> },
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
    IO,
    Input,
    LoginFlow,
    ModelConfig,
    Other,
    Permission,
    Persistence,
    Scroll,
    Session,
    System,
    #[default]
    Unknown,
}

// ─────────────────────────────────────────────────────────────────────────────
// Event classification methods
// ─────────────────────────────────────────────────────────────────────────────

impl Event {
    /// Return the [`EventKind`] for this event variant.
    pub fn kind(&self) -> EventKind {
        match self {
            // Intent variants
            Event::SetPrompt { .. } => EventKind::Intent,
            Event::RunLoadCommand { .. } => EventKind::Intent,
            Event::RunSaveCommand { .. } => EventKind::Intent,
            Event::RunDeleteCommand { .. } => EventKind::Intent,
            Event::RunImportCommand { .. } => EventKind::Intent,
            Event::RunExportCommand { .. } => EventKind::Intent,
            Event::RunSkillCommand { .. } => EventKind::Intent,
            Event::RunLoginCommand { .. } => EventKind::Intent,
            Event::RunLogoutCommand { .. } => EventKind::Intent,
            Event::RunNameCommand { .. } => EventKind::Intent,
            Event::RunForkCommand { .. } => EventKind::Intent,
            Event::RunCompactCommand { .. } => EventKind::Intent,
            Event::RunPromptCommand { .. } => EventKind::Intent,
            Event::RunThinkingCommand { .. } => EventKind::Intent,
            Event::RunPaletteCommand { .. } => EventKind::Intent,
            Event::ToggleWelcome => EventKind::Intent,
            Event::ToggleCommandPalette => EventKind::Intent,
            Event::PaletteFilter(_) => EventKind::Intent,
            Event::PaletteBackspace => EventKind::Intent,
            Event::PaletteUp => EventKind::Intent,
            Event::PaletteDown => EventKind::Intent,
            Event::PaletteSelect => EventKind::Intent,
            Event::PaletteClose => EventKind::Intent,
            Event::ToggleModelSelector => EventKind::Intent,
            Event::ModelSelectorFilter(_) => EventKind::Intent,
            Event::ModelSelectorBackspace => EventKind::Intent,
            Event::ModelSelectorUp => EventKind::Intent,
            Event::ModelSelectorDown => EventKind::Intent,
            Event::ModelSelectorSelect => EventKind::Intent,
            Event::ModelSelectorClose => EventKind::Intent,
            Event::TogglePathCompletion => EventKind::Intent,
            Event::PathCompletionUp => EventKind::Intent,
            Event::PathCompletionDown => EventKind::Intent,
            Event::PathCompletionSelect => EventKind::Intent,
            Event::PathCompletionClose => EventKind::Intent,
            Event::CommandFormInput(_) => EventKind::Intent,
            Event::CommandFormBackspace => EventKind::Intent,
            Event::CommandFormUp => EventKind::Intent,
            Event::CommandFormDown => EventKind::Intent,
            Event::CommandFormSubmit => EventKind::Intent,
            Event::CommandFormClose => EventKind::Intent,
            Event::DialogBack => EventKind::Intent,
            Event::ProvidersDialog => EventKind::Intent,
            Event::ProvidersSelectModel { .. } => EventKind::Intent,
            Event::ProvidersDisconnect { .. } => EventKind::Intent,
            Event::ProvidersAdd => EventKind::Intent,
            Event::ProvidersEditModels { .. } => EventKind::Intent,
            Event::CopyToClipboard(_) => EventKind::Intent,
            Event::CopySelectedBlock => EventKind::Intent,
            Event::CopyBlockMetadata => EventKind::Intent,
            Event::AtFilePicker => EventKind::Intent,
            Event::InsertAtRef(_) => EventKind::Intent,
            Event::PendingEdit { .. } => EventKind::Intent,
            Event::ApproveEdit => EventKind::Intent,
            Event::RejectEdit => EventKind::Intent,
            Event::Input(_) => EventKind::Intent,
            Event::Backspace => EventKind::Intent,
            Event::Newline => EventKind::Intent,
            Event::Submit => EventKind::Intent,
            Event::Escape => EventKind::Intent,
            Event::CursorLeft => EventKind::Intent,
            Event::CursorRight => EventKind::Intent,
            Event::CursorStart => EventKind::Intent,
            Event::CursorEnd => EventKind::Intent,
            Event::DeleteWord => EventKind::Intent,
            Event::DeleteToEnd => EventKind::Intent,
            Event::DeleteToStart => EventKind::Intent,
            Event::KillChar => EventKind::Intent,
            Event::HistoryPrev => EventKind::Intent,
            Event::HistoryNext => EventKind::Intent,
            Event::Undo => EventKind::Intent,
            Event::Redo => EventKind::Intent,
            Event::CursorWordLeft => EventKind::Intent,
            Event::CursorWordRight => EventKind::Intent,
            Event::PageUp => EventKind::Intent,
            Event::PageDown => EventKind::Intent,
            Event::GoToTop => EventKind::Intent,
            Event::GoToBottom => EventKind::Intent,
            Event::Paste(_) => EventKind::Intent,
            Event::PasteImage => EventKind::Intent,
            Event::MouseClick { .. } => EventKind::Intent,
            Event::MouseRelease { .. } => EventKind::Intent,
            Event::MouseDrag { .. } => EventKind::Intent,
            Event::MouseMove { .. } => EventKind::Intent,
            Event::MouseScrollUp => EventKind::Intent,
            Event::MouseScrollDown => EventKind::Intent,
            Event::FocusGained => EventKind::Intent,
            Event::FocusLost => EventKind::Intent,
            Event::TerminalSize { .. } => EventKind::Intent,
            Event::Start => EventKind::Intent,
            Event::SelectProvider { .. } => EventKind::Intent,
            Event::SubmitKey { .. } => EventKind::Intent,
            Event::ToggleModel { .. } => EventKind::Intent,
            Event::Save => EventKind::Intent,
            Event::Cancel => EventKind::Intent,
            Event::SwitchModel { .. } => EventKind::Intent,
            Event::SwitchTheme { .. } => EventKind::Intent,
            Event::CycleModelNext => EventKind::Intent,
            Event::CycleModelPrev => EventKind::Intent,
            Event::ToggleScopedModelsDialog => EventKind::Intent,
            Event::ScopedModelToggle { .. } => EventKind::Intent,
            Event::ScopedModelEnableAll => EventKind::Intent,
            Event::ScopedModelDisableAll => EventKind::Intent,
            Event::ScopedModelToggleProvider { .. } => EventKind::Intent,
            Event::ToggleSettingsDialog => EventKind::Intent,
            Event::SettingsUp => EventKind::Intent,
            Event::SettingsDown => EventKind::Intent,
            Event::SettingsLeft => EventKind::Intent,
            Event::SettingsRight => EventKind::Intent,
            Event::SettingsSelect => EventKind::Intent,
            Event::SettingsClose => EventKind::Intent,
            Event::SettingsSwitchCategory { .. } => EventKind::Intent,
            Event::CycleThinkingLevel => EventKind::Intent,
            Event::SetThinkingLevel(_) => EventKind::Intent,
            Event::ToggleReadOnly => EventKind::Intent,
            Event::TrustProject => EventKind::Intent,
            Event::UntrustProject => EventKind::Intent,
            Event::ReloadAll => EventKind::Intent,
            Event::PermissionResponse { .. } => EventKind::Intent,
            Event::Up => EventKind::Intent,
            Event::Down => EventKind::Intent,
            Event::ForkSession { .. } => EventKind::Intent,
            Event::CloneSession => EventKind::Intent,
            Event::ToggleSessionTree => EventKind::Intent,
            Event::SessionTreeFilterCycle => EventKind::Intent,
            Event::SessionTreeSelect { .. } => EventKind::Intent,
            Event::TransientMessage { .. } => EventKind::Intent,
            Event::TransientError { .. } => EventKind::Intent,
            Event::ClearTransient => EventKind::Intent,
            Event::ShowDiagnostics => EventKind::Intent,
            // Fact variants
            Event::Thinking { .. } => EventKind::Fact,
            Event::ThoughtDone { .. } => EventKind::Fact,
            Event::ToolStart { .. } => EventKind::Fact,
            Event::ToolInputDelta { .. } => EventKind::Fact,
            Event::ToolEnd { .. } => EventKind::Fact,
            Event::ToolConstraintError { .. } => EventKind::Fact,
            Event::ResponseDelta { .. } => EventKind::Fact,
            Event::ThinkingDelta { .. } => EventKind::Fact,
            Event::TextStart { .. } => EventKind::Fact,
            Event::TextEnd { .. } => EventKind::Fact,
            Event::ThinkingStart { .. } => EventKind::Fact,
            Event::ThinkingEnd { .. } => EventKind::Fact,
            Event::Response { .. } => EventKind::Fact,
            Event::TurnComplete { .. } => EventKind::Fact,
            Event::Done { .. } => EventKind::Fact,
            Event::Error { .. } => EventKind::Fact,
            Event::TurnStarted { .. } => EventKind::Fact,
            Event::TurnAborted => EventKind::Fact,
            Event::TurnCompleted => EventKind::Fact,
            Event::TurnErrored { .. } => EventKind::Fact,
            Event::TurnConstraintError { .. } => EventKind::Fact,
            Event::TokenStatsUpdated { .. } => EventKind::Fact,
            Event::CompactionTriggered { .. } => EventKind::Fact,
            Event::StreamStarted { .. } => EventKind::Fact,
            Event::UserMessageSubmitted { .. } => EventKind::Fact,
            Event::QueueAborted { .. } => EventKind::Fact,
            Event::QueuesCleared => EventKind::Fact,
            Event::SteeringDelivered { .. } => EventKind::Fact,
            Event::FollowUpDelivered { .. } => EventKind::Fact,
            Event::MessageDequeued { .. } => EventKind::Fact,
            Event::IdGenerated(_) => EventKind::Fact,
            Event::AssistantMessageReady { .. } => EventKind::Fact,
            Event::GistShared { .. } => EventKind::Fact,
            Event::ExternalEditorClosed { .. } => EventKind::Fact,
            Event::ClipboardWritten { .. } => EventKind::Fact,
            Event::ClipboardRead { .. } => EventKind::Fact,
            Event::ProcessResumed => EventKind::Fact,
            Event::BashOutput { .. } => EventKind::Fact,
            Event::FilesWritten { .. } => EventKind::Fact,
            Event::EnvDetected { .. } => EventKind::Fact,
            Event::FffSearchResult { .. } => EventKind::Fact,
            Event::SkillsLoaded { .. } => EventKind::Fact,
            Event::AuthLoaded { .. } => EventKind::Fact,
            Event::ValidationFailed { .. } => EventKind::Fact,
            Event::ModelsFetched { .. } => EventKind::Fact,
            Event::KeybindingsReloaded => EventKind::Fact,
            Event::MessageReplayed { .. } => EventKind::Fact,
            Event::PermissionRequest { .. } => EventKind::Fact,
            Event::PermissionRequestDismissed => EventKind::Fact,
            Event::InputChanged { .. } => EventKind::Fact,
            Event::ViewChanged { .. } => EventKind::Fact,
            Event::CompletionChanged { .. } => EventKind::Fact,
            Event::TrustLoaded { .. } => EventKind::Fact,
            Event::TrustChanged { .. } => EventKind::Fact,
            Event::TrustSet { .. } => EventKind::Fact,
            Event::ReadOnlyChanged { .. } => EventKind::Fact,
            Event::HistoryLoaded { .. } => EventKind::Fact,
            Event::HistoryAppend { .. } => EventKind::Fact,
            Event::SessionLoaded { .. } => EventKind::Fact,
            Event::SessionSaved { .. } => EventKind::Fact,
            Event::SessionDeleted { .. } => EventKind::Fact,
            Event::SessionImported { .. } => EventKind::Fact,
            Event::SessionExported { .. } => EventKind::Fact,
            Event::SessionList { .. } => EventKind::Fact,
            Event::SessionOperationFailed { .. } => EventKind::Fact,
            Event::SessionChanged { .. } => EventKind::Fact,
            Event::SessionTreeSnapshot { .. } => EventKind::Fact,
            Event::SystemMessage { .. } => EventKind::Fact,
            Event::ConfigLoaded { .. } => EventKind::Fact,
            // Control variants
            Event::Quit => EventKind::Control,
            Event::ForceQuit => EventKind::Control,
            Event::Reset => EventKind::Control,
            Event::Abort => EventKind::Control,
            Event::ClearQueues => EventKind::Control,
            Event::FollowUp => EventKind::Control,
            Event::ToggleExpand => EventKind::Control,
            Event::Dequeue => EventKind::Control,
            Event::OpenExternalEditor => EventKind::Control,
            Event::ExternalEditorDone { .. } => EventKind::Control,
            Event::ShareSession => EventKind::Control,
            Event::Suspend => EventKind::Control,
            Event::ToggleVimMode => EventKind::Control,
            Event::CopyLastResponse => EventKind::Control,
            Event::OpenSessionList => EventKind::Control,
            Event::NewSession => EventKind::Control,
            Event::ResumeSession => EventKind::Control,
            Event::SelectSession { .. } => EventKind::Control,
            Event::StarSession { .. } => EventKind::Control,
            Event::RenameSession { .. } => EventKind::Control,
            Event::DeleteSession { .. } => EventKind::Control,
        }
    }

    /// Return the [`EventCategory`] for this event variant.
    pub fn category(&self) -> EventCategory {
        match self {
            Event::Thinking { .. } => EventCategory::Agent,
            Event::ThoughtDone { .. } => EventCategory::Agent,
            Event::ToolStart { .. } => EventCategory::Agent,
            Event::ToolInputDelta { .. } => EventCategory::Agent,
            Event::ToolEnd { .. } => EventCategory::Agent,
            Event::ToolConstraintError { .. } => EventCategory::Agent,
            Event::ResponseDelta { .. } => EventCategory::Agent,
            Event::ThinkingDelta { .. } => EventCategory::Agent,
            Event::TextStart { .. } => EventCategory::Agent,
            Event::TextEnd { .. } => EventCategory::Agent,
            Event::ThinkingStart { .. } => EventCategory::Agent,
            Event::ThinkingEnd { .. } => EventCategory::Agent,
            Event::Response { .. } => EventCategory::Agent,
            Event::TurnComplete { .. } => EventCategory::Agent,
            Event::Done { .. } => EventCategory::Agent,
            Event::Error { .. } => EventCategory::Agent,
            Event::TurnStarted { .. } => EventCategory::Agent,
            Event::TurnAborted => EventCategory::Agent,
            Event::TurnCompleted => EventCategory::Agent,
            Event::TurnErrored { .. } => EventCategory::Agent,
            Event::TurnConstraintError { .. } => EventCategory::Agent,
            Event::TokenStatsUpdated { .. } => EventCategory::Agent,
            Event::CompactionTriggered { .. } => EventCategory::Agent,
            Event::StreamStarted { .. } => EventCategory::Agent,
            Event::UserMessageSubmitted { .. } => EventCategory::Agent,
            Event::QueueAborted { .. } => EventCategory::Agent,
            Event::QueuesCleared => EventCategory::Agent,
            Event::SteeringDelivered { .. } => EventCategory::Agent,
            Event::FollowUpDelivered { .. } => EventCategory::Agent,
            Event::MessageDequeued { .. } => EventCategory::Agent,
            Event::IdGenerated(_) => EventCategory::Agent,
            Event::AssistantMessageReady { .. } => EventCategory::Agent,
            Event::SetPrompt { .. } => EventCategory::Command,
            Event::RunLoadCommand { .. } => EventCategory::Command,
            Event::RunSaveCommand { .. } => EventCategory::Command,
            Event::RunDeleteCommand { .. } => EventCategory::Command,
            Event::RunImportCommand { .. } => EventCategory::Command,
            Event::RunExportCommand { .. } => EventCategory::Command,
            Event::RunSkillCommand { .. } => EventCategory::Command,
            Event::RunLoginCommand { .. } => EventCategory::Command,
            Event::RunLogoutCommand { .. } => EventCategory::Command,
            Event::RunNameCommand { .. } => EventCategory::Command,
            Event::RunForkCommand { .. } => EventCategory::Command,
            Event::RunCompactCommand { .. } => EventCategory::Command,
            Event::RunPromptCommand { .. } => EventCategory::Command,
            Event::RunThinkingCommand { .. } => EventCategory::Command,
            Event::RunPaletteCommand { .. } => EventCategory::Command,
            Event::Quit => EventCategory::Control,
            Event::ForceQuit => EventCategory::Control,
            Event::Reset => EventCategory::Control,
            Event::Abort => EventCategory::Control,
            Event::ClearQueues => EventCategory::Control,
            Event::FollowUp => EventCategory::Control,
            Event::ToggleExpand => EventCategory::Control,
            Event::Dequeue => EventCategory::Control,
            Event::OpenExternalEditor => EventCategory::Control,
            Event::ExternalEditorDone { .. } => EventCategory::Control,
            Event::ShareSession => EventCategory::Control,
            Event::Suspend => EventCategory::Control,
            Event::ToggleVimMode => EventCategory::Control,
            Event::CopyLastResponse => EventCategory::Control,
            Event::OpenSessionList => EventCategory::Control,
            Event::NewSession => EventCategory::Control,
            Event::ResumeSession => EventCategory::Control,
            Event::SelectSession { .. } => EventCategory::Control,
            Event::StarSession { .. } => EventCategory::Control,
            Event::RenameSession { .. } => EventCategory::Control,
            Event::DeleteSession { .. } => EventCategory::Control,
            Event::ToggleWelcome => EventCategory::Dialog,
            Event::ToggleCommandPalette => EventCategory::Dialog,
            Event::PaletteFilter(_) => EventCategory::Dialog,
            Event::PaletteBackspace => EventCategory::Dialog,
            Event::PaletteUp => EventCategory::Dialog,
            Event::PaletteDown => EventCategory::Dialog,
            Event::PaletteSelect => EventCategory::Dialog,
            Event::PaletteClose => EventCategory::Dialog,
            Event::ToggleModelSelector => EventCategory::Dialog,
            Event::ModelSelectorFilter(_) => EventCategory::Dialog,
            Event::ModelSelectorBackspace => EventCategory::Dialog,
            Event::ModelSelectorUp => EventCategory::Dialog,
            Event::ModelSelectorDown => EventCategory::Dialog,
            Event::ModelSelectorSelect => EventCategory::Dialog,
            Event::ModelSelectorClose => EventCategory::Dialog,
            Event::TogglePathCompletion => EventCategory::Dialog,
            Event::PathCompletionUp => EventCategory::Dialog,
            Event::PathCompletionDown => EventCategory::Dialog,
            Event::PathCompletionSelect => EventCategory::Dialog,
            Event::PathCompletionClose => EventCategory::Dialog,
            Event::CommandFormInput(_) => EventCategory::Dialog,
            Event::CommandFormBackspace => EventCategory::Dialog,
            Event::CommandFormUp => EventCategory::Dialog,
            Event::CommandFormDown => EventCategory::Dialog,
            Event::CommandFormSubmit => EventCategory::Dialog,
            Event::CommandFormClose => EventCategory::Dialog,
            Event::DialogBack => EventCategory::Dialog,
            Event::ProvidersDialog => EventCategory::Dialog,
            Event::ProvidersSelectModel { .. } => EventCategory::Dialog,
            Event::ProvidersDisconnect { .. } => EventCategory::Dialog,
            Event::ProvidersAdd => EventCategory::Dialog,
            Event::ProvidersEditModels { .. } => EventCategory::Dialog,
            Event::CopyToClipboard(_) => EventCategory::Dialog,
            Event::CopySelectedBlock => EventCategory::Dialog,
            Event::CopyBlockMetadata => EventCategory::Dialog,
            Event::AtFilePicker => EventCategory::Dialog,
            Event::InsertAtRef(_) => EventCategory::Dialog,
            Event::PendingEdit { .. } => EventCategory::Edit,
            Event::ApproveEdit => EventCategory::Edit,
            Event::RejectEdit => EventCategory::Edit,
            Event::GistShared { .. } => EventCategory::IO,
            Event::ExternalEditorClosed { .. } => EventCategory::IO,
            Event::ClipboardWritten { .. } => EventCategory::IO,
            Event::ClipboardRead { .. } => EventCategory::IO,
            Event::ProcessResumed => EventCategory::IO,
            Event::BashOutput { .. } => EventCategory::IO,
            Event::FilesWritten { .. } => EventCategory::IO,
            Event::EnvDetected { .. } => EventCategory::IO,
            Event::FffSearchResult { .. } => EventCategory::IO,
            Event::SkillsLoaded { .. } => EventCategory::IO,
            Event::AuthLoaded { .. } => EventCategory::IO,
            Event::Input(_) => EventCategory::Input,
            Event::Backspace => EventCategory::Input,
            Event::Newline => EventCategory::Input,
            Event::Submit => EventCategory::Input,
            Event::Escape => EventCategory::Input,
            Event::CursorLeft => EventCategory::Input,
            Event::CursorRight => EventCategory::Input,
            Event::CursorStart => EventCategory::Input,
            Event::CursorEnd => EventCategory::Input,
            Event::DeleteWord => EventCategory::Input,
            Event::DeleteToEnd => EventCategory::Input,
            Event::DeleteToStart => EventCategory::Input,
            Event::KillChar => EventCategory::Input,
            Event::HistoryPrev => EventCategory::Input,
            Event::HistoryNext => EventCategory::Input,
            Event::Undo => EventCategory::Input,
            Event::Redo => EventCategory::Input,
            Event::CursorWordLeft => EventCategory::Input,
            Event::CursorWordRight => EventCategory::Input,
            Event::PageUp => EventCategory::Input,
            Event::PageDown => EventCategory::Input,
            Event::GoToTop => EventCategory::Input,
            Event::GoToBottom => EventCategory::Input,
            Event::Paste(_) => EventCategory::Input,
            Event::PasteImage => EventCategory::Input,
            Event::MouseClick { .. } => EventCategory::Input,
            Event::MouseRelease { .. } => EventCategory::Input,
            Event::MouseDrag { .. } => EventCategory::Input,
            Event::MouseMove { .. } => EventCategory::Input,
            Event::MouseScrollUp => EventCategory::Input,
            Event::MouseScrollDown => EventCategory::Input,
            Event::FocusGained => EventCategory::Input,
            Event::FocusLost => EventCategory::Input,
            Event::TerminalSize { .. } => EventCategory::Input,
            Event::Start => EventCategory::LoginFlow,
            Event::SelectProvider { .. } => EventCategory::LoginFlow,
            Event::SubmitKey { .. } => EventCategory::LoginFlow,
            Event::ToggleModel { .. } => EventCategory::LoginFlow,
            Event::Save => EventCategory::LoginFlow,
            Event::Cancel => EventCategory::LoginFlow,
            Event::ValidationFailed { .. } => EventCategory::LoginFlow,
            Event::ModelsFetched { .. } => EventCategory::LoginFlow,
            Event::SwitchModel { .. } => EventCategory::ModelConfig,
            Event::SwitchTheme { .. } => EventCategory::ModelConfig,
            Event::CycleModelNext => EventCategory::ModelConfig,
            Event::CycleModelPrev => EventCategory::ModelConfig,
            Event::ToggleScopedModelsDialog => EventCategory::ModelConfig,
            Event::ScopedModelToggle { .. } => EventCategory::ModelConfig,
            Event::ScopedModelEnableAll => EventCategory::ModelConfig,
            Event::ScopedModelDisableAll => EventCategory::ModelConfig,
            Event::ScopedModelToggleProvider { .. } => EventCategory::ModelConfig,
            Event::ToggleSettingsDialog => EventCategory::ModelConfig,
            Event::SettingsUp => EventCategory::ModelConfig,
            Event::SettingsDown => EventCategory::ModelConfig,
            Event::SettingsLeft => EventCategory::ModelConfig,
            Event::SettingsRight => EventCategory::ModelConfig,
            Event::SettingsSelect => EventCategory::ModelConfig,
            Event::SettingsClose => EventCategory::ModelConfig,
            Event::SettingsSwitchCategory { .. } => EventCategory::ModelConfig,
            Event::CycleThinkingLevel => EventCategory::ModelConfig,
            Event::SetThinkingLevel(_) => EventCategory::ModelConfig,
            Event::ToggleReadOnly => EventCategory::ModelConfig,
            Event::TrustProject => EventCategory::ModelConfig,
            Event::UntrustProject => EventCategory::ModelConfig,
            Event::ReloadAll => EventCategory::ModelConfig,
            Event::KeybindingsReloaded => EventCategory::ModelConfig,
            Event::MessageReplayed { .. } => EventCategory::Other,
            Event::PermissionResponse { .. } => EventCategory::Permission,
            Event::PermissionRequest { .. } => EventCategory::Permission,
            Event::PermissionRequestDismissed => EventCategory::Permission,
            Event::InputChanged { .. } => EventCategory::Persistence,
            Event::ViewChanged { .. } => EventCategory::Persistence,
            Event::CompletionChanged { .. } => EventCategory::Persistence,
            Event::TrustLoaded { .. } => EventCategory::Persistence,
            Event::TrustChanged { .. } => EventCategory::Persistence,
            Event::TrustSet { .. } => EventCategory::Persistence,
            Event::ReadOnlyChanged { .. } => EventCategory::Persistence,
            Event::HistoryLoaded { .. } => EventCategory::Persistence,
            Event::HistoryAppend { .. } => EventCategory::Persistence,
            Event::Up => EventCategory::Scroll,
            Event::Down => EventCategory::Scroll,
            Event::ForkSession { .. } => EventCategory::Session,
            Event::CloneSession => EventCategory::Session,
            Event::ToggleSessionTree => EventCategory::Session,
            Event::SessionTreeFilterCycle => EventCategory::Session,
            Event::SessionTreeSelect { .. } => EventCategory::Session,
            Event::SessionLoaded { .. } => EventCategory::Session,
            Event::SessionSaved { .. } => EventCategory::Session,
            Event::SessionDeleted { .. } => EventCategory::Session,
            Event::SessionImported { .. } => EventCategory::Session,
            Event::SessionExported { .. } => EventCategory::Session,
            Event::SessionList { .. } => EventCategory::Session,
            Event::SessionOperationFailed { .. } => EventCategory::Session,
            Event::SessionChanged { .. } => EventCategory::Session,
            Event::SessionTreeSnapshot { .. } => EventCategory::Session,
            Event::TransientMessage { .. } => EventCategory::System,
            Event::TransientError { .. } => EventCategory::System,
            Event::ClearTransient => EventCategory::System,
            Event::ShowDiagnostics => EventCategory::System,
            Event::SystemMessage { .. } => EventCategory::System,
            Event::ConfigLoaded { .. } => EventCategory::System,
        }
    }

    /// Convert this event to an intent [`Event`], if it maps to one.
    ///
    /// Returns `None` for Fact variants. Control variants like Quit, Reset, Abort
    /// are also convertible to intent.
    ///
    /// This method is kept for backward compatibility. New code should
    /// pattern-match on `Event` directly and check `kind()`.
    pub fn into_intent(self) -> Option<Event> {
        match self {
            // Intent-kind events
            Event::RunLoadCommand { name } => Some(Event::RunLoadCommand { name }),
            Event::RunSaveCommand { name } => Some(Event::RunSaveCommand { name }),
            Event::RunDeleteCommand { name } => Some(Event::RunDeleteCommand { name }),
            Event::RunImportCommand { path } => Some(Event::RunImportCommand { path }),
            Event::RunExportCommand { path } => Some(Event::RunExportCommand { path }),
            Event::RunSkillCommand { name } => Some(Event::RunSkillCommand { name }),
            Event::RunLoginCommand { provider, token } => Some(Event::RunLoginCommand { provider, token }),
            Event::RunLogoutCommand { provider } => Some(Event::RunLogoutCommand { provider }),
            Event::RunNameCommand { name } => Some(Event::RunNameCommand { name }),
            Event::RunForkCommand { message_index } => Some(Event::RunForkCommand { message_index }),
            Event::RunCompactCommand { keep, focus } => Some(Event::RunCompactCommand { keep, focus }),
            Event::RunPromptCommand { name } => Some(Event::RunPromptCommand { name }),
            Event::RunThinkingCommand { level } => Some(Event::RunThinkingCommand { level }),
            Event::RunPaletteCommand { name, args } => Some(Event::RunPaletteCommand { name, args }),
            Event::ToggleWelcome => Some(Event::ToggleWelcome),
            Event::ToggleCommandPalette => Some(Event::ToggleCommandPalette),
            Event::PaletteFilter(c) => Some(Event::PaletteFilter(c)),
            Event::PaletteBackspace => Some(Event::PaletteBackspace),
            Event::PaletteUp => Some(Event::PaletteUp),
            Event::PaletteDown => Some(Event::PaletteDown),
            Event::PaletteSelect => Some(Event::PaletteSelect),
            Event::PaletteClose => Some(Event::PaletteClose),
            Event::ToggleModelSelector => Some(Event::ToggleModelSelector),
            Event::ModelSelectorFilter(c) => Some(Event::ModelSelectorFilter(c)),
            Event::ModelSelectorBackspace => Some(Event::ModelSelectorBackspace),
            Event::ModelSelectorUp => Some(Event::ModelSelectorUp),
            Event::ModelSelectorDown => Some(Event::ModelSelectorDown),
            Event::ModelSelectorSelect => Some(Event::ModelSelectorSelect),
            Event::ModelSelectorClose => Some(Event::ModelSelectorClose),
            Event::TogglePathCompletion => Some(Event::TogglePathCompletion),
            Event::PathCompletionUp => Some(Event::PathCompletionUp),
            Event::PathCompletionDown => Some(Event::PathCompletionDown),
            Event::PathCompletionSelect => Some(Event::PathCompletionSelect),
            Event::PathCompletionClose => Some(Event::PathCompletionClose),
            Event::CommandFormInput(c) => Some(Event::CommandFormInput(c)),
            Event::CommandFormBackspace => Some(Event::CommandFormBackspace),
            Event::CommandFormUp => Some(Event::CommandFormUp),
            Event::CommandFormDown => Some(Event::CommandFormDown),
            Event::CommandFormSubmit => Some(Event::CommandFormSubmit),
            Event::CommandFormClose => Some(Event::CommandFormClose),
            Event::DialogBack => Some(Event::DialogBack),
            Event::ProvidersDialog => Some(Event::ProvidersDialog),
            Event::ProvidersSelectModel { provider, model } => Some(Event::ProvidersSelectModel { provider, model }),
            Event::ProvidersDisconnect { provider } => Some(Event::ProvidersDisconnect { provider }),
            Event::ProvidersAdd => Some(Event::ProvidersAdd),
            Event::ProvidersEditModels { provider } => Some(Event::ProvidersEditModels { provider }),
            Event::CopyToClipboard(s) => Some(Event::CopyToClipboard(s)),
            Event::CopySelectedBlock => Some(Event::CopySelectedBlock),
            Event::CopyBlockMetadata => Some(Event::CopyBlockMetadata),
            Event::AtFilePicker => Some(Event::AtFilePicker),
            Event::InsertAtRef(s) => Some(Event::InsertAtRef(s)),
            Event::PendingEdit { path, original, proposed } => Some(Event::PendingEdit { path, original, proposed }),
            Event::ApproveEdit => Some(Event::ApproveEdit),
            Event::RejectEdit => Some(Event::RejectEdit),
            Event::Input(c) => Some(Event::Input(c)),
            Event::Backspace => Some(Event::Backspace),
            Event::Newline => Some(Event::Newline),
            Event::Submit => Some(Event::Submit),
            Event::Escape => Some(Event::Escape),
            Event::CursorLeft => Some(Event::CursorLeft),
            Event::CursorRight => Some(Event::CursorRight),
            Event::CursorStart => Some(Event::CursorStart),
            Event::CursorEnd => Some(Event::CursorEnd),
            Event::DeleteWord => Some(Event::DeleteWord),
            Event::DeleteToEnd => Some(Event::DeleteToEnd),
            Event::DeleteToStart => Some(Event::DeleteToStart),
            Event::KillChar => Some(Event::KillChar),
            Event::HistoryPrev => Some(Event::HistoryPrev),
            Event::HistoryNext => Some(Event::HistoryNext),
            Event::Undo => Some(Event::Undo),
            Event::Redo => Some(Event::Redo),
            Event::CursorWordLeft => Some(Event::CursorWordLeft),
            Event::CursorWordRight => Some(Event::CursorWordRight),
            Event::PageUp => Some(Event::PageUp),
            Event::PageDown => Some(Event::PageDown),
            Event::GoToTop => Some(Event::GoToTop),
            Event::GoToBottom => Some(Event::GoToBottom),
            Event::Paste(s) => Some(Event::Paste(s)),
            Event::PasteImage => Some(Event::PasteImage),
            Event::MouseClick { row, col, button } => Some(Event::MouseClick { row, col, button }),
            Event::MouseRelease { row, col, button } => Some(Event::MouseRelease { row, col, button }),
            Event::MouseDrag { row, col, button } => Some(Event::MouseDrag { row, col, button }),
            Event::MouseMove { row, col } => Some(Event::MouseMove { row, col }),
            Event::MouseScrollUp => Some(Event::MouseScrollUp),
            Event::MouseScrollDown => Some(Event::MouseScrollDown),
            Event::FocusGained => Some(Event::FocusGained),
            Event::FocusLost => Some(Event::FocusLost),
            Event::TerminalSize { width, height } => Some(Event::TerminalSize { width, height }),
            Event::Start => Some(Event::Start),
            Event::SelectProvider { provider } => Some(Event::SelectProvider { provider }),
            Event::SubmitKey { provider, key } => Some(Event::SubmitKey { provider, key }),
            Event::ToggleModel { model } => Some(Event::ToggleModel { model }),
            Event::Save => Some(Event::Save),
            Event::Cancel => Some(Event::Cancel),
            Event::SwitchModel { provider, model, explicit } => Some(Event::SwitchModel { provider, model, explicit }),
            Event::SwitchTheme { name } => Some(Event::SwitchTheme { name }),
            Event::CycleModelNext => Some(Event::CycleModelNext),
            Event::CycleModelPrev => Some(Event::CycleModelPrev),
            Event::ToggleScopedModelsDialog => Some(Event::ToggleScopedModelsDialog),
            Event::ScopedModelToggle { provider, name } => Some(Event::ScopedModelToggle { provider, name }),
            Event::ScopedModelEnableAll => Some(Event::ScopedModelEnableAll),
            Event::ScopedModelDisableAll => Some(Event::ScopedModelDisableAll),
            Event::ScopedModelToggleProvider { provider } => Some(Event::ScopedModelToggleProvider { provider }),
            Event::ToggleSettingsDialog => Some(Event::ToggleSettingsDialog),
            Event::SettingsUp => Some(Event::SettingsUp),
            Event::SettingsDown => Some(Event::SettingsDown),
            Event::SettingsLeft => Some(Event::SettingsLeft),
            Event::SettingsRight => Some(Event::SettingsRight),
            Event::SettingsSelect => Some(Event::SettingsSelect),
            Event::SettingsClose => Some(Event::SettingsClose),
            Event::SettingsSwitchCategory { category } => Some(Event::SettingsSwitchCategory { category }),
            Event::CycleThinkingLevel => Some(Event::CycleThinkingLevel),
            Event::SetThinkingLevel(lvl) => Some(Event::SetThinkingLevel(lvl)),
            Event::ToggleReadOnly => Some(Event::ToggleReadOnly),
            Event::TrustProject => Some(Event::TrustProject),
            Event::UntrustProject => Some(Event::UntrustProject),
            Event::ReloadAll => Some(Event::ReloadAll),
            Event::PermissionResponse { request_id, action } => Some(Event::PermissionResponse { request_id, action }),
            Event::Up => Some(Event::Up),
            Event::Down => Some(Event::Down),
            Event::ForkSession { message_index } => Some(Event::ForkSession { message_index }),
            Event::CloneSession => Some(Event::CloneSession),
            Event::ToggleSessionTree => Some(Event::ToggleSessionTree),
            Event::SessionTreeFilterCycle => Some(Event::SessionTreeFilterCycle),
            Event::SessionTreeSelect { id } => Some(Event::SessionTreeSelect { id }),
            Event::TransientMessage { content, level } => Some(Event::TransientMessage { content, level }),
            Event::ClearTransient => Some(Event::ClearTransient),
            Event::ShowDiagnostics => Some(Event::ShowDiagnostics),
            Event::SetPrompt { name } => Some(Event::SetPrompt { name }),
            // Control variants that are also intents
            Event::Quit => Some(Event::Quit),
            Event::ForceQuit => Some(Event::ForceQuit),
            Event::Reset => Some(Event::Reset),
            Event::Abort => Some(Event::Abort),
            Event::ClearQueues => Some(Event::ClearQueues),
            Event::FollowUp => Some(Event::FollowUp),
            Event::ToggleExpand => Some(Event::ToggleExpand),
            Event::Dequeue => Some(Event::Dequeue),
            Event::OpenExternalEditor => Some(Event::OpenExternalEditor),
            Event::ExternalEditorDone { content } => Some(Event::ExternalEditorDone { content }),
            Event::ShareSession => Some(Event::ShareSession),
            Event::Suspend => Some(Event::Suspend),
            Event::ToggleVimMode => Some(Event::ToggleVimMode),
            Event::CopyLastResponse => Some(Event::CopyLastResponse),
            Event::OpenSessionList => Some(Event::OpenSessionList),
            Event::NewSession => Some(Event::NewSession),
            Event::ResumeSession => Some(Event::ResumeSession),
            Event::SelectSession { id } => Some(Event::SelectSession { id }),
            Event::StarSession { id } => Some(Event::StarSession { id }),
            Event::RenameSession { id, name } => Some(Event::RenameSession { id, name }),
            Event::DeleteSession { id } => Some(Event::DeleteSession { id }),
            // Fact variants - return None
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// is_fact_variant — fast-path predicate
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if this event is a fact (not an intent or control).
pub fn is_fact_variant(e: &Event) -> bool {
    matches!(
        e,
        Event::Thinking { .. }
            | Event::ThoughtDone { .. }
            | Event::ToolStart { .. }
            | Event::ToolInputDelta { .. }
            | Event::ToolEnd { .. }
            | Event::ToolConstraintError { .. }
            | Event::ResponseDelta { .. }
            | Event::ThinkingDelta { .. }
            | Event::TextStart { .. }
            | Event::TextEnd { .. }
            | Event::ThinkingStart { .. }
            | Event::ThinkingEnd { .. }
            | Event::Response { .. }
            | Event::TurnComplete { .. }
            | Event::Done { .. }
            | Event::Error { .. }
            | Event::TurnStarted { .. }
            | Event::TurnAborted
            | Event::TurnCompleted
            | Event::TurnErrored { .. }
            | Event::TurnConstraintError { .. }
            | Event::TokenStatsUpdated { .. }
            | Event::StreamStarted { .. }
            | Event::UserMessageSubmitted { .. }
            | Event::QueueAborted { .. }
            | Event::QueuesCleared
            | Event::SteeringDelivered { .. }
            | Event::FollowUpDelivered { .. }
            | Event::MessageDequeued { .. }
            | Event::IdGenerated(_)
            | Event::AssistantMessageReady { .. }
            | Event::GistShared { .. }
            | Event::ExternalEditorClosed { .. }
            | Event::ClipboardWritten { .. }
            | Event::ClipboardRead { .. }
            | Event::ProcessResumed
            | Event::BashOutput { .. }
            | Event::FilesWritten { .. }
            | Event::EnvDetected { .. }
            | Event::FffSearchResult { .. }
            | Event::MessageReplayed { .. }
            | Event::InputChanged { .. }
            | Event::ViewChanged { .. }
            | Event::CompletionChanged { .. }
            | Event::TrustLoaded { .. }
            | Event::TrustChanged { .. }
            | Event::TrustSet { .. }
            | Event::ReadOnlyChanged { .. }
            | Event::HistoryLoaded { .. }
            | Event::HistoryAppend { .. }
            | Event::TransientMessage { .. }
            | Event::TransientError { .. }
            | Event::ClearTransient
            | Event::ShowDiagnostics
            | Event::SystemMessage { .. }
            | Event::ConfigLoaded { .. }
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// EVENT_NAMES — zero-arg constructor table for bindable variants
// ─────────────────────────────────────────────────────────────────────────────

/// Zero-argument event constructor signature.
pub type EventCtor = fn() -> Event;

/// Bindable event names paired with their zero-arg constructors.
///
/// These are events that can be constructed with no arguments, used for
/// keybinding resolution and command palette lookup.
pub const EVENT_NAMES: &[(&str, EventCtor)] = &[
    ("ToggleWelcome", || Event::ToggleWelcome),
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
    ("TogglePathCompletion", || Event::TogglePathCompletion),
    ("PathCompletionUp", || Event::PathCompletionUp),
    ("PathCompletionDown", || Event::PathCompletionDown),
    ("PathCompletionSelect", || Event::PathCompletionSelect),
    ("PathCompletionClose", || Event::PathCompletionClose),
    ("CommandFormBackspace", || Event::CommandFormBackspace),
    ("CommandFormUp", || Event::CommandFormUp),
    ("CommandFormDown", || Event::CommandFormDown),
    ("CommandFormSubmit", || Event::CommandFormSubmit),
    ("CommandFormClose", || Event::CommandFormClose),
    ("DialogBack", || Event::DialogBack),
    ("ProvidersDialog", || Event::ProvidersDialog),
    ("ProvidersAdd", || Event::ProvidersAdd),
    ("CopySelectedBlock", || Event::CopySelectedBlock),
    ("CopyBlockMetadata", || Event::CopyBlockMetadata),
    ("AtFilePicker", || Event::AtFilePicker),
    ("ApproveEdit", || Event::ApproveEdit),
    ("RejectEdit", || Event::RejectEdit),
    ("Backspace", || Event::Backspace),
    ("Newline", || Event::Newline),
    ("Submit", || Event::Submit),
    ("Escape", || Event::Escape),
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
    ("PageUp", || Event::PageUp),
    ("PageDown", || Event::PageDown),
    ("GoToTop", || Event::GoToTop),
    ("GoToBottom", || Event::GoToBottom),
    ("PasteImage", || Event::PasteImage),
    ("MouseScrollUp", || Event::MouseScrollUp),
    ("MouseScrollDown", || Event::MouseScrollDown),
    ("FocusGained", || Event::FocusGained),
    ("FocusLost", || Event::FocusLost),
    ("Start", || Event::Start),
    ("Save", || Event::Save),
    ("Cancel", || Event::Cancel),
    ("CycleModelNext", || Event::CycleModelNext),
    ("CycleModelPrev", || Event::CycleModelPrev),
    ("ToggleScopedModelsDialog", || Event::ToggleScopedModelsDialog),
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
    ("ReloadAll", || Event::ReloadAll),
    ("Up", || Event::Up),
    ("Down", || Event::Down),
    ("CloneSession", || Event::CloneSession),
    ("ToggleSessionTree", || Event::ToggleSessionTree),
    ("SessionTreeFilterCycle", || Event::SessionTreeFilterCycle),
    ("ClearTransient", || Event::ClearTransient),
    ("ShowDiagnostics", || Event::ShowDiagnostics),
    ("Quit", || Event::Quit),
    ("ForceQuit", || Event::ForceQuit),
    ("Reset", || Event::Reset),
    ("Abort", || Event::Abort),
    ("ClearQueues", || Event::ClearQueues),
    ("FollowUp", || Event::FollowUp),
    ("ToggleExpand", || Event::ToggleExpand),
    ("Dequeue", || Event::Dequeue),
    ("OpenExternalEditor", || Event::OpenExternalEditor),
    ("ShareSession", || Event::ShareSession),
    ("Suspend", || Event::Suspend),
    ("ToggleVimMode", || Event::ToggleVimMode),
    ("CopyLastResponse", || Event::CopyLastResponse),
    ("OpenSessionList", || Event::OpenSessionList),
    ("NewSession", || Event::NewSession),
    ("ResumeSession", || Event::ResumeSession),
];

// ─────────────────────────────────────────────────────────────────────────────
// Helper constructors for variants with optional fields
// ─────────────────────────────────────────────────────────────────────────────

impl Event {
    /// Create a Response with default durable fields.
    pub fn response(id: impl Into<String>, content: impl Into<String>) -> Self {
        Event::Response {
            id: id.into(),
            content: content.into(),
            role: String::new(),
            timestamp: 0.0,
            provider: String::new(),
        }
    }

    /// Create a ToolEnd with default input field.
    pub fn tool_end(id: impl Into<String>, duration_secs: f64, output: impl Into<String>) -> Self {
        Event::ToolEnd {
            id: id.into(),
            input: None,
            duration_secs,
            output: output.into(),
        }
    }
}
