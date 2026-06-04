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
    Plan,
    Subagents,
    Questionnaire,
    FullscreenViewer,
}

/// Layout mode for responsive terminal display
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LayoutMode {
    #[default]
    Fullscreen,
    Inline,
    Compact,
    Tmux,
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
    ScrollHalfPageUp,
    ScrollHalfPageDown,
    ScrollToTop,
    ScrollToBottom,
    ScrollToPrevUserTurn,
    ScrollToNextUserTurn,
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
    SetPermissionMode(PermissionMode),
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
    HomeScreenToggleSessions,
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
    ShowHelp,
    CollapseEntry,
    ExpandEntry,
    ToggleFoldEntry,
    ToggleAllEntries,
    CopyBlockContent,
    ToggleRawMarkdown,
    FocusPrompt,
    GoHome,
    ToggleAutoApprove,
    OpenExtensionsModal,
    CloseExtensionsModal,
    ExtensionsModalUp,
    ExtensionsModalDown,
    ExtensionsModalSelect,
    ExtensionsModalLeft,
    ExtensionsModalRight,
    ExtensionsModalSearchInput(char),
    ExtensionsModalSearchBackspace,
    ToggleSubagentPanel,
    // Fullscreen viewer
    OpenFullscreenViewer,
    // Mouse support
    MouseClick { x: u16, y: u16, button: u16 },
    // Block operations
    CopyBlockMetadata,
    OpenEntry,
    OpenEntryOptions,
    // Prompt queue
    TogglePromptQueue,
    // Worktree
    NewSessionWorktree,
    ToggleWorktreeMode,
    // Settings import
    ImportClaudeSettings,
    // Questionnaire
    QuestionnaireUp,
    QuestionnaireDown,
    QuestionnairePrevQuestion,
    QuestionnaireNextQuestion,
    QuestionnaireSelect,
    QuestionnaireToggleCustom,
    CloseQuestionnaire,
    ToggleQuestionnaire,
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        if self.is_unit_variant() {
            return std::mem::discriminant(self) == std::mem::discriminant(other);
        }
        // Non-unit variants with data
        compare_msg_data(self, other)
    }
}

fn compare_msg_data(self_: &Msg, other: &Msg) -> bool {
    
    // Guard: ensure same variant
    if std::mem::discriminant(self_) != std::mem::discriminant(other) {
        return false;
    }
    // Variants with single value comparison
    single_eq(self_, other)
        || multi_eq(self_, other)
        || unit_like_eq(self_, other)
}

fn single_eq(self_: &Msg, other: &Msg) -> bool {
    use Msg::*;
    match (self_, other) {
        (TextareaKey(a), TextareaKey(b)) => a == b,
        (CommandPaletteFilter(a), CommandPaletteFilter(b)) => a == b,
        (OnboardingSelectProvider(a), OnboardingSelectProvider(b)) => a == b,
        (OnboardingSelectModel(a), OnboardingSelectModel(b)) => a == b,
        (OnboardingKeyInput(a), OnboardingKeyInput(b)) => a == b,
        (OnboardingSearchInput(a), OnboardingSearchInput(b)) => a == b,
        (DirectCommand(a), DirectCommand(b)) => a == b,
        (Paste(a), Paste(b)) => a == b,
        (ModelsFetched(a), ModelsFetched(b)) => a == b,
        (ModelsFetchFailed(a), ModelsFetchFailed(b)) => a == b,
        _ => false,
    }
}

fn multi_eq(self_: &Msg, other: &Msg) -> bool {
    use Msg::*;
    match (self_, other) {
        (Resize(a_w, a_h), Resize(b_w, b_h)) => a_w == b_w && a_h == b_h,
        (MouseClick { x: ax, y: ay, button: ab }, MouseClick { x: bx, y: by, button: bb }) => {
            ax == bx && ay == by && ab == bb
        }
        _ => false,
    }
}

fn unit_like_eq(self_: &Msg, other: &Msg) -> bool {
    use Msg::*;
    match (self_, other) {
        (SetGitInfo { .. }, SetGitInfo { .. }) => true,
        (SetTopBarMockChecks { .. }, SetTopBarMockChecks { .. }) => true,
        (SetTopBarRealChecks { .. }, SetTopBarRealChecks { .. }) => true,
        (SetInputRightInfo(a), SetInputRightInfo(b)) => a == b,
        (SetCurrentModel(a), SetCurrentModel(b)) => a == b,
        (SetMockMode(a), SetMockMode(b)) => a == b,
        (SetPermissionMode(a), SetPermissionMode(b)) => a == b,
        (UpdateTopBarContext { .. }, UpdateTopBarContext { .. }) => true,
        (ExtensionsModalSearchInput(a), ExtensionsModalSearchInput(b)) => a == b,
        _ => false,
    }
}

impl Msg {
    fn is_unit_variant(&self) -> bool {
        matches!(
            self,
            Msg::Submit | Msg::InsertNewline | Msg::Quit | Msg::ToggleSidebar
                | Msg::ToggleThoughts | Msg::OpenCommandPalette | Msg::CloseModal | Msg::ConfirmModal
                | Msg::ScrollUp | Msg::ScrollDown | Msg::ScrollPageUp | Msg::ScrollPageDown
                | Msg::ScrollHalfPageUp | Msg::ScrollHalfPageDown | Msg::ScrollToTop | Msg::ScrollToBottom
                | Msg::ScrollToPrevUserTurn | Msg::ScrollToNextUserTurn
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
                | Msg::ShowHelp
                | Msg::CollapseEntry | Msg::ExpandEntry | Msg::ToggleFoldEntry | Msg::ToggleAllEntries
                | Msg::OpenExtensionsModal | Msg::CloseExtensionsModal | Msg::ExtensionsModalUp
                | Msg::ExtensionsModalDown | Msg::ExtensionsModalSelect | Msg::ExtensionsModalLeft
                | Msg::ExtensionsModalRight | Msg::ExtensionsModalSearchBackspace
                | Msg::CopyBlockContent | Msg::CopyBlockMetadata | Msg::ToggleRawMarkdown | Msg::FocusPrompt | Msg::GoHome
                | Msg::ToggleAutoApprove | Msg::ToggleSubagentPanel
                | Msg::OpenFullscreenViewer
                | Msg::OpenEntry | Msg::OpenEntryOptions
                | Msg::TogglePromptQueue | Msg::NewSessionWorktree | Msg::ToggleWorktreeMode
                | Msg::ImportClaudeSettings | Msg::HomeScreenToggleSessions
        )
    }

    fn eq_data_variant(&self, other: &Self) -> bool {
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
            (Msg::MouseClick { x: ax, y: ay, button: ab }, Msg::MouseClick { x: bx, y: by, button: bb }) => ax == bx && ay == by && ab == bb,
            (Msg::SetGitInfo { .. }, Msg::SetGitInfo { .. }) => true,
            (Msg::SetTopBarMockChecks { .. }, Msg::SetTopBarMockChecks { .. }) => true,
            (Msg::SetTopBarRealChecks { .. }, Msg::SetTopBarRealChecks { .. }) => true,
            (Msg::SetInputRightInfo(a), Msg::SetInputRightInfo(b)) => a == b,
            (Msg::SetCurrentModel(a), Msg::SetCurrentModel(b)) => a == b,
            (Msg::SetMockMode(a), Msg::SetMockMode(b)) => a == b,
            (Msg::UpdateTopBarContext { .. }, Msg::UpdateTopBarContext { .. }) => true,
            (Msg::ExtensionsModalSearchInput(a), Msg::ExtensionsModalSearchInput(b)) => a == b,
            _ => false,
        }
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

