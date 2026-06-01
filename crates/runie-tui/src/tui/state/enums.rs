//! Enum types for TUI state management.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PermissionMode {
    #[default]
    Normal,
    AutoApprove,
    Plan,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TuiMode {
    Chat,
    Overlay,
    Select,
    Permission,
    CommandPalette,
    DiffViewer,
    SessionTree,
    Onboarding,
    HomeScreen,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Submit,
    TextareaKey(KeyEvent),
    InsertNewline,
    Quit,
    ToggleSidebar,
    ToggleThoughts,
    OpenCommandPalette,
    CloseModal,
    ConfirmModal,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    PermissionConfirm,
    PermissionCancel,
    PermissionAlways,
    PermissionSkip,
    CommandPaletteFilter(char),
    CommandPaletteBackspace,
    CommandPaletteUp,
    CommandPaletteDown,
    CommandPaletteConfirm,
    CommandPaletteCancelArgument,
    AgentEvent(AgentEvent),
    Tick,
    CursorBlink,
    SlashCommand(runie_core::slash_command::SlashCommand),
    ToggleSessionTree,
    SessionTreeUp,
    SessionTreeDown,
    SessionTreeConfirm,
    OnboardingNext,
    OnboardingBack,
    OnboardingNavigateUp,
    OnboardingNavigateDown,
    OnboardingSelectProvider(usize),
    OnboardingSelectModel(usize),
    OnboardingKeyInput(char),
    OnboardingKeyBackspace,
    OnboardingSearchInput(char),
    OnboardingSearchBackspace,
    OnboardingSubmit,
    OnboardingSkip,
    PermissionTimeout,
    SelectUp,
    SelectDown,
    SelectConfirm,
    SelectToggleDetails,
    ClearInput,
    ClearInputConfirm,
    ClearChat,
    DirectCommand(PaletteCommand),
    Paste(String),
    ModelsFetched(Vec<ModelInfo>),
    ModelsFetchFailed(String),
    Resize(u16, u16),
    Stop,
    SwitchModel,
    SetGitInfo { repo: String, branch: String, path: String },
    SetTopBarMockChecks {
        checks_passed: Option<usize>,
        checks_total: Option<usize>,
        percentage: Option<f32>,
        context_badges: Vec<String>,
    },
    SetTopBarRealChecks { context_badges: Vec<String> },
    SetInputRightInfo(String),
    EnterOnboarding,
    SetCurrentModel(Option<String>),
    SetMockMode(bool),
    ResetAgentState,
    UpdateTopBarContext { model: String, context_window: Option<usize>, estimated_tokens: Option<usize> },
    HistoryUp,
    HistoryDown,
    HistorySearchStart,
    HistorySearchQuery(char),
    HistorySearchBackspace,
    HistorySearchNext,
    HistorySearchPrev,
    HistorySearchCancel,
    HistorySearchConfirm,
    CopyLastResponse,
    Interject,
    SlashMenuUp,
    SlashMenuDown,
    SlashMenuConfirm,
    CloseSlashMenu,
    OpenShortcutsPanel,
    CloseShortcutsPanel,
    ShortcutsPanelUp,
    ShortcutsPanelDown,
    ShortcutsPanelToggleSection,
    ShortcutsPanelToggleFilter,
    ShortcutsPanelFilterInput(char),
    ShortcutsPanelFilterBackspace,
    OpenSettingsModal,
    CloseSettingsModal,
    SettingsModalUp,
    SettingsModalDown,
    SettingsModalNextTab,
    SettingsModalPrevTab,
    SettingsModalSelect,
    HomeScreenUp,
    HomeScreenDown,
    HomeScreenSelect,
    CloseHomeScreen,
    FilePickerUp,
    FilePickerDown,
    FilePickerConfirm,
    FilePickerFilter(char),
    FilePickerBackspace,
    CloseFilePicker,
    TogglePermissionMode,
    ClearAlwaysApprove,
    PlanModeApprove,
    PlanModeDeny,
    PlanModeViewNext,
    PlanModeViewPrev,
    ToggleScrollFocus,
    OpenContextUsageModal,
    CloseContextUsageModal,
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        // Unit variants - same discriminant means equal
        if self.is_unit_variant() {
            return std::mem::discriminant(self) == std::mem::discriminant(other);
        }
        // Data-carrying variants
        match (self, other) {
            (Msg::TextareaKey(a), Msg::TextareaKey(b)) => a == b,
            (Msg::CommandPaletteFilter(a), Msg::CommandPaletteFilter(b)) => a == b,
            (Msg::OnboardingSelectProvider(a), Msg::OnboardingSelectProvider(b)) => a == b,
            (Msg::OnboardingSelectModel(a), Msg::OnboardingSelectModel(b)) => a == b,
            (Msg::OnboardingKeyInput(a), Msg::OnboardingKeyInput(b)) => a == b,
            (Msg::OnboardingSearchInput(a), Msg::OnboardingSearchInput(b)) => a == b,
            (Msg::DirectCommand(a), Msg::DirectCommand(b)) => a == b,
            (Msg::Paste(a), Msg::Paste(b)) => a == b,
            (Msg::ModelsFetched(a), Msg::ModelsFetched(b)) => a == b,
            (Msg::ModelsFetchFailed(a), Msg::ModelsFetchFailed(b)) => a == b,
            (Msg::Resize(a_w, a_h), Msg::Resize(b_w, b_h)) => a_w == b_w && a_h == b_h,
            (Msg::SetGitInfo { .. }, Msg::SetGitInfo { .. }) => true,
            (Msg::SetTopBarMockChecks { .. }, Msg::SetTopBarMockChecks { .. }) => true,
            (Msg::SetTopBarRealChecks { .. }, Msg::SetTopBarRealChecks { .. }) => true,
            (Msg::SetInputRightInfo(a), Msg::SetInputRightInfo(b)) => a == b,
            (Msg::SetCurrentModel(a), Msg::SetCurrentModel(b)) => a == b,
            (Msg::SetMockMode(a), Msg::SetMockMode(b)) => a == b,
            (Msg::UpdateTopBarContext { .. }, Msg::UpdateTopBarContext { .. }) => true,
            _ => false,
        }
    }
}

impl Msg {
    fn is_unit_variant(&self) -> bool {
        matches!(
            self,
            Msg::Submit | Msg::InsertNewline | Msg::Quit | Msg::ToggleSidebar
                | Msg::ToggleThoughts | Msg::OpenCommandPalette | Msg::CloseModal | Msg::ConfirmModal
                | Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown
                | Msg::PermissionConfirm | Msg::PermissionCancel | Msg::PermissionAlways
                | Msg::PermissionSkip | Msg::CommandPaletteBackspace | Msg::CommandPaletteUp
                | Msg::CommandPaletteDown | Msg::CommandPaletteConfirm | Msg::CommandPaletteCancelArgument
                | Msg::AgentEvent(_) | Msg::Tick | Msg::CursorBlink | Msg::SlashCommand(_)
                | Msg::ToggleSessionTree | Msg::SessionTreeUp | Msg::SessionTreeDown
                | Msg::SessionTreeConfirm | Msg::OnboardingNext | Msg::OnboardingBack
                | Msg::OnboardingNavigateUp | Msg::OnboardingNavigateDown | Msg::OnboardingKeyBackspace
                | Msg::OnboardingSearchBackspace | Msg::OnboardingSubmit | Msg::OnboardingSkip
                | Msg::ClearInput | Msg::ClearInputConfirm | Msg::ClearChat | Msg::Stop
                | Msg::PermissionTimeout | Msg::SelectUp | Msg::SelectDown | Msg::SelectConfirm
                | Msg::SelectToggleDetails | Msg::SwitchModel | Msg::EnterOnboarding
                | Msg::ResetAgentState | Msg::HistoryUp | Msg::HistoryDown | Msg::HistorySearchStart
                | Msg::HistorySearchNext | Msg::HistorySearchPrev | Msg::HistorySearchCancel | Msg::HistorySearchConfirm
                | Msg::CopyLastResponse
                | Msg::FilePickerUp | Msg::FilePickerDown | Msg::FilePickerConfirm | Msg::CloseFilePicker
                | Msg::TogglePermissionMode | Msg::ClearAlwaysApprove | Msg::PlanModeApprove | Msg::PlanModeDeny | Msg::PlanModeViewNext | Msg::PlanModeViewPrev | Msg::ToggleScrollFocus
                | Msg::OpenContextUsageModal | Msg::CloseContextUsageModal
        )
    }
}

// ─── Cmd ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Cmd {
    SpawnAgent { messages: Vec<AgentMessage> },
    SendPermission { decision: PermissionDecision },
    SlashCommand(SlashCommand),
    SaveSettings { provider: String, model: String, api_key: String },
    FetchModels { provider_id: String, api_key: String },
    Rollback { tool_call_id: String },
    Interrupt,
}

impl PartialEq for Cmd {
    fn eq(&self, other: &Self) -> bool {
        use Cmd::*;
        match (self, other) {
            (SpawnAgent { .. }, SpawnAgent { .. }) => true,
            (SendPermission { decision: a }, SendPermission { decision: b }) => a == b,
            (SlashCommand(_), SlashCommand(_)) => true,
            (SaveSettings { .. }, SaveSettings { .. }) => true,
            (FetchModels { provider_id: a, api_key: b }, FetchModels { provider_id: c, api_key: d }) => a == c && b == d,
            (Rollback { tool_call_id: a }, Rollback { tool_call_id: b }) => a == b,
            (Interrupt, Interrupt) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TuiAction {
    Quit,
    Submit(String),
    Command(String),
    Cancel,
    ToolPermission { tool: String, action: PermissionAction },
}
