//! Renderer-independent transcript line vocabulary and reducer intents.

use std::collections::{HashMap, HashSet};

use runie_core::types::{ThemeKind, ToolDisplayMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    User,
    Assistant,
    Reasoning,
    ThinkingStatus,
    Tool,
    ToolRunning,
    ToolError,
    ToolResult,
    ToolOutput,
    SessionStart,
    System,
    Separator,
    TurnSummary,
    CompletedAssistant,
    Activity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub tool_call_id: Option<String>,
    /// Opaque reducer identity for a live tool header. Compatibility-seeded
    /// rows intentionally leave this unset.
    pub tool_row_id: Option<u64>,
    /// True while this reducer-owned row may receive lifecycle mutations.
    /// Completed rows retain their identity for replay/debug assertions but
    /// are no longer eligible targets for a later duplicate call ID.
    tool_row_active: bool,
    has_vpad: bool,
}

/// Immutable feed projection shared across actors, scenario runners, and
/// renderers. It intentionally contains facts and view controls only; the
/// mutable reducer and terminal caches remain in `runie-tui`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeedSnapshot {
    pub lines: Vec<Line>,
    pub tool_blocks: Vec<ToolBlock>,
    /// Tool names are reducer facts used to resolve specialized Grok cards.
    pub tool_names: HashMap<String, String>,
    pub autoscroll: bool,
    pub scroll_offset: usize,
    pub reasoning_expanded: bool,
    pub activity_expanded: bool,
    pub prompt_timestamp: Option<String>,
    pub follow_latest_user: bool,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub theme: ThemeKind,
    pub animation_frame: usize,
    pub tool_modes: HashMap<String, ToolDisplayMode>,
    /// Dense Grok activity groups explicitly revealed by entry selection.
    pub revealed_dense_groups: HashSet<String>,
    /// Whether selection has revealed the centered entry in a dense group.
    pub center_revealed_entry: bool,
}

/// Renderer-independent navigation and animation facts for a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedNavigation {
    pub autoscroll: bool,
    pub scroll_offset: usize,
    pub follow_latest_user: bool,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub animation_frame: usize,
    pub reasoning_expanded: bool,
    pub activity_expanded: bool,
}

impl Default for FeedNavigation {
    fn default() -> Self {
        Self {
            autoscroll: true,
            scroll_offset: 0,
            follow_latest_user: false,
            selected_tool_id: None,
            selected_entry: None,
            animation_frame: 0,
            reasoning_expanded: false,
            activity_expanded: false,
        }
    }
}

impl FeedNavigation {
    pub fn advance_animation(&mut self) {
        self.animation_frame = self.animation_frame.wrapping_add(1);
    }

    pub fn reveal_latest(&mut self, content_len: usize) {
        self.autoscroll = true;
        self.follow_latest_user = false;
        self.scroll_offset = content_len;
    }

    pub fn detach_from_tail(&mut self) {
        self.autoscroll = false;
        self.follow_latest_user = false;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Read-only typed projection of one Grok tool block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBlock {
    pub tool_call_id: String,
    pub header: String,
    pub kind: ToolCardKind,
    pub output: Vec<String>,
    pub mode: ToolDisplayMode,
    pub is_running: bool,
    pub is_error: bool,
    /// Identity of the live header when this projection originates from one.
    pub tool_row_id: Option<u64>,
}

/// Grok's specialized tool-card families supported by pi-core tool events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCardKind {
    Execute,
    Read,
    Edit,
    ListDir,
    Search,
    WebSearch,
    WebFetch,
    MemorySearch,
    Workflow,
    Todo,
    Use,
    SearchTools,
    Background,
    Generic,
}

/// Grok's source default: command execution starts truncated, while other
/// tool cards start collapsed until an explicit UI intent expands them.
pub fn default_tool_display_mode(tool_name: &str) -> ToolDisplayMode {
    if matches!(tool_name, "bash" | "shell" | "exec" | "run") {
        ToolDisplayMode::Truncated
    } else {
        ToolDisplayMode::Collapsed
    }
}

/// Pure projection from transcript facts to Grok's typed tool cards.
/// Ordering follows first appearance in the transcript, including parallel
/// tool calls. Terminal widgets must consume this result rather than rebuild
/// card identity from rendered cells.
#[allow(
    clippy::too_many_lines,
    reason = "keeps the pure line-to-card projection and its ordering rules together"
)]
pub fn project_tool_blocks(
    lines: &[Line],
    tool_names: &HashMap<String, String>,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> Vec<ToolBlock> {
    let mut blocks = Vec::new();
    for line in lines {
        let Some(id) = line.tool_call_id.as_deref() else {
            continue;
        };
        let kind_for = |text: &str| {
            tool_names.get(id).map_or_else(
                || ToolCardKind::from_header(text),
                |name| ToolCardKind::from_header(name),
            )
        };
        let Some(index) = blocks
            .iter()
            .position(|block: &ToolBlock| block.tool_call_id == id)
        else {
            if matches!(
                line.kind,
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
            ) {
                blocks.push(ToolBlock {
                    tool_call_id: id.to_owned(),
                    header: line.text.clone(),
                    kind: kind_for(&line.text),
                    output: Vec::new(),
                    mode: tool_modes
                        .get(id)
                        .copied()
                        .unwrap_or(ToolDisplayMode::Expanded),
                    is_running: line.kind == LineKind::ToolRunning,
                    is_error: line.kind == LineKind::ToolError,
                    tool_row_id: line.tool_row_id,
                });
            }
            continue;
        };
        let block = &mut blocks[index];
        match line.kind {
            LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError => {
                block.header = line.text.clone();
                block.kind = kind_for(&line.text);
                block.is_running = line.kind == LineKind::ToolRunning;
                block.is_error = line.kind == LineKind::ToolError;
            }
            LineKind::ToolOutput | LineKind::ToolResult => block.output.push(line.text.clone()),
            _ => {}
        }
    }
    blocks
}

impl ToolCardKind {
    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "the pure tool alias vocabulary maps Grok card families explicitly"
    )]
    pub fn from_header(header: &str) -> Self {
        let lower = header.trim_start().to_ascii_lowercase();
        if matches!(lower.as_str(), "bash" | "shell" | "exec" | "run")
            || lower.starts_with("run ")
            || lower.starts_with("execute ")
        {
            Self::Execute
        } else if matches!(lower.as_str(), "read" | "read_file") || lower.starts_with("read ") {
            Self::Read
        } else if matches!(
            lower.as_str(),
            "edit" | "write" | "write_file" | "search_replace"
        ) || lower.starts_with("edit ")
            || lower.starts_with("write ")
        {
            Self::Edit
        } else if matches!(lower.as_str(), "list_dir" | "list_files") || lower.starts_with("list ")
        {
            Self::ListDir
        } else if matches!(lower.as_str(), "web_search" | "web-search")
            || lower.starts_with("web search ")
        {
            Self::WebSearch
        } else if matches!(lower.as_str(), "search" | "grep" | "find")
            || lower.starts_with("search ")
        {
            Self::Search
        } else if matches!(lower.as_str(), "web_fetch" | "web-fetch" | "fetch")
            || lower.starts_with("fetch ")
        {
            Self::WebFetch
        } else if matches!(lower.as_str(), "memory_search" | "memory-search")
            || lower.starts_with("memory search ")
        {
            Self::MemorySearch
        } else if matches!(lower.as_str(), "workflow" | "run_workflow" | "run-workflow")
            || lower.starts_with("workflow ")
        {
            Self::Workflow
        } else if matches!(lower.as_str(), "todo" | "todo_write" | "todo-write")
            || lower.starts_with("todo ")
        {
            Self::Todo
        } else if matches!(lower.as_str(), "use" | "use_tool" | "use-tool")
            || lower.starts_with("use ")
        {
            Self::Use
        } else if matches!(lower.as_str(), "search_tools" | "search-tools")
            || lower.starts_with("search tools ")
        {
            Self::SearchTools
        } else if matches!(lower.as_str(), "subagent" | "agent" | "task")
            || lower.starts_with("subagent ")
        {
            Self::Background
        } else {
            Self::Generic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{default_tool_display_mode, project_tool_blocks, Line, LineKind, ToolCardKind};
    use runie_core::types::ToolDisplayMode;
    use std::collections::HashMap;

    #[test]
    fn default_tool_modes_match_grok_families() {
        assert_eq!(
            default_tool_display_mode("bash"),
            ToolDisplayMode::Truncated
        );
        assert_eq!(
            default_tool_display_mode("read"),
            ToolDisplayMode::Collapsed
        );
        assert_eq!(
            default_tool_display_mode("memory_search"),
            ToolDisplayMode::Collapsed
        );
    }

    #[test]
    fn tool_projection_is_ordered_and_renderer_independent() {
        let lines = vec![
            Line::new(LineKind::Tool, "read src/lib.rs").for_tool("second"),
            Line::new(LineKind::ToolOutput, "line").for_tool("second"),
            Line::new(LineKind::ToolRunning, "bash cargo test").for_tool("first"),
        ];
        let names = HashMap::from([
            ("second".to_owned(), "read".to_owned()),
            ("first".to_owned(), "bash".to_owned()),
        ]);
        let blocks = project_tool_blocks(&lines, &names, &HashMap::new());
        assert_eq!(
            blocks
                .iter()
                .map(|block| block.tool_call_id.as_str())
                .collect::<Vec<_>>(),
            ["second", "first"]
        );
        assert_eq!(blocks[0].output, ["line"]);
        assert_eq!(blocks[1].kind, ToolCardKind::Execute);
    }

    #[test]
    fn navigation_transitions_are_pure_and_resettable() {
        let mut navigation = super::FeedNavigation::default();
        navigation.advance_animation();
        navigation.detach_from_tail();
        navigation.reveal_latest(12);
        assert_eq!(navigation.animation_frame, 1);
        assert_eq!(navigation.scroll_offset, 12);
        assert!(navigation.autoscroll);
        assert!(!navigation.follow_latest_user);
        navigation.reset();
        assert_eq!(navigation, super::FeedNavigation::default());
    }
}

impl FeedSnapshot {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

impl Line {
    pub fn new(kind: LineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            tool_call_id: None,
            tool_row_id: None,
            tool_row_active: false,
            has_vpad: false,
        }
    }

    pub fn with_vpad(mut self, has_vpad: bool) -> Self {
        self.has_vpad = has_vpad;
        self
    }

    pub fn has_vpad(&self) -> bool {
        self.has_vpad
    }

    pub fn for_tool(mut self, tool_call_id: impl Into<String>) -> Self {
        self.tool_call_id = Some(tool_call_id.into());
        self
    }

    pub fn for_tool_row(mut self, row_id: u64) -> Self {
        self.tool_row_id = Some(row_id);
        self.tool_row_active = true;
        self
    }

    pub fn is_tool_row_active(&self) -> bool {
        self.tool_row_active
    }

    pub fn settle_tool_row(&mut self) {
        self.tool_row_active = false;
    }
}

/// Inputs accepted by the actor-owned transcript reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollbackMsg {
    Append(Line),
    AppendTurnSummary(String),
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
    SetToolMode(String, ToolDisplayMode),
    ToggleToolMode(String),
    SelectNextTool,
    SelectPreviousTool,
    SelectNextEntry,
    SelectPreviousEntry,
    ScrollBy(i32),
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
