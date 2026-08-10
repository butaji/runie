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
