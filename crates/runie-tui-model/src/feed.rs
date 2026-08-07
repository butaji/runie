//! Renderer-independent transcript line vocabulary and reducer intents.

use std::collections::{HashMap, HashSet};

use runie_core::types::{ThemeKind, ToolDisplayMode};

pub const GROK_GROUP_MAX_VISIBLE: usize = 10;

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
    pub settled_no_tool_phase: bool,
    pub live_grok_layout: bool,
    pub next_tool_row_id: u64,
    pub autoscroll: bool,
    pub scroll_offset: usize,
    pub reasoning_expanded: bool,
    pub activity_expanded: bool,
    pub prompt_timestamp: Option<String>,
    pub revealed_dense_groups: HashSet<String>,
    pub center_revealed_entry: bool,
    pub workflow_headers: HashMap<String, String>,
    pub workflow_phases: HashMap<String, Vec<(String, String)>>,
    pub follow_latest_user: bool,
    pub selected_tool_id: Option<String>,
    pub selected_entry: Option<usize>,
    pub selected_member_index: Option<usize>,
    pub theme: ThemeKind,
    pub animation_frame: usize,
    pub tool_modes: HashMap<String, ToolDisplayMode>,
    pub turn_started: bool,
    /// Last renderer measurement delivered through `LayoutMeasured`.
    pub measured_content_rows: usize,
    pub measured_viewport_rows: usize,
    pub measured_anchor_row: Option<usize>,
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
    pub tool_modes: HashMap<String, ToolDisplayMode>,
    pub theme: ThemeKind,
    pub prompt_timestamp: Option<String>,
    pub revealed_dense_groups: HashSet<String>,
    pub center_revealed_entry: bool,
    pub workflow_headers: HashMap<String, String>,
    pub workflow_phases: HashMap<String, Vec<(String, String)>>,
    pub tool_names: HashMap<String, String>,
    pub settled_no_tool_phase: bool,
    pub live_grok_layout: bool,
    pub next_tool_row_id: u64,
    pub turn_started: bool,
    pub measured_content_rows: usize,
    pub measured_viewport_rows: usize,
    pub measured_anchor_row: Option<usize>,
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
            tool_modes: HashMap::new(),
            theme: ThemeKind::GrokNight,
            prompt_timestamp: None,
            revealed_dense_groups: HashSet::new(),
            center_revealed_entry: false,
            workflow_headers: HashMap::new(),
            workflow_phases: HashMap::new(),
            tool_names: HashMap::new(),
            settled_no_tool_phase: false,
            live_grok_layout: false,
            next_tool_row_id: 0,
            turn_started: false,
            measured_content_rows: 0,
            measured_viewport_rows: 0,
            measured_anchor_row: None,
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

/// Semantic row within a typed Grok tool card. Renderers may add spans,
/// colours, and terminal geometry, but must not rediscover this identity from
/// text after crossing the model boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCardRowKind {
    Header,
    Content,
    Status,
}

/// Renderer-neutral semantic paint role for a typed Grok card row.
///
/// This is deliberately not a terminal colour or Ratatui style: theme and
/// capability resolution belongs to the renderer boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCardPaintIntent {
    Header,
    Running,
    Content,
    Success,
    Error,
    Muted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCardRow {
    pub tool_call_id: String,
    /// Stable ordinal of the logical member within its contiguous card group,
    /// shared by that member's header, content, and status rows.
    pub member_index: usize,
    pub card_kind: ToolCardKind,
    pub row_kind: ToolCardRowKind,
    pub text: String,
    pub mode: ToolDisplayMode,
    pub is_running: bool,
    pub is_error: bool,
}

/// Return the logical member ordinal for a tool call in transcript order.
/// This is the single identity calculation shared by snapshots and renderers.
pub fn logical_tool_member_index(lines: &[Line], tool_call_id: &str) -> Option<usize> {
    let mut indices = HashMap::new();
    let mut next = 0usize;
    for line in lines {
        let Some(id) = line.tool_call_id.as_deref() else {
            continue;
        };
        let index = if let Some(index) = indices.get(id) {
            *index
        } else {
            let index = next;
            next += 1;
            indices.insert(id.to_owned(), index);
            index
        };
        if id == tool_call_id {
            return Some(index);
        }
    }
    None
}

impl ToolCardRow {
    pub fn paint_intent(&self) -> ToolCardPaintIntent {
        match self.row_kind {
            ToolCardRowKind::Header if self.is_running => ToolCardPaintIntent::Running,
            ToolCardRowKind::Header => ToolCardPaintIntent::Header,
            ToolCardRowKind::Content if self.card_kind == ToolCardKind::MemorySearch => {
                ToolCardPaintIntent::Muted
            }
            ToolCardRowKind::Content => ToolCardPaintIntent::Content,
            ToolCardRowKind::Status if self.is_error => ToolCardPaintIntent::Error,
            ToolCardRowKind::Status => ToolCardPaintIntent::Success,
        }
    }
}

/// Project transcript rows into semantic card rows in transcript order.
#[allow(
    clippy::too_many_lines,
    reason = "typed card row projection keeps ownership and lifecycle mapping together"
)]
pub fn project_tool_card_rows(
    lines: &[Line],
    tool_names: &HashMap<String, String>,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> Vec<ToolCardRow> {
    let mut rows = Vec::new();
    let mut member_indices: HashMap<String, usize> = HashMap::new();
    let mut next_member_index = 0usize;
    for line in lines {
        let Some(tool_call_id) = line.tool_call_id.as_deref() else {
            continue;
        };
        let header = tool_names
            .get(tool_call_id)
            .map(String::as_str)
            .unwrap_or(&line.text);
        let row_kind = match line.kind {
            LineKind::Tool | LineKind::ToolRunning if !line.text.trim_end().ends_with('✗') => {
                ToolCardRowKind::Header
            }
            LineKind::ToolError | LineKind::Tool => ToolCardRowKind::Status,
            LineKind::ToolOutput | LineKind::ToolResult => ToolCardRowKind::Content,
            _ => continue,
        };
        let row_member_index = if let Some(index) = member_indices.get(tool_call_id) {
            *index
        } else {
            let index = next_member_index;
            next_member_index += 1;
            member_indices.insert(tool_call_id.to_owned(), index);
            index
        };
        rows.push(ToolCardRow {
            tool_call_id: tool_call_id.to_owned(),
            member_index: row_member_index,
            card_kind: ToolCardKind::from_header(header),
            row_kind,
            text: line.text.clone(),
            mode: tool_mode_for_line(line, tool_modes),
            is_running: line.kind == LineKind::ToolRunning,
            is_error: line.kind == LineKind::ToolError,
        });
    }
    rows
}

pub fn tool_mode_for_line(
    line: &Line,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> ToolDisplayMode {
    tool_mode_override_for_line(line, tool_modes).unwrap_or(ToolDisplayMode::Expanded)
}

pub fn tool_mode_override_for_line(
    line: &Line,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> Option<ToolDisplayMode> {
    line.tool_call_id
        .as_deref()
        .and_then(|id| tool_modes.get(id).copied())
        .or_else(|| {
            line.tool_row_id
                .and_then(|row_id| tool_modes.get(&format!("#row:{row_id}")).copied())
        })
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
    if matches!(
        tool_name,
        "bash" | "shell" | "exec" | "run" | "execute" | "run_terminal_command" | "run_terminal_cmd"
    ) {
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
        let is_header = matches!(
            line.kind,
            LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
        );
        let existing_index = if is_header {
            if let Some(row_id) = line.tool_row_id {
                blocks
                    .iter()
                    .position(|block: &ToolBlock| block.tool_row_id == Some(row_id))
            } else {
                blocks
                    .iter()
                    .position(|block: &ToolBlock| block.tool_call_id == id)
            }
        } else {
            blocks
                .iter()
                .rposition(|block: &ToolBlock| block.tool_call_id == id)
        };
        let Some(index) = existing_index else {
            if is_header {
                blocks.push(ToolBlock {
                    tool_call_id: id.to_owned(),
                    header: line.text.clone(),
                    kind: kind_for(&line.text),
                    output: Vec::new(),
                    mode: tool_mode_for_line(line, tool_modes),
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
                block.mode = tool_mode_for_line(line, tool_modes);
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
        if matches!(
            lower.as_str(),
            "bash"
                | "shell"
                | "exec"
                | "run"
                | "execute"
                | "run_terminal_command"
                | "run_terminal_cmd"
        ) || lower.starts_with("run ")
            || lower.starts_with("execute ")
        {
            Self::Execute
        } else if matches!(lower.as_str(), "read" | "read_file") || lower.starts_with("read ") {
            Self::Read
        } else if matches!(
            lower.as_str(),
            "edit" | "write" | "write_file" | "search_replace" | "apply_patch" | "strreplace"
        ) || lower.starts_with("edit ")
            || lower.starts_with("write ")
            || lower.starts_with("apply_patch ")
            || lower.starts_with("strreplace ")
        {
            Self::Edit
        } else if matches!(lower.as_str(), "list_dir" | "list_files" | "ls")
            || lower.starts_with("list ")
            || lower.starts_with("ls ")
        {
            Self::ListDir
        } else if matches!(lower.as_str(), "web_search" | "web-search")
            || lower.starts_with("web search ")
        {
            Self::WebSearch
        } else if matches!(lower.as_str(), "search" | "grep" | "find" | "glob")
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
        } else if matches!(
            lower.as_str(),
            "search_tools" | "search-tools" | "search_tool"
        ) || lower.starts_with("search tools ")
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
    use super::{
        default_tool_display_mode, project_tool_blocks, project_tool_card_rows, FeedState, Line,
        LineKind, ToolCardKind, ToolCardPaintIntent, ToolCardRow, ToolCardRowKind,
    };
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
    fn edit_aliases_match_groks_edit_card_family() {
        for header in [
            "apply_patch",
            "apply_patch src/lib.rs",
            "strreplace",
            "edit",
        ] {
            assert_eq!(ToolCardKind::from_header(header), ToolCardKind::Edit);
        }
    }

    #[test]
    fn ls_alias_matches_groks_list_dir_card_family() {
        assert_eq!(ToolCardKind::from_header("ls"), ToolCardKind::ListDir);
        assert_eq!(ToolCardKind::from_header("ls src"), ToolCardKind::ListDir);
    }

    #[test]
    fn terminal_command_aliases_match_groks_execute_family() {
        for header in ["execute", "run_terminal_command", "run_terminal_cmd"] {
            assert_eq!(ToolCardKind::from_header(header), ToolCardKind::Execute);
            assert_eq!(
                default_tool_display_mode(header),
                ToolDisplayMode::Truncated
            );
        }
    }

    #[test]
    fn grok_search_aliases_keep_their_specialized_card_families() {
        assert_eq!(ToolCardKind::from_header("glob"), ToolCardKind::Search);
        assert_eq!(
            ToolCardKind::from_header("search_tool"),
            ToolCardKind::SearchTools
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
    fn typed_card_rows_expose_semantic_paint_intents() {
        let header = ToolCardRow {
            tool_call_id: "read-1".into(),
            member_index: 0,
            card_kind: ToolCardKind::Read,
            row_kind: ToolCardRowKind::Header,
            text: "Read file".into(),
            mode: ToolDisplayMode::Collapsed,
            is_running: true,
            is_error: false,
        };
        let output = ToolCardRow {
            row_kind: ToolCardRowKind::Content,
            ..header.clone()
        };
        let error = ToolCardRow {
            row_kind: ToolCardRowKind::Status,
            is_running: false,
            is_error: true,
            ..header.clone()
        };
        let memory = ToolCardRow {
            card_kind: ToolCardKind::MemorySearch,
            row_kind: ToolCardRowKind::Content,
            ..header.clone()
        };
        assert_eq!(header.paint_intent(), ToolCardPaintIntent::Running);
        let mut settled_header = header.clone();
        settled_header.is_running = false;
        assert_eq!(settled_header.paint_intent(), ToolCardPaintIntent::Header);
        assert_eq!(output.paint_intent(), ToolCardPaintIntent::Content);
        assert_eq!(error.paint_intent(), ToolCardPaintIntent::Error);
        assert_eq!(memory.paint_intent(), ToolCardPaintIntent::Muted);
    }

    #[test]
    fn card_rows_preserve_specialized_identity_and_semantic_role() {
        let lines = vec![
            Line::new(LineKind::Tool, "Read README.md").for_tool("call-1"),
            Line::new(LineKind::ToolOutput, "first line").for_tool("call-1"),
            Line::new(LineKind::ToolError, "failed").for_tool("call-1"),
        ];
        let names = HashMap::from([(String::from("call-1"), String::from("read"))]);
        let rows = project_tool_card_rows(&lines, &names, &HashMap::new());
        assert_eq!(rows[0].card_kind, ToolCardKind::Read);
        assert_eq!(rows[0].row_kind, ToolCardRowKind::Header);
        assert_eq!(rows[1].row_kind, ToolCardRowKind::Content);
        assert!(rows[2].is_error);
        assert_eq!(rows[2].row_kind, ToolCardRowKind::Status);
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

    #[test]
    fn feed_state_reduces_event_sequence_without_renderer_types() {
        let mut state = super::FeedState::default();
        for message in [
            super::ScrollbackMsg::Append(super::Line::new(super::LineKind::User, "Hey")),
            super::ScrollbackMsg::SetToolName("call-1".into(), "read".into()),
            super::ScrollbackMsg::ToolStart {
                tool_call_id: "call-1".into(),
                header: "Read README.md".into(),
                activity: None,
            },
            super::ScrollbackMsg::ToolUpdate {
                tool_call_id: "call-1".into(),
                header: None,
                output: vec!["line one".into()],
            },
            super::ScrollbackMsg::ToolEnd {
                tool_call_id: "call-1".into(),
                header: "Read README.md (1 line)".into(),
                activity: None,
                output: vec![(super::LineKind::ToolResult, "done".into())],
            },
        ] {
            state.reduce(message);
        }
        let snapshot = state.snapshot();
        assert_eq!(snapshot.lines[0].kind, super::LineKind::User);
        assert_eq!(snapshot.tool_blocks.len(), 1);
        assert_eq!(snapshot.tool_blocks[0].output, ["line one", "done"]);
        assert_eq!(snapshot.tool_blocks[0].kind, super::ToolCardKind::Read);
    }

    #[test]
    fn terminal_tool_output_replay_is_not_appended_twice() {
        let mut state = super::FeedState::default();
        state.reduce(super::ScrollbackMsg::SetToolName(
            "call-1".into(),
            "read".into(),
        ));
        state.reduce(super::ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        state.reduce(super::ScrollbackMsg::ToolUpdate {
            tool_call_id: "call-1".into(),
            header: None,
            output: vec!["first".into(), "second".into()],
        });
        state.reduce(super::ScrollbackMsg::ToolEnd {
            tool_call_id: "call-1".into(),
            header: "Read README.md (2 lines)".into(),
            activity: Some("completed".into()),
            output: vec![
                (super::LineKind::ToolResult, "first".into()),
                (super::LineKind::ToolResult, "second".into()),
            ],
        });
        assert_eq!(state.snapshot().tool_blocks[0].output, ["first", "second"]);
    }

    #[test]
    fn workflow_phase_glyphs_match_grok_fallback_for_terminal_states() {
        assert_eq!(
            super::workflow_text_model(
                "Workflow release: ship it",
                &[("upload".into(), "cancelled".into())],
                "cancelled",
                Some(900),
                0,
            ),
            "Workflow release ◌ cancelled after 0.9s: ship it  [upload ○]"
        );
    }

    #[test]
    fn running_generic_fold_cycle_is_preserved_by_model_delegation() {
        let mut state = super::FeedState::default();
        state.reduce(super::ScrollbackMsg::ToolStartRunning {
            tool_call_id: "call-1".into(),
            header: "custom_tool running".into(),
            activity: None,
        });
        state.reduce(super::ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            state.snapshot().tool_blocks[0].mode,
            ToolDisplayMode::Truncated
        );
        state.reduce(super::ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            state.snapshot().tool_blocks[0].mode,
            ToolDisplayMode::Expanded
        );
    }

    #[test]
    fn read_card_settles_collapsed_after_completion() {
        let mut state = super::FeedState::default();
        state.reduce(super::ScrollbackMsg::SetToolName(
            "read-1".into(),
            "read".into(),
        ));
        state.reduce(super::ScrollbackMsg::ToolStart {
            tool_call_id: "read-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        state.reduce(super::ScrollbackMsg::SetToolMode(
            "read-1".into(),
            ToolDisplayMode::Expanded,
        ));
        state.reduce(super::ScrollbackMsg::ToolEnd {
            tool_call_id: "read-1".into(),
            header: "Read README.md (2 lines)".into(),
            activity: None,
            output: vec![],
        });
        assert_eq!(
            state.snapshot().tool_blocks[0].mode,
            ToolDisplayMode::Collapsed
        );
    }

    #[test]
    fn layout_measurement_is_delivered_through_the_feed_event_boundary() {
        let mut state = FeedState::default();
        state.reduce(super::ScrollbackMsg::LayoutMeasured {
            content_rows: 42,
            viewport_rows: 12,
            anchor_row: Some(9),
        });
        let snapshot = state.snapshot();
        assert_eq!(snapshot.measured_content_rows, 42);
        assert_eq!(snapshot.measured_viewport_rows, 12);
        assert_eq!(snapshot.measured_anchor_row, Some(9));
    }

    #[test]
    fn measured_anchor_restores_manual_viewport_after_tool_fold() {
        let mut state = FeedState::default();
        state.reduce(super::ScrollbackMsg::ToolStartRunning {
            tool_call_id: "call-1".into(),
            header: "custom_tool running".into(),
            activity: None,
        });
        state.reduce(super::ScrollbackMsg::LayoutMeasured {
            content_rows: 30,
            viewport_rows: 6,
            anchor_row: Some(17),
        });
        state.reduce(super::ScrollbackMsg::ScrollBy(3));
        state.reduce(super::ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(state.snapshot().scroll_offset, 14);
        assert!(!state.snapshot().autoscroll);
    }
}

fn workflow_text_model(
    header: &str,
    phases: &[(String, String)],
    status: &str,
    elapsed_ms: Option<u64>,
    active_agents: u32,
) -> String {
    let body = header.strip_prefix("Workflow ").unwrap_or(header);
    let (name, objective) = body.split_once(':').unwrap_or((body, ""));
    let duration = elapsed_ms
        .map(|ms| format!("{:.1}s", ms as f64 / 1000.0))
        .unwrap_or_default();
    let elapsed = if duration.is_empty() {
        String::new()
    } else {
        format!(" in {duration}")
    };
    let verb = match status {
        "active" => format!("{name}: "),
        "cancelled" => format!("{name} ◌ cancelled after {duration}: "),
        "paused" => format!("{name} paused at {duration}: "),
        "failed" | "interrupted" => format!("{name} failed{elapsed}: "),
        _ => format!("{name} done{elapsed}: "),
    };
    let objective = objective.split_whitespace().collect::<Vec<_>>().join(" ");
    let trail = phases
        .iter()
        .map(|(title, phase_state)| {
            let mark = match phase_state.as_str() {
                "active" | "running" => '●',
                "done" | "completed" => '✓',
                "failed" | "error" | "interrupted" => '✗',
                _ => '○',
            };
            format!("{title} {mark}")
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let mut result = format!("Workflow {verb}{objective}");
    if !trail.is_empty() {
        result.push_str(&format!("  [{trail}]"));
    }
    if status == "active" && active_agents > 0 {
        result.push_str(&format!("  ({active_agents} agents)"));
    }
    result
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
    TurnStart,
    TurnEnd,
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

/// Pure actor-owned feed state. It contains transcript facts and navigation
/// only; terminal geometry, styles, and Ratatui buffers remain outside this
/// crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedState {
    pub lines: Vec<Line>,
    pub navigation: FeedNavigation,
}

impl FeedState {
    pub fn snapshot(&self) -> FeedSnapshot {
        let selected_member_index = self.selected_member_index();
        FeedSnapshot {
            lines: self.lines.clone(),
            tool_blocks: project_tool_blocks(
                &self.lines,
                &self.navigation.tool_names,
                &self.navigation.tool_modes,
            ),
            tool_names: self.navigation.tool_names.clone(),
            settled_no_tool_phase: self.navigation.settled_no_tool_phase,
            live_grok_layout: self.navigation.live_grok_layout,
            next_tool_row_id: self.navigation.next_tool_row_id,
            autoscroll: self.navigation.autoscroll,
            scroll_offset: self.navigation.scroll_offset,
            reasoning_expanded: self.navigation.reasoning_expanded,
            activity_expanded: self.navigation.activity_expanded,
            prompt_timestamp: self.navigation.prompt_timestamp.clone(),
            revealed_dense_groups: self.navigation.revealed_dense_groups.clone(),
            center_revealed_entry: self.navigation.center_revealed_entry,
            workflow_headers: self.navigation.workflow_headers.clone(),
            workflow_phases: self.navigation.workflow_phases.clone(),
            follow_latest_user: self.navigation.follow_latest_user,
            selected_tool_id: self.navigation.selected_tool_id.clone(),
            selected_entry: self.navigation.selected_entry,
            selected_member_index,
            theme: self.navigation.theme,
            animation_frame: self.navigation.animation_frame,
            tool_modes: self.navigation.tool_modes.clone(),
            turn_started: self.navigation.turn_started,
            measured_content_rows: self.navigation.measured_content_rows,
            measured_viewport_rows: self.navigation.measured_viewport_rows,
            measured_anchor_row: self.navigation.measured_anchor_row,
        }
    }

    fn selected_member_index(&self) -> Option<usize> {
        let entry = self.navigation.selected_entry?;
        let selected_id = self.lines.get(entry)?.tool_call_id.as_ref()?;
        logical_tool_member_index(&self.lines, selected_id)
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "the event vocabulary is reduced in one explicit actor boundary"
    )]
    pub fn reduce(&mut self, message: ScrollbackMsg) {
        match message {
            ScrollbackMsg::Append(line) => {
                self.append(line);
            }
            ScrollbackMsg::AppendTurnSummary(text) => {
                self.append(Line::new(LineKind::TurnSummary, text));
            }
            ScrollbackMsg::TurnStart => self.navigation.turn_started = true,
            ScrollbackMsg::TurnEnd => self.navigation.turn_started = false,
            ScrollbackMsg::Clear => self.clear(),
            ScrollbackMsg::SetTheme(theme) => self.navigation.theme = theme,
            ScrollbackMsg::AdvanceAnimation => self.navigation.advance_animation(),
            ScrollbackMsg::RemoveKind(kind) => self.lines.retain(|line| line.kind != kind),
            ScrollbackMsg::NormalizeLiveCompletedAssistants => {
                for line in &mut self.lines {
                    if line.kind == LineKind::Assistant && !line.text.is_empty() {
                        line.kind = LineKind::CompletedAssistant;
                    }
                }
            }
            ScrollbackMsg::AddLiveAssistantTimestamp(_) => {}
            ScrollbackMsg::RemoveEmptyAfter(kind) => self.remove_empty_after(kind),
            ScrollbackMsg::NormalizeActivitySpacing => self.normalize_activity_spacing(),
            ScrollbackMsg::SetReasoningExpanded(value) => {
                self.navigation.reasoning_expanded = value
            }
            ScrollbackMsg::SetActivityExpanded(value) => self.navigation.activity_expanded = value,
            ScrollbackMsg::ToggleActivityExpanded => {
                self.navigation.activity_expanded = !self.navigation.activity_expanded;
            }
            ScrollbackMsg::SetPromptTimestamp(value) => self.navigation.prompt_timestamp = value,
            ScrollbackMsg::SetFollowLatestUser(value) => self.navigation.follow_latest_user = value,
            ScrollbackMsg::SetToolName(id, name) => {
                self.navigation.tool_names.insert(id, name);
            }
            ScrollbackMsg::SetToolMode(id, mode) => {
                if let Some(row_id) = self
                    .lines
                    .iter()
                    .rev()
                    .find(|line| line.tool_call_id.as_deref() == Some(id.as_str()))
                    .and_then(|line| line.tool_row_id)
                {
                    self.navigation
                        .tool_modes
                        .insert(format!("#row:{row_id}"), mode);
                    self.navigation.tool_modes.insert(id, mode);
                } else {
                    self.navigation.tool_modes.insert(id, mode);
                }
            }
            ScrollbackMsg::ToggleToolMode(id) => self.toggle_tool_mode(&id),
            ScrollbackMsg::SelectNextTool => self.select_tool(1),
            ScrollbackMsg::SelectPreviousTool => self.select_tool(-1),
            ScrollbackMsg::SelectNextEntry => self.select_entry(1),
            ScrollbackMsg::SelectPreviousEntry => self.select_entry(-1),
            ScrollbackMsg::ScrollBy(delta) => self.scroll_by(delta),
            ScrollbackMsg::LayoutMeasured {
                content_rows,
                viewport_rows,
                anchor_row,
            } => {
                self.navigation.measured_content_rows = content_rows;
                self.navigation.measured_viewport_rows = viewport_rows;
                self.navigation.measured_anchor_row = anchor_row;
            }
            ScrollbackMsg::RevealLatest => self.navigation.reveal_latest(self.lines.len()),
            ScrollbackMsg::MarkToolError(id) => self.mark_tool_error(&id),
            ScrollbackMsg::ReplaceLine(index, text) => {
                if let Some(line) = self.lines.get_mut(index) {
                    line.text = text;
                }
            }
            ScrollbackMsg::ReplaceLastByKind(kind, text) => {
                if let Some(line) = self.lines.iter_mut().rev().find(|line| line.kind == kind) {
                    line.text = text;
                }
            }
            ScrollbackMsg::AppendToLastByKind(kind, text) => {
                if let Some(line) = self.lines.iter_mut().rev().find(|line| line.kind == kind) {
                    line.text.push_str(&text);
                } else {
                    self.append(Line::new(kind, text));
                }
            }
            ScrollbackMsg::ToolStart {
                tool_call_id,
                header,
                activity,
            } => self.start_tool(tool_call_id, header, activity, false),
            ScrollbackMsg::ToolStartRunning {
                tool_call_id,
                header,
                activity,
            } => self.start_tool(tool_call_id, header, activity, true),
            ScrollbackMsg::ToolUpdate {
                tool_call_id,
                header,
                output,
            } => {
                if let Some(header) = header {
                    self.update_tool(&tool_call_id, header);
                }
                for text in output {
                    self.append(Line::new(LineKind::ToolOutput, text).for_tool(&tool_call_id));
                }
            }
            ScrollbackMsg::ToolEnd {
                tool_call_id,
                header,
                activity,
                output,
            } => {
                let mode_key = self
                    .lines
                    .iter()
                    .rev()
                    .find(|line| {
                        line.is_tool_row_active()
                            && line.tool_call_id.as_deref() == Some(tool_call_id.as_str())
                    })
                    .and_then(|line| line.tool_row_id)
                    .map_or_else(|| tool_call_id.clone(), |row_id| format!("#row:{row_id}"));
                self.replace_tool(&tool_call_id, header);
                if let Some(name) = self.navigation.tool_names.get(&tool_call_id) {
                    if matches!(name.as_str(), "read" | "read_file") {
                        // Grok's ReadToolCallBlock always settles back to its
                        // title-only card after completion, even if it was
                        // expanded while running.
                        self.navigation
                            .tool_modes
                            .insert(mode_key.clone(), ToolDisplayMode::Collapsed);
                        self.navigation
                            .tool_modes
                            .insert(tool_call_id.clone(), ToolDisplayMode::Collapsed);
                    } else if matches!(name.as_str(), "bash" | "shell" | "exec" | "run")
                        && self
                            .navigation
                            .tool_modes
                            .get(&mode_key)
                            .or_else(|| self.navigation.tool_modes.get(&tool_call_id))
                            == Some(&ToolDisplayMode::Truncated)
                    {
                        self.navigation
                            .tool_modes
                            .insert(mode_key, ToolDisplayMode::Expanded);
                        self.navigation
                            .tool_modes
                            .insert(tool_call_id.clone(), ToolDisplayMode::Expanded);
                    }
                }
                let terminal_output_is_replay_of_update =
                    self.tool_output_suffix_matches(&tool_call_id, &output);
                if !terminal_output_is_replay_of_update {
                    for (kind, text) in output {
                        self.append(Line::new(kind, text).for_tool(&tool_call_id));
                    }
                }
                self.replace_or_append_activity(activity);
            }
            ScrollbackMsg::WorkflowStart {
                run_id,
                name,
                objective,
            } => {
                let header = format!("Workflow {name}: {objective}");
                self.navigation
                    .workflow_headers
                    .insert(run_id.clone(), header.clone());
                self.navigation
                    .workflow_phases
                    .insert(run_id.clone(), Vec::new());
                self.append(Line::new(LineKind::ToolRunning, header).for_tool(run_id));
            }
            ScrollbackMsg::WorkflowProgress {
                run_id,
                phase,
                state,
                active_agents,
            } => {
                let phases = self
                    .navigation
                    .workflow_phases
                    .entry(run_id.clone())
                    .or_default();
                if let Some(existing) = phases.iter_mut().find(|(title, _)| title == &phase) {
                    existing.1 = state;
                } else {
                    phases.push((phase, state));
                }
                let header = self
                    .navigation
                    .workflow_headers
                    .get(&run_id)
                    .cloned()
                    .unwrap_or_else(|| "Workflow".into());
                let phases = self
                    .navigation
                    .workflow_phases
                    .get(&run_id)
                    .cloned()
                    .unwrap_or_default();
                self.replace_tool(
                    &run_id,
                    workflow_text_model(&header, &phases, "active", None, active_agents),
                );
            }
            ScrollbackMsg::WorkflowEnd {
                run_id,
                status,
                elapsed_ms,
            } => {
                let header = self
                    .navigation
                    .workflow_headers
                    .get(&run_id)
                    .cloned()
                    .unwrap_or_else(|| "Workflow".into());
                let phases = self
                    .navigation
                    .workflow_phases
                    .get(&run_id)
                    .cloned()
                    .unwrap_or_default();
                self.replace_tool(
                    &run_id,
                    workflow_text_model(&header, &phases, &status, elapsed_ms, 0),
                );
            }
            ScrollbackMsg::FinalizeAssistant {
                has_reasoning,
                reasoning_expanded,
                summary,
                settled_no_tool_phase,
            } => {
                self.navigation.settled_no_tool_phase = settled_no_tool_phase;
                if !has_reasoning || reasoning_expanded {
                    self.lines
                        .retain(|line| line.kind != LineKind::ThinkingStatus);
                } else if let Some(line) = self
                    .lines
                    .iter_mut()
                    .rev()
                    .find(|line| line.kind == LineKind::ThinkingStatus)
                {
                    line.kind = LineKind::TurnSummary;
                    line.text = summary;
                    self.lines.retain(|line| line.kind != LineKind::Reasoning);
                }
            }
        }
    }

    fn start_tool(
        &mut self,
        tool_call_id: String,
        header: String,
        activity: Option<String>,
        running: bool,
    ) {
        self.replace_or_append_activity(activity);
        if let Some(tool_name) = self.navigation.tool_names.get(&tool_call_id) {
            self.navigation
                .tool_modes
                .entry(tool_call_id.clone())
                .or_insert_with(|| default_tool_display_mode(tool_name));
        }
        let kind = if running || header.starts_with("Subagent running:") {
            LineKind::ToolRunning
        } else {
            LineKind::Tool
        };
        let row_id = self.navigation.next_tool_row_id;
        self.navigation.next_tool_row_id = row_id.wrapping_add(1);
        self.append(
            Line::new(kind, header)
                .for_tool(tool_call_id)
                .for_tool_row(row_id),
        );
    }

    fn append(&mut self, line: Line) {
        if line.kind == LineKind::User {
            self.navigation.follow_latest_user = true;
        }
        self.lines.push(line);
        if self.navigation.autoscroll {
            self.navigation.scroll_offset = self.lines.len();
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.navigation.tool_names.clear();
        self.navigation.tool_modes.clear();
        self.navigation.workflow_headers.clear();
        self.navigation.workflow_phases.clear();
        self.navigation.revealed_dense_groups.clear();
        self.navigation.next_tool_row_id = 0;
        self.navigation.selected_tool_id = None;
        self.navigation.selected_entry = None;
        self.navigation.scroll_offset = 0;
        self.navigation.follow_latest_user = false;
    }

    fn replace_tool(&mut self, id: &str, text: String) {
        // Provider call IDs are not guaranteed to be unique across replayed
        // or concurrent lifecycle fragments. Prefer the newest actor-owned
        // live row, exactly as the event stream's row identity requires;
        // falling back to a settled row is only for compatibility-seeded
        // transcripts that have no opaque row identity.
        if let Some(line) = self.live_header_mut(id) {
            line.text = text;
            line.kind = LineKind::Tool;
            line.settle_tool_row();
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.text = text;
            line.kind = LineKind::Tool;
            line.settle_tool_row();
        }
    }

    fn update_tool(&mut self, id: &str, text: String) {
        if let Some(line) = self.live_header_mut(id) {
            line.text = text;
            return;
        }
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.text = text;
        }
    }

    fn live_header_mut(&mut self, id: &str) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|line| {
            line.tool_row_id.is_some()
                && line.is_tool_row_active()
                && line.tool_call_id.as_deref() == Some(id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        })
    }

    fn tool_output_suffix_matches(&self, id: &str, output: &[(LineKind, String)]) -> bool {
        if output.is_empty() || self.lines.len() < output.len() {
            return false;
        }
        let existing: Vec<&str> = self
            .lines
            .iter()
            .filter(|line| line.tool_call_id.as_deref() == Some(id))
            .map(|line| line.text.as_str())
            .collect();
        output
            .iter()
            .all(|(_kind, expected)| existing.contains(&expected.as_str()))
    }

    fn mark_tool_error(&mut self, id: &str) {
        if let Some(line) = self.lines.iter_mut().rev().find(|line| {
            line.tool_call_id.as_deref() == Some(id)
                && matches!(
                    line.kind,
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                )
        }) {
            line.kind = LineKind::ToolError;
        }
    }

    fn replace_or_append_activity(&mut self, activity: Option<String>) {
        let Some(activity) = activity else {
            return;
        };
        if let Some(line) = self
            .lines
            .iter_mut()
            .rev()
            .find(|line| line.kind == LineKind::Activity)
        {
            line.text = activity;
        } else {
            self.append(Line::new(LineKind::Activity, activity));
        }
    }

    fn remove_empty_after(&mut self, kind: LineKind) {
        if let Some(index) = self.lines.iter().position(|line| line.kind == kind) {
            if self
                .lines
                .get(index + 1)
                .is_some_and(|line| line.text.is_empty())
            {
                self.lines.remove(index + 1);
            }
        }
    }

    fn normalize_activity_spacing(&mut self) {
        let Some(index) = self
            .lines
            .iter()
            .position(|line| line.kind == LineKind::Activity)
        else {
            return;
        };
        self.lines
            .retain(|line| !(line.kind == LineKind::System && line.text.is_empty()));
        self.lines
            .insert(index + 1, Line::new(LineKind::Separator, ""));
    }

    fn selectable_entries(&self) -> Vec<usize> {
        let mut seen = HashSet::new();
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let selectable = match line.kind {
                    LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError => line
                        .tool_call_id
                        .as_ref()
                        .is_none_or(|id| seen.insert(id.clone())),
                    LineKind::User | LineKind::Assistant | LineKind::Reasoning => true,
                    _ => false,
                };
                selectable.then_some(index)
            })
            .collect()
    }

    fn select_entry(&mut self, direction: i8) {
        let entries = self.selectable_entries();
        if entries.is_empty() {
            self.navigation.selected_entry = None;
            return;
        }
        let current = self
            .navigation
            .selected_entry
            .and_then(|entry| entries.iter().position(|candidate| *candidate == entry));
        let next = match (current, direction) {
            (None, 1) => 0,
            (None, -1) => entries.len() - 1,
            (Some(index), 1) => (index + 1) % entries.len(),
            (Some(0), -1) => entries.len() - 1,
            (Some(index), -1) => index - 1,
            _ => 0,
        };
        self.navigation.selected_entry = Some(entries[next]);
        self.navigation.selected_tool_id = self.lines[entries[next]].tool_call_id.clone();
        self.navigation.detach_from_tail();
    }

    fn select_tool(&mut self, direction: i8) {
        let ids: Vec<String> = project_tool_blocks(
            &self.lines,
            &self.navigation.tool_names,
            &self.navigation.tool_modes,
        )
        .into_iter()
        .map(|block| block.tool_call_id)
        .collect();
        if ids.is_empty() {
            self.navigation.selected_tool_id = None;
            return;
        }
        let current = self
            .navigation
            .selected_tool_id
            .as_ref()
            .and_then(|id| ids.iter().position(|candidate| candidate == id));
        let next = match (current, direction) {
            (None, 1) => 0,
            (None, -1) => ids.len() - 1,
            (Some(index), 1) => (index + 1) % ids.len(),
            (Some(0), -1) => ids.len() - 1,
            (Some(index), -1) => index - 1,
            _ => 0,
        };
        let selected_id = ids[next].clone();
        self.navigation.selected_tool_id = Some(selected_id.clone());
        self.navigation.selected_entry = self
            .lines
            .iter()
            .position(|line| line.tool_call_id.as_deref() == Some(selected_id.as_str()));
        self.reveal_dense_group(&selected_id);
    }

    fn reveal_dense_group(&mut self, tool_id: &str) {
        let Some(member_index) = self
            .lines
            .iter()
            .position(|line| line.tool_call_id.as_deref() == Some(tool_id))
        else {
            return;
        };
        let start = self.lines[..=member_index]
            .iter()
            .rposition(|line| {
                !matches!(
                    line.kind,
                    LineKind::Tool
                        | LineKind::ToolRunning
                        | LineKind::ToolError
                        | LineKind::ToolOutput
                        | LineKind::ToolResult
                )
            })
            .map_or(0, |index| index + 1);
        let ids: Vec<String> = self.lines[start..]
            .iter()
            .take_while(|line| {
                matches!(
                    line.kind,
                    LineKind::Tool
                        | LineKind::ToolRunning
                        | LineKind::ToolError
                        | LineKind::ToolOutput
                        | LineKind::ToolResult
                )
            })
            .filter_map(|line| line.tool_call_id.clone())
            .collect();
        if ids.len() > GROK_GROUP_MAX_VISIBLE {
            self.navigation.revealed_dense_groups.insert(ids[0].clone());
            self.navigation.selected_entry = Some(member_index);
            self.navigation.center_revealed_entry = true;
        }
    }

    fn toggle_tool_mode(&mut self, id: &str) {
        let read_card = self
            .navigation
            .tool_names
            .get(id)
            .is_some_and(|name| matches!(name.as_str(), "read" | "read_file"));
        let running_generic_card = project_tool_blocks(
            &self.lines,
            &self.navigation.tool_names,
            &self.navigation.tool_modes,
        )
        .iter()
        .any(|block| {
            block.tool_call_id == id && block.is_running && block.kind == ToolCardKind::Generic
        });
        let mode = self
            .navigation
            .tool_modes
            .get(id)
            .copied()
            .unwrap_or(ToolDisplayMode::Expanded);
        let next = match mode {
            ToolDisplayMode::Collapsed if read_card || running_generic_card => {
                ToolDisplayMode::Truncated
            }
            ToolDisplayMode::Collapsed => ToolDisplayMode::Expanded,
            ToolDisplayMode::Truncated if running_generic_card => ToolDisplayMode::Expanded,
            ToolDisplayMode::Truncated => ToolDisplayMode::Collapsed,
            ToolDisplayMode::Expanded if running_generic_card => ToolDisplayMode::Truncated,
            ToolDisplayMode::Expanded => ToolDisplayMode::Collapsed,
        };
        self.navigation.tool_modes.insert(id.to_owned(), next);
        self.restore_measured_anchor();
    }

    /// Re-anchor a manually scrolled viewport after a fold changes physical
    /// row count. The measurement is renderer-provided state delivered through
    /// the actor; without it, compatibility behavior remains unchanged.
    fn restore_measured_anchor(&mut self) {
        if self.navigation.autoscroll {
            return;
        }
        let Some(anchor) = self.navigation.measured_anchor_row else {
            return;
        };
        let half_viewport = self.navigation.measured_viewport_rows / 2;
        self.navigation.scroll_offset = anchor.saturating_sub(half_viewport);
    }

    fn scroll_by(&mut self, delta: i32) {
        if delta == 0 {
            return;
        }
        self.navigation.detach_from_tail();
        if delta.is_negative() {
            self.navigation.scroll_offset = self
                .navigation
                .scroll_offset
                .saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.navigation.scroll_offset =
                self.navigation.scroll_offset.saturating_add(delta as usize);
        }
    }
}
