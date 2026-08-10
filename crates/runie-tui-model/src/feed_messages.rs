#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbackDomain {
    Lifecycle,
    Content,
    Tool,
    Workflow,
    Navigation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollbackMsg {
    Append(Line),
    AppendTurnSummary(String),
    TurnStart,
    TurnEnd,
    AssistantStreamStart,
    AssistantStreamEnd,
    Clear,
    SetTheme(ThemeKind),
    AdvanceAnimation,
    RemoveKind(LineKind),
    NormalizeLiveCompletedAssistants,
    AddLiveAssistantTimestamp(usize),
    RemoveEmptyAfter(LineKind),
    NormalizeActivitySpacing,
    SetReasoningExpanded(bool),
    SetActivityExpanded(bool),
    ToggleActivityExpanded,
    SetPromptTimestamp(Option<String>),
    SetFollowLatestUser(bool),
    SetToolName(String, String),
    SetToolArgs(String, serde_json::Value),
    RemoveToolArgs(String),
    ActivityReset,
    ActivityToolStart(String),
    ActivityToolEnd {
        is_error: bool,
    },
    SetToolMode(String, ToolDisplayMode),
    ToggleToolMode(String),
    SelectRange {
        anchor: usize,
        head: usize,
    },
    ClearSelection,
    MouseSelectionStart(CellPosition),
    MouseSelectionExtend(CellPosition),
    MouseSelectionCommit,
    ClearCellSelection,
    RequestCopySelection,
    ClearCopyRequest,
    SelectNextTool,
    SelectPreviousTool,
    SelectNextEntry,
    SelectPreviousEntry,
    ScrollBy(i32),
    /// Deliver physical layout facts from the renderer without mutating the
    /// feed outside its owning actor. The reducer may use these facts for
    /// future Grok-equivalent fold-anchor restoration.
    LayoutMeasured {
        content_rows: usize,
        viewport_rows: usize,
        anchor_row: Option<usize>,
    },
    /// Re-enable follow mode and reveal the newest transcript content.
    /// This models Grok's explicit follow/goto-bottom transition.
    RevealLatest,
    MarkToolError(String),
    ReplaceLine(usize, String),
    ReplaceLastByKind(LineKind, String),
    AppendToLastByKind(LineKind, String),
    ToolStart {
        tool_call_id: String,
        header: String,
        activity: Option<String>,
    },
    /// Explicit provider lifecycle start for an ordinary running tool.
    /// Compatibility seed rows continue to use `ToolStart`.
    ToolStartRunning {
        tool_call_id: String,
        header: String,
        activity: Option<String>,
    },
    ToolUpdate {
        tool_call_id: String,
        header: Option<String>,
        output: Vec<String>,
    },
    ToolEnd {
        tool_call_id: String,
        header: String,
        activity: Option<String>,
        output: Vec<(LineKind, String)>,
    },
    WorkflowStart {
        run_id: String,
        name: String,
        objective: String,
    },
    WorkflowProgress {
        run_id: String,
        phase: String,
        state: String,
        active_agents: u32,
    },
    WorkflowEnd {
        run_id: String,
        status: String,
        elapsed_ms: Option<u64>,
    },
    FinalizeAssistant {
        has_reasoning: bool,
        reasoning_expanded: bool,
        summary: String,
        settled_no_tool_phase: bool,
    },
}

/// Grouped domain events preserve producer intent while the compatibility
/// reducer continues to consume the legacy message vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollbackEvent {
    Lifecycle(ScrollbackLifecycleEvent),
    Content(ScrollbackContentEvent),
    Tool(ScrollbackToolEvent),
    Workflow(ScrollbackWorkflowEvent),
    Navigation(ScrollbackNavigationEvent),
}

macro_rules! declare_lifecycle_events {
    ($($variant:ident => $message:ident),+ $(,)?) => {
        #[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum ScrollbackLifecycleEvent { $($variant),+ }

        impl From<ScrollbackLifecycleEvent> for ScrollbackMsg {
            fn from(event: ScrollbackLifecycleEvent) -> Self {
                match event {
                    $(ScrollbackLifecycleEvent::$variant => ScrollbackMsg::$message,)+
                }
            }
        }
    };
}

declare_lifecycle_events! {
    TurnStarted => TurnStart,
    TurnEnded => TurnEnd,
    AssistantStarted => AssistantStreamStart,
    AssistantEnded => AssistantStreamEnd,
    Cleared => Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollbackContentEvent {
    Append(Line),
    FinalizeAssistant {
        has_reasoning: bool,
        reasoning_expanded: bool,
        summary: String,
        settled_no_tool_phase: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollbackToolEvent {
    Started { tool_call_id: String, header: String, activity: Option<String> },
    Updated { tool_call_id: String, header: Option<String>, output: Vec<String> },
    Ended { tool_call_id: String, header: String, activity: Option<String>, output: Vec<(LineKind, String)> },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollbackWorkflowEvent {
    Started { run_id: String, name: String, objective: String },
    Progress { run_id: String, phase: String, state: String, active_agents: u32 },
    Ended { run_id: String, status: String, elapsed_ms: Option<u64> },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollbackNavigationEvent {
    ClearSelection,
    RevealLatest,
    ScrollBy(i32),
}

impl ScrollbackEvent {
    pub fn into_messages(self) -> Vec<ScrollbackMsg> {
        vec![match self {
            Self::Lifecycle(event) => event.into(),
            Self::Content(event) => match event {
                ScrollbackContentEvent::Append(line) => ScrollbackMsg::Append(line),
                ScrollbackContentEvent::FinalizeAssistant {
                    has_reasoning,
                    reasoning_expanded,
                    summary,
                    settled_no_tool_phase,
                } => ScrollbackMsg::FinalizeAssistant {
                    has_reasoning,
                    reasoning_expanded,
                    summary,
                    settled_no_tool_phase,
                },
            },
            Self::Tool(event) => match event {
                ScrollbackToolEvent::Started { tool_call_id, header, activity } => ScrollbackMsg::ToolStart { tool_call_id, header, activity },
                ScrollbackToolEvent::Updated { tool_call_id, header, output } => ScrollbackMsg::ToolUpdate { tool_call_id, header, output },
                ScrollbackToolEvent::Ended { tool_call_id, header, activity, output } => ScrollbackMsg::ToolEnd { tool_call_id, header, activity, output },
            },
            Self::Workflow(event) => match event {
                ScrollbackWorkflowEvent::Started { run_id, name, objective } => ScrollbackMsg::WorkflowStart { run_id, name, objective },
                ScrollbackWorkflowEvent::Progress { run_id, phase, state, active_agents } => ScrollbackMsg::WorkflowProgress { run_id, phase, state, active_agents },
                ScrollbackWorkflowEvent::Ended { run_id, status, elapsed_ms } => ScrollbackMsg::WorkflowEnd { run_id, status, elapsed_ms },
            },
            Self::Navigation(event) => match event {
                ScrollbackNavigationEvent::ClearSelection => ScrollbackMsg::ClearSelection,
                ScrollbackNavigationEvent::RevealLatest => ScrollbackMsg::RevealLatest,
                ScrollbackNavigationEvent::ScrollBy(delta) => ScrollbackMsg::ScrollBy(delta),
            },
        }]
    }
}
