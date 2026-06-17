//! Flat `Event` enum — every leaf variant lives at the top level.
//!
//! Sub-enums are reduced to type aliases for backward compatibility, so old
//! code such as `InputEvent::Submit` still resolves to `Event::Submit`.

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use super::EVENT_NAMES;
use crate::model::ThinkingLevel;
use crate::orchestrator::{OrchestratorPlan, SubagentTask, TaskStatus};
use crate::orchestrator_actor::OrchestratorState;
use crate::settings::SettingsCategory;
use crate::state::{AgentEntry, AgentStatus};

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
    MouseClick { row: u16, col: u16, button: String },
    MouseRelease { row: u16, col: u16, button: String },
    MouseDrag { row: u16, col: u16, button: String },
    MouseMove { row: u16, col: u16 },
    MouseScrollUp,
    MouseScrollDown,
    FocusGained,
    FocusLost,
    TerminalSize { width: u16, height: u16 },

    // Agent
    Thinking { id: String },
    ThoughtDone { id: String },
    ToolStart { id: String, name: String, input: serde_json::Value },
    ToolEnd { id: String, duration_secs: f64, output: String },
    ResponseDelta { id: String, content: String },
    Response { id: String, content: String },
    TurnComplete { id: String, duration_secs: f64 },
    Done { id: String },
    Error { id: String, message: String },

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
    Reset,
    Abort,
    FollowUp,
    SpawnAgent { prompt: String },
    SteerAgent { agent_id: String, message: String },
    CancelAgent { agent_id: String },
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

    // ModelConfig
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
    SettingsSwitchCategory { category: SettingsCategory },
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
    RunSaveCommand { name: String },
    DialogBack,
    ProvidersDialog,
    ProvidersSelectModel { provider: String, model: String },
    ProvidersDisconnect { provider: String },
    ProvidersAdd,
    OpenAgentsManager,
    AgentsManagerSetField { name: String, field: String, value: String },
    AgentsManagerSave { name: String },
    AgentsManagerDelete { name: String },
    CopyToClipboard(String),
    CopySelectedBlock,
    CopyBlockMetadata,
    AtFilePicker,
    InsertAtRef(String),

    // Edit
    PendingEdit { path: String, original: String, proposed: String },
    ApproveEdit,
    RejectEdit,

    // System
    SystemMessage { content: String },
    TransientMessage { content: String, level: super::TransientLevel },
    TransientError { content: String },
    ClearTransient,
    ShowDiagnostics,

    // Session
    ForkSession { message_index: usize },
    CloneSession,
    ToggleSessionTree,
    SessionTreeFilterCycle,
    SessionTreeSelect { id: String },

    // Command
    RunLoadCommand { name: String },
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

    // LoginFlow
    Start,
    SelectProvider { provider: String },
    SubmitKey { provider: String, key: String },
    ValidationDone { provider: String, key: String, models: Vec<String> },
    ValidationFailed { provider: String, key: String, error: String },
    ModelsFetched { provider: String, key: String, models: Vec<String> },
    ToggleModel { model: String },
    Save,
    Cancel,

    // Sidebar
    Show,
    Hide,
    FocusOrchestrator,
    FocusSubagent(usize),
    UpdateStatus { id: String, status: AgentStatus },
    SetSubagents(Vec<AgentEntry>),
    SetOrchestratorStatus(AgentStatus),

    // Orchestrator
    StateChanged { from: Box<OrchestratorState>, to: Box<OrchestratorState> },
    PlanStarted,
    PlanningStarted,
    PlanGenerated { plan: Box<OrchestratorPlan> },
    PlanningFailed { error: String },
    SubagentDispatched { task: Box<SubagentTask> },
    SubagentStatusChanged { task_id: String, status: TaskStatus },
    SubagentCompleted { task_id: String, output: String },
    SubagentFailed { task_id: String, error: String },
    SynthesisStarted,
    SynthesisComplete { response: String, elapsed_secs: f64 },
    Finished { success: bool },
    Cancelled,
}

impl Event {
    /// Convert this event to a durable core event for JSONL persistence.
    /// Returns `None` for transient-only events (keystrokes, scroll, streaming deltas).
    pub fn to_durable(&self) -> Option<super::DurableCoreEvent> {
        use super::DurableCoreEvent;
        match self {
            Event::ResponseDelta { .. } => None,
            Event::Response { id, content } => Some(DurableCoreEvent::MessageSent {
                id: id.clone(),
                role: "assistant".into(),
                content: content.clone(),
                timestamp: crate::model::now(),
                provider: String::new(),
            }),
            Event::ToolStart { id, name, input } => Some(DurableCoreEvent::ToolCalled {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            Event::ToolEnd { id, output, .. } => Some(DurableCoreEvent::ToolResult {
                id: id.clone(),
                output: output.clone(),
                success: true,
            }),
            Event::SwitchModel { provider, model } => Some(DurableCoreEvent::ModelSwitched {
                provider: provider.clone(),
                model: model.clone(),
            }),
            Event::RunNameCommand { name } => Some(DurableCoreEvent::SessionRenamed { name: name.clone() }),
            _ => None,
        }
    }

    /// Canonical string name for bindable variants (those in EVENT_NAMES).
    pub fn name(&self) -> Option<&'static str> {
        match self {
            Event::Backspace | Event::Newline | Event::Submit | Event::Escape | Event::CursorLeft | Event::CursorRight |
            Event::CursorStart | Event::CursorEnd | Event::DeleteWord | Event::DeleteToEnd | Event::DeleteToStart | Event::KillChar |
            Event::HistoryPrev | Event::HistoryNext | Event::Undo | Event::Redo | Event::CursorWordLeft | Event::CursorWordRight |
            Event::PageUp | Event::PageDown | Event::GoToTop | Event::GoToBottom | Event::PasteImage | Event::FocusGained |
            Event::FocusLost | Event::Quit | Event::Reset | Event::Abort | Event::FollowUp | Event::ToggleExpand |
            Event::Dequeue | Event::OpenExternalEditor | Event::Suspend | Event::ShareSession | Event::ToggleVimMode | Event::CopyLastResponse |
            Event::OpenSessionList | Event::NewSession | Event::ResumeSession | Event::CopySelectedBlock | Event::CopyBlockMetadata | Event::ToggleCommandPalette |
            Event::PaletteBackspace | Event::PaletteUp | Event::PaletteDown | Event::PaletteSelect | Event::PaletteClose | Event::ToggleModelSelector |
            Event::ModelSelectorBackspace | Event::ModelSelectorUp | Event::ModelSelectorDown | Event::ModelSelectorSelect | Event::ModelSelectorClose | Event::ToggleSettingsDialog |
            Event::SettingsUp | Event::SettingsDown | Event::SettingsLeft | Event::SettingsRight | Event::SettingsSelect | Event::SettingsClose |
            Event::CommandFormBackspace | Event::CommandFormUp | Event::CommandFormDown | Event::CommandFormSubmit | Event::CommandFormClose | Event::ToggleScopedModelsDialog |
            Event::ScopedModelEnableAll | Event::ScopedModelDisableAll | Event::DialogBack | Event::TogglePathCompletion | Event::PathCompletionUp | Event::PathCompletionDown |
            Event::PathCompletionSelect | Event::PathCompletionClose | Event::ProvidersDialog | Event::ProvidersAdd | Event::AtFilePicker | Event::CycleModelNext |
            Event::CycleModelPrev | Event::CycleThinkingLevel | Event::ToggleReadOnly | Event::TrustProject | Event::UntrustProject | Event::OpenAgentsManager |
            Event::ClearTransient => Some(<&str>::from(self.clone())),
            _ => None,
        }
    }

    /// Build an Event from its canonical name. Supports `Input:<char>` prefix.
    pub fn from_name(name: &str) -> Option<Event> {
        if let Some(rest) = name.strip_prefix("Input:") {
            let c = rest.chars().next()?;
            return Some(Event::Input(c));
        }
        for (n, ctor) in EVENT_NAMES {
            if *n == name {
                return Some(ctor());
            }
        }
        None
    }
}

// ── Convenience constructors ───────────────────────────────────────────────────

impl Event {
    pub fn input(c: char) -> Self {
        Event::Input(c)
    }
    pub fn backspace() -> Self {
        Event::Backspace
    }
    pub fn newline() -> Self {
        Event::Newline
    }
    pub fn submit() -> Self {
        Event::Submit
    }
    pub fn scroll_up() -> Self {
        Event::Up
    }
    pub fn scroll_down() -> Self {
        Event::Down
    }
    pub fn page_up() -> Self {
        Event::PageUp
    }
    pub fn page_down() -> Self {
        Event::PageDown
    }
    pub fn go_to_top() -> Self {
        Event::GoToTop
    }
    pub fn go_to_bottom() -> Self {
        Event::GoToBottom
    }
    pub fn quit() -> Self {
        Event::Quit
    }
    pub fn reset() -> Self {
        Event::Reset
    }
    pub fn abort() -> Self {
        Event::Abort
    }
    pub fn switch_model(provider: String, model: String) -> Self {
        Event::SwitchModel { provider, model }
    }
    pub fn switch_theme(name: String) -> Self {
        Event::SwitchTheme { name }
    }
    pub fn agent_thinking(id: String) -> Self {
        Event::Thinking { id }
    }
    pub fn agent_thought_done(id: String) -> Self {
        Event::ThoughtDone { id }
    }
    pub fn agent_tool_start(id: String, name: String, input: serde_json::Value) -> Self {
        Event::ToolStart { id, name, input }
    }
    pub fn agent_tool_end(id: String, duration_secs: f64, output: String) -> Self {
        Event::ToolEnd { id, duration_secs, output }
    }
    pub fn agent_response(id: String, content: String) -> Self {
        Event::Response { id, content }
    }
    pub fn agent_turn_complete(id: String, duration_secs: f64) -> Self {
        Event::TurnComplete { id, duration_secs }
    }
    pub fn agent_done(id: String) -> Self {
        Event::Done { id }
    }
    pub fn agent_error(id: String, message: String) -> Self {
        Event::Error { id, message }
    }

    pub fn paste(s: String) -> Self {
        Event::Paste(s)
    }
    pub fn set_thinking_level(level: ThinkingLevel) -> Self {
        Event::SetThinkingLevel(level)
    }
    pub fn palette_select() -> Self {
        Event::PaletteSelect
    }
    pub fn palette_filter(c: char) -> Self {
        Event::PaletteFilter(c)
    }
    pub fn palette_close() -> Self {
        Event::PaletteClose
    }
    pub fn palette_down() -> Self {
        Event::PaletteDown
    }
    pub fn settings_close() -> Self {
        Event::SettingsClose
    }
    pub fn show_diagnostics() -> Self {
        Event::ShowDiagnostics
    }
    pub fn dialog(event: Event) -> Self {
        event
    }
    pub fn toggle_command_palette() -> Self {
        Event::ToggleCommandPalette
    }
    pub fn dialog_back() -> Self {
        Event::DialogBack
    }
}

#[cfg(test)]
mod size_tests {
    use super::Event;

    /// Pre-optimization size of `Event` before boxing large orchestrator payloads.
    const EVENT_BASELINE_SIZE: usize = 288;

    #[test]
    fn event_size_reduced() {
        let size = std::mem::size_of::<Event>();
        assert!(
            size < EVENT_BASELINE_SIZE,
            "Event size {} should be smaller than baseline {}",
            size,
            EVENT_BASELINE_SIZE
        );
    }
}

