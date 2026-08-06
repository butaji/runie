//! Scrollback widget: append-only transcript with autoscroll.

use std::collections::{HashMap, HashSet};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as RatLine, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use crate::appearance;
use runie_core::types::ThemeKind;

// Grok reserves a visible gutter between the first assistant row and its
// right-aligned clock before wrapping the remaining response text.
const TIMESTAMP_GUTTER_SPACES: usize = 3;

fn format_elapsed(elapsed_ms: Option<u64>) -> String {
    elapsed_ms
        .map(|millis| format!(" in {:.1}s", millis as f64 / 1_000.0))
        .unwrap_or_default()
}

/// Grok's default dense activity-group budget. A zero budget is reserved for
/// the source-compatible "no truncation" configuration.
pub const GROK_GROUP_MAX_VISIBLE: usize = 10;

/// Classify consecutive tool members into Grok dense groups. The returned
/// tuples contain `(member_index, group_size)` for each tool id; non-tool
/// entries are represented by `None`. This is deliberately pure so the actor
/// reducer, renderer, and YAML oracle can share the same grouping semantics.
pub fn dense_tool_group_members(tool_ids: &[Option<&str>]) -> Vec<Option<(usize, usize)>> {
    let mut result = vec![None; tool_ids.len()];
    let mut start = 0;
    while start < tool_ids.len() {
        if tool_ids[start].is_none() {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < tool_ids.len() && tool_ids[end].is_some() {
            end += 1;
        }
        let size = tool_ids[start..end]
            .iter()
            .filter(|candidate| candidate.is_some())
            .count();
        for (member_index, slot) in result[start..end].iter_mut().enumerate() {
            if tool_ids[start + member_index].is_some() {
                *slot = Some((member_index, size));
            }
        }
        start = end;
    }
    result
}

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

impl LineKind {
    pub fn style(self) -> Style {
        self.style_for(ThemeKind::GrokNight)
    }

    pub fn style_for(self, theme: ThemeKind) -> Style {
        match self {
            LineKind::User => appearance::user_style_for(theme),
            // Grok uses a vertical transcript bar for assistant/reasoning
            // blocks; body text stays primary rather than green.
            LineKind::Assistant => appearance::base_style_for(theme),
            LineKind::Reasoning => {
                appearance::muted_style_for(theme).add_modifier(Modifier::DIM | Modifier::ITALIC)
            }
            LineKind::ThinkingStatus => {
                appearance::accent_style_for(theme).add_modifier(Modifier::BOLD)
            }
            LineKind::Tool => appearance::base_style_for(theme),
            LineKind::ToolRunning => appearance::accent_style_for(theme),
            LineKind::ToolError => appearance::error_style_for(theme),
            LineKind::ToolResult => appearance::success_style_for(theme),
            LineKind::ToolOutput => appearance::base_style_for(theme),
            LineKind::SessionStart => appearance::muted_style_for(theme),
            LineKind::System => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::Separator => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::TurnSummary => appearance::muted_style_for(theme).add_modifier(Modifier::DIM),
            LineKind::CompletedAssistant => appearance::base_style_for(theme),
            LineKind::Activity => appearance::accent_style_for(theme),
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            // Grok reserves a three-column transcript gutter before user
            // content: the cursor is at column 5 in the 80-column frame.
            LineKind::User => "   ❯ ",
            LineKind::Assistant => "┃  ",
            LineKind::Reasoning => "┃  ",
            LineKind::ThinkingStatus => "┃  ",
            LineKind::Tool => "◆ ",
            LineKind::ToolRunning => "◆ ",
            LineKind::ToolError => "◆ ",
            LineKind::ToolResult => "  ↳ ",
            // Structured Grok tools render terminal output directly below the
            // tool header, with a two-column indentation and no result arrow.
            LineKind::ToolOutput => "  ",
            LineKind::SessionStart => "   ",
            LineKind::System => "   * ",
            LineKind::Separator => "",
            LineKind::TurnSummary => "   ",
            LineKind::CompletedAssistant => "   ",
            LineKind::Activity => "❙  ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub tool_call_id: Option<String>,
    has_vpad: bool,
}

impl Line {
    pub fn new(kind: LineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
            tool_call_id: None,
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
}

/// Explicit reducer inputs for the actor-owned transcript projection.
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
    SetToolMode(String, runie_core::types::ToolDisplayMode),
    ToggleToolMode(String),
    SelectNextTool,
    SelectPreviousTool,
    SelectNextEntry,
    SelectPreviousEntry,
    ScrollBy(i32),
    MarkToolError(String),
    ReplaceLine(usize, String),
    ReplaceLastByKind(LineKind, String),
    AppendToLastByKind(LineKind, String),
    ToolStart {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scrollback {
    lines: Vec<Line>,
    autoscroll: bool,
    scroll_offset: usize,
    reasoning_expanded: bool,
    activity_expanded: bool,
    prompt_timestamp: Option<String>,
    settled_no_tool_phase: bool,
    tool_modes: HashMap<String, runie_core::types::ToolDisplayMode>,
    revealed_dense_groups: HashSet<String>,
    center_revealed_entry: bool,
    tool_names: HashMap<String, String>,
    theme: ThemeKind,
    animation_frame: usize,
    selected_tool_id: Option<String>,
    selected_entry: Option<usize>,
    follow_latest_user: bool,
    workflow_headers: HashMap<String, String>,
    workflow_phases: HashMap<String, Vec<(String, String)>>,
}

/// Read-only typed projection of one Grok tool block. It is rebuilt from the
/// actor-owned scrollback lines; it is not a second mutable source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBlock {
    pub tool_call_id: String,
    pub header: String,
    pub kind: ToolCardKind,
    pub output: Vec<String>,
    pub mode: runie_core::types::ToolDisplayMode,
    pub is_running: bool,
    pub is_error: bool,
}

/// Grok's specialized tool-card families as a read-only presentation
/// projection. Lifecycle ownership remains with core/event actors.
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

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "the pure tool-name vocabulary maps Grok aliases to one card family"
)]
fn tool_card_kind(header: &str) -> ToolCardKind {
    let lower = header.trim_start().to_ascii_lowercase();
    if matches!(lower.as_str(), "bash" | "shell" | "exec" | "run")
        || lower.starts_with("run ")
        || lower.starts_with("execute ")
    {
        ToolCardKind::Execute
    } else if matches!(lower.as_str(), "read" | "read_file") || lower.starts_with("read ") {
        ToolCardKind::Read
    } else if matches!(
        lower.as_str(),
        "edit" | "write" | "write_file" | "search_replace"
    ) || lower.starts_with("edit ")
        || lower.starts_with("write ")
    {
        ToolCardKind::Edit
    } else if matches!(lower.as_str(), "list_dir" | "list_files") || lower.starts_with("list ") {
        ToolCardKind::ListDir
    } else if matches!(lower.as_str(), "web_search" | "web-search")
        || lower.starts_with("web search ")
    {
        ToolCardKind::WebSearch
    } else if matches!(lower.as_str(), "search" | "grep" | "find") || lower.starts_with("search ") {
        ToolCardKind::Search
    } else if matches!(lower.as_str(), "web_fetch" | "web-fetch" | "fetch")
        || lower.starts_with("fetch ")
    {
        ToolCardKind::WebFetch
    } else if matches!(lower.as_str(), "memory_search" | "memory-search")
        || lower.starts_with("memory search ")
    {
        ToolCardKind::MemorySearch
    } else if matches!(lower.as_str(), "workflow" | "run_workflow" | "run-workflow")
        || lower.starts_with("workflow ")
    {
        ToolCardKind::Workflow
    } else if matches!(lower.as_str(), "todo" | "todo_write" | "todo-write")
        || lower.starts_with("todo ")
    {
        ToolCardKind::Todo
    } else if matches!(lower.as_str(), "use" | "use_tool" | "use-tool") || lower.starts_with("use ")
    {
        ToolCardKind::Use
    } else if matches!(lower.as_str(), "search_tools" | "search-tools")
        || lower.starts_with("search tools ")
    {
        ToolCardKind::SearchTools
    } else if matches!(lower.as_str(), "subagent" | "agent" | "task")
        || lower.starts_with("subagent ")
    {
        ToolCardKind::Background
    } else {
        ToolCardKind::Generic
    }
}

impl Scrollback {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            autoscroll: true,
            scroll_offset: 0,
            reasoning_expanded: false,
            activity_expanded: false,
            prompt_timestamp: None,
            settled_no_tool_phase: false,
            tool_modes: HashMap::new(),
            revealed_dense_groups: HashSet::new(),
            center_revealed_entry: false,
            tool_names: HashMap::new(),
            theme: ThemeKind::GrokNight,
            animation_frame: 0,
            selected_tool_id: None,
            selected_entry: None,
            follow_latest_user: false,
            workflow_headers: HashMap::new(),
            workflow_phases: HashMap::new(),
        }
    }

    pub fn set_theme(&mut self, theme: ThemeKind) {
        self.theme = theme;
    }

    /// Apply one explicit transcript transition. Actor implementations and
    /// compatibility callers share this reducer boundary.
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "the reducer keeps all explicit transcript messages in one readable match"
    )]
    pub fn apply(&mut self, message: ScrollbackMsg) -> Option<usize> {
        match message {
            ScrollbackMsg::Append(line) => return Some(self.append(line)),
            ScrollbackMsg::AppendTurnSummary(text) => {
                return Some(self.append(Line::new(LineKind::TurnSummary, text)));
            }
            ScrollbackMsg::Clear => {
                self.clear();
            }
            ScrollbackMsg::SetTheme(theme) => self.set_theme(theme),
            ScrollbackMsg::AdvanceAnimation => {
                self.animation_frame = self.animation_frame.wrapping_add(1);
            }
            ScrollbackMsg::RemoveKind(kind) => self.remove_kind(kind),
            ScrollbackMsg::NormalizeLiveCompletedAssistants => {
                self.normalize_live_completed_assistants()
            }
            ScrollbackMsg::AddLiveAssistantTimestamp(width) => {
                self.add_live_assistant_timestamp(width)
            }
            ScrollbackMsg::RemoveEmptyAfter(kind) => self.remove_empty_after(kind),
            ScrollbackMsg::NormalizeActivitySpacing => self.normalize_activity_spacing(),
            ScrollbackMsg::SetReasoningExpanded(expanded) => self.set_reasoning_expanded(expanded),
            ScrollbackMsg::SetActivityExpanded(expanded) => self.set_activity_expanded(expanded),
            ScrollbackMsg::ToggleActivityExpanded => {
                self.set_activity_expanded(!self.activity_expanded);
            }
            ScrollbackMsg::SetPromptTimestamp(timestamp) => self.set_prompt_timestamp(timestamp),
            ScrollbackMsg::SetFollowLatestUser(follow) => self.follow_latest_user = follow,
            ScrollbackMsg::SetToolName(tool_call_id, tool_name) => {
                self.tool_names.insert(tool_call_id, tool_name);
            }
            ScrollbackMsg::SetToolMode(id, mode) => self.set_tool_mode(id, mode),
            ScrollbackMsg::ToggleToolMode(id) => self.toggle_tool_mode(&id),
            ScrollbackMsg::SelectNextTool => self.select_tool(1),
            ScrollbackMsg::SelectPreviousTool => self.select_tool(-1),
            ScrollbackMsg::SelectNextEntry => self.select_entry(1),
            ScrollbackMsg::SelectPreviousEntry => self.select_entry(-1),
            ScrollbackMsg::ScrollBy(lines) => self.scroll_by(lines),
            ScrollbackMsg::MarkToolError(id) => self.mark_tool_error(&id),
            ScrollbackMsg::ReplaceLine(index, text) => {
                if let Some(line) = self.line_mut(index) {
                    line.text = text;
                }
            }
            ScrollbackMsg::ReplaceLastByKind(kind, text) => {
                if let Some(line) = self.last_mut_by_kind(kind) {
                    line.text = text;
                }
            }
            ScrollbackMsg::AppendToLastByKind(kind, text) => {
                if let Some(line) = self.last_mut_by_kind(kind) {
                    line.text.push_str(&text);
                } else {
                    self.append(Line::new(kind, text));
                }
            }
            ScrollbackMsg::ToolStart {
                tool_call_id,
                header,
                activity,
            } => {
                self.replace_or_append_activity(activity);
                let kind = if header.starts_with("Subagent running:") {
                    LineKind::ToolRunning
                } else {
                    LineKind::Tool
                };
                self.append(Line::new(kind, header).for_tool(tool_call_id));
            }
            ScrollbackMsg::ToolUpdate {
                tool_call_id,
                header,
                output,
            } => {
                if let Some(header) = header {
                    self.replace_tool_by_id(&tool_call_id, header);
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
                self.finish_tool_by_id(&tool_call_id, header);
                for (kind, text) in output {
                    self.append(Line::new(kind, text).for_tool(&tool_call_id));
                }
                self.replace_or_append_activity(activity);
            }
            ScrollbackMsg::WorkflowStart {
                run_id,
                name,
                objective,
            } => {
                self.workflow_headers
                    .insert(run_id.clone(), format!("Workflow {name}: {objective}"));
                self.workflow_phases.insert(run_id.clone(), Vec::new());
                self.append(
                    Line::new(
                        LineKind::ToolRunning,
                        format!("Workflow {name}: {objective} [running] (0 agents)"),
                    )
                    .for_tool(run_id),
                );
            }
            ScrollbackMsg::WorkflowProgress {
                run_id,
                phase,
                state,
                active_agents,
            } => {
                let phases = self.workflow_phases.entry(run_id.clone()).or_default();
                if let Some(existing) = phases.iter_mut().find(|(title, _)| title == &phase) {
                    existing.1 = state.clone();
                } else {
                    phases.push((phase, state));
                }
                let trail = self
                    .workflow_phases
                    .get(&run_id)
                    .map(|items| {
                        items
                            .iter()
                            .map(|(title, state)| format!("{title}: {state}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                self.replace_tool_by_id(
                    &run_id,
                    format!(
                        "{} [{}] ({} agents)",
                        self.workflow_headers
                            .get(&run_id)
                            .map(String::as_str)
                            .unwrap_or("Workflow"),
                        trail,
                        active_agents
                    ),
                );
            }
            ScrollbackMsg::WorkflowEnd {
                run_id,
                status,
                elapsed_ms,
            } => {
                let trail = self
                    .workflow_phases
                    .get(&run_id)
                    .map(|items| {
                        items
                            .iter()
                            .map(|(title, state)| format!("{title}: {state}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                self.replace_tool_by_id(
                    &run_id,
                    format!(
                        "{} [{}] {}{}",
                        self.workflow_headers
                            .get(&run_id)
                            .map(String::as_str)
                            .unwrap_or("Workflow"),
                        trail,
                        status,
                        format_elapsed(elapsed_ms)
                    ),
                );
            }
            ScrollbackMsg::FinalizeAssistant {
                has_reasoning,
                reasoning_expanded,
                summary,
                settled_no_tool_phase,
            } => {
                self.settled_no_tool_phase = settled_no_tool_phase;
                if !has_reasoning || reasoning_expanded {
                    self.remove_kind(LineKind::ThinkingStatus);
                } else {
                    if let Some(thinking) = self.last_mut_by_kind(LineKind::ThinkingStatus) {
                        thinking.kind = LineKind::TurnSummary;
                        thinking.text = summary;
                    }
                    self.remove_kind(LineKind::Reasoning);
                }
            }
        }
        None
    }

    pub fn theme(&self) -> ThemeKind {
        self.theme
    }

    /// Project line storage into Grok's typed tool-block view. Ordering follows
    /// first appearance in the transcript, including parallel tool calls.
    #[allow(
        clippy::too_many_lines,
        reason = "keeps the pure line-to-block projection together"
    )]
    pub fn tool_blocks(&self) -> Vec<ToolBlock> {
        let mut blocks = Vec::new();
        for line in &self.lines {
            let Some(id) = line.tool_call_id.as_deref() else {
                continue;
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
                        kind: self.tool_names.get(id).map_or_else(
                            || tool_card_kind(&line.text),
                            |name| tool_card_kind(name),
                        ),
                        output: Vec::new(),
                        mode: self
                            .tool_modes
                            .get(id)
                            .copied()
                            .unwrap_or(runie_core::types::ToolDisplayMode::Expanded),
                        is_running: line.kind == LineKind::ToolRunning,
                        is_error: line.kind == LineKind::ToolError,
                    });
                }
                continue;
            };
            let block = &mut blocks[index];
            match line.kind {
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError => {
                    block.header = line.text.clone();
                    block.kind = self
                        .tool_names
                        .get(id)
                        .map_or_else(|| tool_card_kind(&line.text), |name| tool_card_kind(name));
                    block.is_running = line.kind == LineKind::ToolRunning;
                    block.is_error = line.kind == LineKind::ToolError;
                }
                LineKind::ToolOutput | LineKind::ToolResult => {
                    block.output.push(line.text.clone());
                }
                _ => {}
            }
        }
        blocks
    }

    fn replace_tool_by_id(&mut self, tool_call_id: &str, text: String) {
        if let Some(line) = self
            .lines
            .iter_mut()
            .rev()
            .find(|line| line.tool_call_id.as_deref() == Some(tool_call_id))
        {
            line.text = text;
        }
    }

    fn finish_tool_by_id(&mut self, tool_call_id: &str, text: String) {
        if let Some(line) = self
            .lines
            .iter_mut()
            .rev()
            .find(|line| line.tool_call_id.as_deref() == Some(tool_call_id))
        {
            line.text = text;
            if line.kind == LineKind::ToolRunning {
                line.kind = LineKind::Tool;
            }
        }
    }

    fn mark_tool_error(&mut self, tool_call_id: &str) {
        if let Some(line) = self
            .lines
            .iter_mut()
            .rev()
            .find(|line| line.tool_call_id.as_deref() == Some(tool_call_id))
        {
            line.kind = LineKind::ToolError;
        }
    }

    fn replace_or_append_activity(&mut self, activity: Option<String>) {
        let Some(activity) = activity else {
            return;
        };
        if let Some(line) = self.last_mut_by_kind(LineKind::Activity) {
            line.text = activity;
        } else {
            self.append(Line::new(LineKind::Activity, activity));
        }
    }

    pub fn append(&mut self, line: Line) -> usize {
        let index = self.lines.len();
        if line.kind == LineKind::User {
            self.follow_latest_user = true;
        }
        self.lines.push(line);
        if self.autoscroll {
            // Hold offset so the tail is in view after the next render
            // (the actual clamp happens in `render` once we know area height).
            self.scroll_offset = self.lines.len();
        }
        index
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.tool_names.clear();
        self.tool_modes.clear();
        self.workflow_headers.clear();
        self.workflow_phases.clear();
        self.revealed_dense_groups.clear();
        self.center_revealed_entry = false;
        self.scroll_offset = 0;
        self.selected_tool_id = None;
        self.selected_entry = None;
        self.follow_latest_user = false;
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Borrow the lines (for tests).
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn remove_kind(&mut self, kind: LineKind) {
        self.lines.retain(|line| line.kind != kind);
    }

    /// The live Grok feed drops the vertical assistant rail after a turn
    /// completes; replay fixtures retain the richer transcript rail.
    pub fn normalize_live_completed_assistants(&mut self) {
        for line in &mut self.lines {
            if line.kind == LineKind::Assistant && !line.text.is_empty() {
                line.kind = LineKind::CompletedAssistant;
            }
        }
    }

    pub fn add_live_assistant_timestamp(&mut self, width: usize) {
        // Kept as a compatibility reducer message; timestamp placement is a
        // pure view overlay in `physical_rows`, so event-owned text remains
        // immutable and does not wrap around the timestamp.
        let _ = width;
    }

    pub fn remove_empty_after(&mut self, kind: LineKind) {
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

    pub fn normalize_activity_spacing(&mut self) {
        let Some(index) = self
            .lines
            .iter()
            .position(|line| line.kind == LineKind::Activity)
        else {
            return;
        };
        let mut cursor = index + 1;
        while cursor < self.lines.len() {
            if self.lines[cursor].kind == LineKind::System && self.lines[cursor].text.is_empty() {
                self.lines.remove(cursor);
            } else {
                cursor += 1;
            }
        }
        self.lines
            .insert(index + 1, Line::new(LineKind::Separator, ""));
    }

    /// Set Grok-compatible reasoning display mode. Collapsed mode renders
    /// only `Thought`; expanded mode renders the captured reasoning body.
    pub fn set_reasoning_expanded(&mut self, expanded: bool) {
        self.reasoning_expanded = expanded;
    }

    pub fn reasoning_expanded(&self) -> bool {
        self.reasoning_expanded
    }

    /// Set Grok-compatible grouped-tool display mode. Collapsed mode keeps
    /// the activity summary and hides member tool/output rows.
    pub fn set_activity_expanded(&mut self, expanded: bool) {
        self.activity_expanded = expanded;
    }

    pub fn set_prompt_timestamp(&mut self, timestamp: Option<String>) {
        self.prompt_timestamp = timestamp;
    }

    pub fn activity_expanded(&self) -> bool {
        self.activity_expanded
    }

    /// Background work keeps its running bullet animated after the main turn
    /// becomes idle.
    pub fn animation_demand(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.kind == LineKind::ToolRunning)
    }

    pub fn set_tool_mode(
        &mut self,
        tool_call_id: impl Into<String>,
        mode: runie_core::types::ToolDisplayMode,
    ) {
        self.tool_modes.insert(tool_call_id.into(), mode);
    }

    /// Apply Grok's fold action to one selected tool block. The actor owns the
    /// transition; callers only publish the tool id as an intent.
    pub fn toggle_tool_mode(&mut self, tool_call_id: &str) {
        let next = match self
            .tool_modes
            .get(tool_call_id)
            .copied()
            .unwrap_or(runie_core::types::ToolDisplayMode::Expanded)
        {
            runie_core::types::ToolDisplayMode::Collapsed
            | runie_core::types::ToolDisplayMode::Truncated => {
                runie_core::types::ToolDisplayMode::Expanded
            }
            runie_core::types::ToolDisplayMode::Expanded => {
                runie_core::types::ToolDisplayMode::Collapsed
            }
        };
        self.set_tool_mode(tool_call_id, next);
    }

    pub fn selected_tool_id(&self) -> Option<&str> {
        self.selected_tool_id.as_deref()
    }

    pub fn selected_entry(&self) -> Option<usize> {
        self.selected_entry
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    fn selectable_entries(&self) -> Vec<usize> {
        let mut entries = Vec::new();
        let mut seen_tools = HashSet::new();
        for (index, line) in self.lines.iter().enumerate() {
            let selectable = match line.kind {
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError => line
                    .tool_call_id
                    .as_ref()
                    .is_none_or(|id| seen_tools.insert(id.clone())),
                LineKind::User | LineKind::Assistant | LineKind::Reasoning => true,
                _ => false,
            };
            if selectable {
                entries.push(index);
            }
        }
        entries
    }

    fn select_entry(&mut self, direction: i8) {
        let entries = self.selectable_entries();
        if entries.is_empty() {
            self.selected_entry = None;
            return;
        }
        let current = self
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
        self.selected_entry = Some(entries[next]);
        self.selected_tool_id = self.lines[entries[next]].tool_call_id.clone();
        self.autoscroll = false;
        self.follow_latest_user = false;
    }

    /// Apply Grok's explicit Ctrl+j/Ctrl+k viewport scroll intent. The actor
    /// owns the offset and hands follow mode off to the user once scrolling
    /// begins; rendering only clamps it against measured physical rows.
    pub fn scroll_by(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        self.autoscroll = false;
        self.follow_latest_user = false;
        if lines.is_negative() {
            self.scroll_offset = self
                .scroll_offset
                .saturating_sub(lines.unsigned_abs() as usize);
        } else {
            self.scroll_offset = self.scroll_offset.saturating_add(lines as usize);
        }
    }

    fn select_tool(&mut self, direction: i8) {
        let ids = self
            .tool_blocks()
            .into_iter()
            .map(|block| block.tool_call_id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            self.selected_tool_id = None;
            return;
        }
        let current = self
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
        self.selected_tool_id = Some(ids[next].clone());
        self.reveal_dense_group(&ids[next]);
    }

    /// Selecting any member is Grok's reveal operation: the dense group is
    /// keyed by its first member and all hidden members become renderable.
    fn reveal_dense_group(&mut self, tool_id: &str) {
        let groups = self.dense_tool_groups();
        let Some((member_index, group_size)) = groups.get(tool_id) else {
            return;
        };
        if *group_size > GROK_GROUP_MAX_VISIBLE
            && *member_index < group_size - GROK_GROUP_MAX_VISIBLE
        {
            self.selected_entry = self
                .lines
                .iter()
                .position(|line| line.tool_call_id.as_deref() == Some(tool_id));
            self.center_revealed_entry = true;
            if let Some(anchor) = groups.iter().find_map(|(id, (index, size))| {
                (*size == *group_size && *index == 0).then_some(id.clone())
            }) {
                self.revealed_dense_groups.insert(anchor);
            }
        }
    }

    /// Find the index of the first line whose `text` contains the needle.
    pub fn find_first_containing(&self, needle: &str) -> Option<usize> {
        self.lines.iter().position(|l| l.text.contains(needle))
    }

    /// Find all line indices whose `text` contains the needle.
    pub fn find_all_containing(&self, needle: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                if l.text.contains(needle) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Mutable reference to the last line of `kind`, if any.
    pub fn last_mut_by_kind(&mut self, kind: LineKind) -> Option<&mut Line> {
        self.lines.iter_mut().rev().find(|l| l.kind == kind)
    }

    pub fn line_mut(&mut self, index: usize) -> Option<&mut Line> {
        self.lines.get_mut(index)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "render keeps viewport selection and cell projection together"
    )]
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render_with_terminal_height(area, 0, buf);
    }

    /// Render using the full terminal height for Grok's responsive mode.
    /// `0` preserves the unmeasured compatibility behavior used by isolated
    /// widget tests; application callers should pass the outer frame height.
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "render keeps responsive layout, selection styling, and cell projection together"
    )]
    pub fn render_with_terminal_height(
        &mut self,
        area: Rect,
        terminal_rows: u16,
        buf: &mut Buffer,
    ) {
        // Wrap-aware: each Line is one logical row that may wrap to multiple
        // physical rows. We approximate by giving each line 1 "slot" plus
        // overflow based on text length and area width.
        let compact = crate::layout::grok_effective_compact(false, terminal_rows);
        let mut physical_rows = self.physical_rows(area.width as usize, compact, area.height);
        // Grok reserves one visual lead row for a submitted prompt. Live
        // event sequences normally already contain a separator; direct/YAML
        // reducer inputs may not, so add the lead only to the projection and
        // never to actor-owned logical state.
        if self.follow_latest_user {
            if let Some(user_row) = physical_rows
                .iter()
                .position(|(kind, _, _)| *kind == LineKind::User)
            {
                if user_row == 0 || physical_rows[user_row - 1].0 != LineKind::Separator {
                    physical_rows.insert(user_row, (LineKind::Separator, String::new(), false));
                }
            }
        }
        let total = physical_rows.len();
        let visible = area.height as usize;
        let compact_scroll_lead =
            if total > visible + crate::layout::COMPACT_SCROLL_OVERFLOW_THRESHOLD {
                crate::layout::COMPACT_SCROLL_OVERFLOW_LEAD_ROWS
            } else {
                crate::layout::COMPACT_SCROLL_LEAD_ROWS
            };
        // Clamp scroll_offset so the tail is visible.
        if total > visible {
            let max_offset = total - visible;
            if self.autoscroll {
                self.scroll_offset = if area.width < 50 {
                    max_offset.saturating_sub(compact_scroll_lead)
                } else {
                    max_offset
                };
            } else if self.scroll_offset > max_offset {
                self.scroll_offset = max_offset;
            }
        } else {
            self.scroll_offset = 0;
        }

        if let Some(selected_text) = self
            .selected_entry
            .and_then(|index| self.lines.get(index).map(|line| line.text.as_str()))
        {
            if !selected_text.is_empty() {
                if let Some(selected_row) = physical_rows
                    .iter()
                    .position(|(_, text, _)| text.contains(selected_text))
                {
                    if self.center_revealed_entry {
                        let max_offset = total.saturating_sub(visible);
                        self.scroll_offset =
                            selected_row.saturating_sub(visible / 2).min(max_offset);
                        self.center_revealed_entry = false;
                    } else if selected_row < self.scroll_offset {
                        self.scroll_offset = selected_row;
                    } else if selected_row >= self.scroll_offset + visible {
                        self.scroll_offset = selected_row.saturating_sub(visible - 1);
                    }
                }
            }
        }
        let start = if self.follow_latest_user {
            physical_rows
                .iter()
                .rposition(|(kind, _, _)| *kind == LineKind::User)
                .map(|last_user_row| {
                    let mut user_row = last_user_row;
                    while user_row > 0 && physical_rows[user_row - 1].0 == LineKind::User {
                        user_row -= 1;
                    }
                    let anchored = if user_row > 0 && physical_rows[user_row - 1].1.is_empty() {
                        user_row - 1
                    } else {
                        user_row
                    };
                    // Keep a newly submitted prompt at the top while the
                    // response fits. Once incoming content outgrows the
                    // viewport, follow the tail so new output remains visible.
                    let incoming_rows = total.saturating_sub(anchored.saturating_add(1));
                    if incoming_rows > visible {
                        total.saturating_sub(visible)
                    } else {
                        anchored
                    }
                })
                .unwrap_or(self.scroll_offset)
        } else {
            self.scroll_offset
        };
        let end = (start + visible).min(total);
        let selected_non_tool_text = self.selected_entry.and_then(|index| {
            self.lines.get(index).and_then(|line| {
                if line.tool_call_id.is_none() {
                    Some(line.text.as_str())
                } else {
                    None
                }
            })
        });

        if start >= end {
            // Nothing to render. Avoid passing an empty slice to ratatui's
            // Paragraph/Line, which can panic on some versions.
            return;
        }

        let mut selected_non_tool_row = None;
        for (row, (kind, text, code_row)) in physical_rows[start..end].iter().enumerate() {
            let line = if *code_row {
                styled_code_line(text, self.theme)
            } else {
                styled_line_for(*kind, text, self.theme)
            };
            let mut line = line;
            let selected_row = text.starts_with("› ")
                || text.starts_with("⌄ ")
                || selected_non_tool_text.is_some_and(|value| text.contains(value));
            if selected_row {
                let selected_style = appearance::selected_style_for(self.theme);
                for span in &mut line.spans {
                    span.style = span.style.patch(selected_style);
                }
            }
            Paragraph::new(line).wrap(Wrap { trim: false }).render(
                Rect {
                    x: area.x,
                    y: area.y + row as u16,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
            if *kind == LineKind::User {
                let user_style = appearance::user_style_for(self.theme);
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(user_style));
                    }
                }
            }
            if matches!(*kind, LineKind::ToolOutput | LineKind::ToolResult)
                && (text.starts_with('+') || text.starts_with('-'))
            {
                let diff_style = if text.starts_with('+') {
                    appearance::diff_insert_style_for(self.theme)
                } else {
                    appearance::diff_delete_style_for(self.theme)
                };
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(diff_style));
                    }
                }
            }
            if selected_row {
                let selected_style = appearance::selected_style_for(self.theme);
                for column in area.x..area.x.saturating_add(area.width) {
                    if let Some(cell) = buf.cell_mut((column, area.y + row as u16)) {
                        cell.set_style(cell.style().patch(selected_style));
                    }
                }
                if selected_non_tool_text.is_some_and(|value| text.contains(value)) {
                    selected_non_tool_row = Some(area.y + row as u16);
                }
            }
        }
        if let Some(inner_y) = selected_non_tool_row {
            let border_style = appearance::selected_border_style_for(self.theme);
            let left = area.x;
            let right = area.x + area.width.saturating_sub(1);
            if let Some(cell) = buf.cell_mut((left, inner_y)) {
                cell.set_symbol("│").set_style(border_style);
            }
            if right > left {
                if let Some(cell) = buf.cell_mut((right, inner_y)) {
                    cell.set_symbol("│").set_style(border_style);
                }
            }
            if inner_y > area.y {
                let top = inner_y - 1;
                for x in left..=right {
                    if let Some(cell) = buf.cell_mut((x, top)) {
                        cell.set_symbol(if x == left {
                            "┌"
                        } else if x == right {
                            "┐"
                        } else {
                            "─"
                        })
                        .set_style(border_style);
                    }
                }
            }
            let bottom = inner_y.saturating_add(1);
            if bottom < area.y.saturating_add(area.height) {
                for x in left..=right {
                    if let Some(cell) = buf.cell_mut((x, bottom)) {
                        cell.set_symbol(if x == left {
                            "└"
                        } else if x == right {
                            "┘"
                        } else {
                            "─"
                        })
                        .set_style(border_style);
                    }
                }
            }
        }
    }

    #[allow(
        clippy::cognitive_complexity,
        clippy::too_many_lines,
        reason = "physical row projection keeps fold, markdown, and wrapping rules together"
    )]
    fn physical_rows(
        &self,
        width: usize,
        compact: bool,
        available_height: u16,
    ) -> Vec<(LineKind, String, bool)> {
        let mut rows = Vec::new();
        let mut code_block = false;
        let mut truncated_output = HashSet::new();
        let dense_groups = self.dense_tool_groups();
        let mut emitted_dense_headers = HashSet::new();
        let mut user_vpad_emitted = false;
        let mut skip_full_user_separator = false;
        for (line_index, line) in self.lines.iter().enumerate() {
            if width >= 50
                && self.settled_no_tool_phase
                && line.kind == LineKind::Separator
                && self
                    .lines
                    .get(line_index + 1)
                    .is_some_and(|next| next.kind == LineKind::TurnSummary)
            {
                continue;
            }
            if width < 50
                && matches!(line.kind, LineKind::System | LineKind::Separator)
                && line.text.is_empty()
                && self
                    .lines
                    .get(line_index + 1)
                    .is_some_and(|next| next.kind == LineKind::TurnSummary)
            {
                continue;
            }
            if width >= 70
                && skip_full_user_separator
                && matches!(line.kind, LineKind::System | LineKind::Separator)
                && line.text.is_empty()
            {
                skip_full_user_separator = false;
                continue;
            }
            if line.has_vpad()
                && width >= 70
                && available_height >= 3
                && !compact
                && !user_vpad_emitted
            {
                // Grok's prompt block enables vertical padding in full mode;
                // the narrow pager variant suppresses it.
                rows.push((LineKind::System, String::new(), false));
                user_vpad_emitted = true;
                skip_full_user_separator = true;
            }
            if width >= 50 && line.kind == LineKind::TurnSummary {
                rows.push((LineKind::System, String::new(), false));
            }
            let tool_mode = line
                .tool_call_id
                .as_ref()
                .and_then(|id| self.tool_modes.get(id));
            if self.activity_expanded {
                if let Some(tool_id) = line.tool_call_id.as_deref() {
                    if let Some((member_index, group_size)) = dense_groups.get(tool_id) {
                        let hidden = group_size.saturating_sub(GROK_GROUP_MAX_VISIBLE);
                        let group_revealed = dense_groups
                            .iter()
                            .find(|(_, (index, size))| *index == 0 && *size == *group_size)
                            .is_some_and(|(anchor, _)| self.revealed_dense_groups.contains(anchor));
                        if !group_revealed && hidden > 0 && *member_index < hidden {
                            if emitted_dense_headers.insert(tool_id.to_owned()) {
                                rows.push((
                                    LineKind::Activity,
                                    format!("╶╶ {} more", hidden.saturating_sub(1)),
                                    false,
                                ));
                            }
                            continue;
                        }
                    }
                }
            }
            if matches!(
                tool_mode,
                Some(runie_core::types::ToolDisplayMode::Collapsed)
            ) && matches!(
                line.kind,
                LineKind::ToolOutput | LineKind::ToolResult | LineKind::ToolError
            ) {
                continue;
            }
            if matches!(
                tool_mode,
                Some(runie_core::types::ToolDisplayMode::Truncated)
            ) && matches!(
                line.kind,
                LineKind::ToolOutput | LineKind::ToolResult | LineKind::ToolError
            ) && line
                .tool_call_id
                .as_ref()
                .is_some_and(|id| !truncated_output.insert(id.clone()))
            {
                continue;
            }
            if !self.activity_expanded
                && matches!(
                    line.kind,
                    LineKind::Tool
                        | LineKind::ToolRunning
                        | LineKind::ToolOutput
                        | LineKind::ToolResult
                        | LineKind::ToolError
                )
                && line.text != "session_start"
                && !line
                    .tool_call_id
                    .as_ref()
                    .and_then(|id| self.tool_modes.get(id))
                    .is_some_and(|mode| *mode != runie_core::types::ToolDisplayMode::Collapsed)
            {
                continue;
            }
            let selected = line
                .tool_call_id
                .as_ref()
                .is_some_and(|id| self.selected_tool_id.as_ref() == Some(id));
            let source = if line.kind == LineKind::Reasoning && !self.reasoning_expanded {
                "Thought".to_owned()
            } else {
                line.text.clone()
            };
            let fence = line.kind == LineKind::Assistant && is_fence(&source);
            let parts: Vec<_> = source.split('\n').collect();
            for (index, part) in parts.iter().enumerate() {
                let prefix = if line.kind == LineKind::TurnSummary && width >= 50 {
                    if width < 70 {
                        "   "
                    } else {
                        "     "
                    }
                } else if width < 70
                    && matches!(
                        line.kind,
                        LineKind::Assistant | LineKind::Reasoning | LineKind::ThinkingStatus
                    )
                {
                    "┃"
                } else if selected
                    && matches!(
                        line.kind,
                        LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
                    )
                {
                    match tool_mode {
                        Some(runie_core::types::ToolDisplayMode::Collapsed) => "› ",
                        _ => "⌄ ",
                    }
                } else if line.kind == LineKind::ToolRunning {
                    running_bullet(self.animation_frame)
                } else {
                    line.kind.prefix()
                };
                let mut text = if part.is_empty() {
                    String::new()
                } else if index == 0 {
                    format!("{prefix}{part}")
                } else {
                    format!("{}{}", " ".repeat(prefix.chars().count()), part)
                };
                if line.kind == LineKind::CompletedAssistant && index == 0 {
                    if let Some(timestamp) = self.prompt_timestamp.as_deref() {
                        let timestamp_width = timestamp.chars().count();
                        let target =
                            width.saturating_sub(timestamp_width + TIMESTAMP_GUTTER_SPACES);
                        if text.chars().count() > target {
                            let chars: Vec<_> = text.chars().collect();
                            let mut split = target.min(chars.len());
                            while split > 0 && split < chars.len() && !chars[split].is_whitespace()
                            {
                                split -= 1;
                            }
                            let head: String = chars[..split].iter().collect();
                            let rest: String = chars[split..].iter().collect();
                            rows.push((
                                LineKind::CompletedAssistant,
                                format!(
                                    "{head}{:>reserved$}",
                                    timestamp,
                                    reserved = timestamp_width + TIMESTAMP_GUTTER_SPACES + 1
                                ),
                                false,
                            ));
                            append_wrapped_words(
                                &mut rows,
                                line.kind,
                                format!(
                                    "{}{}",
                                    " ".repeat(line.kind.prefix().chars().count()),
                                    rest.trim_start()
                                ),
                                width.saturating_sub(timestamp_width + TIMESTAMP_GUTTER_SPACES + 3),
                            );
                            continue;
                        }
                        const ASSISTANT_TIMESTAMP_EDGE_OFFSET: usize = 1;
                        let padding = width
                            .saturating_sub(text.chars().count() + timestamp_width)
                            .saturating_add(ASSISTANT_TIMESTAMP_EDGE_OFFSET);
                        text.push_str(&" ".repeat(padding));
                        text.push_str(timestamp);
                    }
                }
                if line.kind == LineKind::User && index == 0 && self.prompt_timestamp.is_some() {
                    append_user_with_timestamp(
                        &mut rows,
                        text,
                        self.prompt_timestamp.as_deref().unwrap_or_default(),
                        width,
                    );
                } else {
                    append_wrapped(&mut rows, line.kind, text, code_block && !fence, width);
                }
                if line.kind == LineKind::Assistant
                    && is_table_row(part)
                    && !is_table_separator(part)
                    && parts.get(index + 1).is_none_or(|next| !is_table_row(next))
                {
                    let border_prefix = if index == 0 {
                        prefix.to_owned()
                    } else {
                        " ".repeat(prefix.chars().count())
                    };
                    let border = format!("{}{}", border_prefix, table_bottom_border(part));
                    append_wrapped(&mut rows, line.kind, border, false, width);
                }
            }
            if fence {
                code_block = !code_block;
            }
        }
        rows
    }

    /// Build the ordered, consecutive tool-member groups used by Grok's
    /// `N more` projection. Outputs remain attached to their member id and
    /// therefore disappear with that member instead of consuming budget.
    fn dense_tool_groups(&self) -> HashMap<String, (usize, usize)> {
        let mut groups = HashMap::new();
        let mut members = Vec::new();
        let mut flush = |members: &mut Vec<String>| {
            let size = members.len();
            for (index, id) in members.drain(..).enumerate() {
                groups.insert(id, (index, size));
            }
        };
        for line in &self.lines {
            let is_member = matches!(
                line.kind,
                LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
            ) && line.tool_call_id.is_some();
            if is_member {
                if let Some(id) = &line.tool_call_id {
                    if !members.iter().any(|member| member == id) {
                        members.push(id.clone());
                    }
                }
            } else if !matches!(
                line.kind,
                LineKind::Activity | LineKind::ToolOutput | LineKind::ToolResult
            ) {
                flush(&mut members);
            }
        }
        flush(&mut members);
        groups
    }
}

fn append_wrapped(
    rows: &mut Vec<(LineKind, String, bool)>,
    kind: LineKind,
    text: String,
    code: bool,
    width: usize,
) {
    if width == 0 || text.chars().count() <= width {
        rows.push((kind, text, code));
        return;
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        let head: String = chars.drain(..width.min(chars.len())).collect();
        rows.push((kind, head, code));
    }
}

fn append_user_with_timestamp(
    rows: &mut Vec<(LineKind, String, bool)>,
    text: String,
    timestamp: &str,
    width: usize,
) {
    // Grok reserves a timestamp gutter when deciding where long prompts wrap,
    // then right-aligns the timestamp to the feed's terminal edge.
    let timestamp_width = timestamp.chars().count();
    const PROMPT_TIMESTAMP_WRAP_GUTTER: usize = 8;
    let first_width = width.saturating_sub(timestamp_width + PROMPT_TIMESTAMP_WRAP_GUTTER);
    let mut chars: Vec<char> = text.chars().collect();
    let mut split = first_width.min(chars.len());
    while split > 0 && split < chars.len() && !chars[split].is_whitespace() {
        split -= 1;
    }
    let first: String = chars.drain(..split).collect();
    const TIMESTAMP_EDGE_OFFSET: usize = 2;
    let padding = width
        .saturating_sub(first.chars().count() + timestamp_width)
        .saturating_sub(TIMESTAMP_EDGE_OFFSET);
    rows.push((
        LineKind::User,
        format!("{first}{blank}{timestamp}", blank = " ".repeat(padding)),
        false,
    ));
    let indent = " ".repeat(LineKind::User.prefix().chars().count());
    let rest: String = chars.into_iter().collect();
    append_wrapped_words(
        rows,
        LineKind::User,
        format!("{indent}{}", rest.trim_start()),
        first_width,
    );
}

fn append_wrapped_words(
    rows: &mut Vec<(LineKind, String, bool)>,
    kind: LineKind,
    text: String,
    width: usize,
) {
    let leading: String = text.chars().take_while(|ch| ch.is_whitespace()).collect();
    let mut line = leading.clone();
    for word in text.split_whitespace() {
        let candidate = if line.trim().is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.trim().is_empty() && candidate.chars().count() > width {
            rows.push((kind, std::mem::replace(&mut line, leading.clone()), false));
        }
        if line.trim().is_empty() {
            line.push_str(word);
        } else {
            line.push(' ');
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        rows.push((kind, line, false));
    }
}

/// Render the small CommonMark subset that is visible in Grok's normal
/// transcript: headings, bullets, inline emphasis, and inline code. Keeping
/// this at the widget boundary means replay events remain the single source
/// of truth and no test needs a terminal process.
fn is_fence(text: &str) -> bool {
    text.trim_start()
        .strip_prefix("┃ ")
        .unwrap_or(text)
        .starts_with("```")
}

fn styled_code_line(text: &str, theme: ThemeKind) -> RatLine<'static> {
    RatLine::from(Span::styled(
        text.to_owned(),
        appearance::base_style_for(theme).add_modifier(Modifier::DIM),
    ))
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "semantic feed card styling keeps Grok header variants together"
)]
#[cfg(test)]
fn styled_line(kind: LineKind, text: &str) -> RatLine<'static> {
    styled_line_for(kind, text, ThemeKind::GrokNight)
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "semantic feed card styling keeps Grok header variants together"
)]
fn styled_line_for(kind: LineKind, text: &str, theme: ThemeKind) -> RatLine<'static> {
    let style = kind.style_for(theme);
    if kind == LineKind::User {
        let pointer = text.find('❯');
        if let Some(pointer) = pointer {
            let body_start = pointer + '❯'.len_utf8();
            return RatLine::from(vec![
                Span::styled(text[..body_start].to_owned(), style),
                Span::styled(text[body_start..].to_owned(), style),
            ]);
        }
    }
    if kind == LineKind::TurnSummary && text.contains("◆ Thought") {
        return styled_thought_summary(text, style);
    }
    if matches!(
        kind,
        LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
    ) {
        let (header_start, marker_len) = text
            .find("◆ ")
            .map(|start| (start, "◆ ".len()))
            .or_else(|| text.find("› ").map(|start| (start, "› ".len())))
            .or_else(|| text.find("⌄ ").map(|start| (start, "⌄ ".len())))
            .unwrap_or((usize::MAX, 0));
        if header_start == usize::MAX {
            return RatLine::from(text.to_owned()).style(style);
        }
        let split = header_start + marker_len;
        let (prefix, body) = text.split_at(split);
        let name_end = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        let name = &body[..name_end];
        let rest = &body[name_end..];
        return RatLine::from(vec![
            Span::styled(prefix.to_owned(), style),
            Span::styled(name.to_owned(), style.add_modifier(Modifier::BOLD)),
            Span::styled(rest.to_owned(), style),
        ]);
    }
    if kind == LineKind::SessionStart {
        let Some(header_start) = text.find("◆ ") else {
            return RatLine::from(text.to_owned()).style(style);
        };
        let split = header_start + "◆ ".len();
        let (prefix, body) = text.split_at(split);
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        let name = &body[..name_end];
        let rest = &body[name_end..];
        return RatLine::from(vec![
            Span::styled(prefix.to_owned(), style),
            Span::styled(name.to_owned(), style.add_modifier(Modifier::BOLD)),
            Span::styled(rest.to_owned(), style),
        ]);
    }
    if kind == LineKind::Activity {
        return styled_activity_line(text, style);
    }
    if matches!(kind, LineKind::ToolOutput | LineKind::ToolResult) {
        let diff_style = if text.starts_with('+') && !text.starts_with("+++") {
            Some(appearance::diff_insert_style_for(theme))
        } else if text.starts_with('-') && !text.starts_with("---") {
            Some(appearance::diff_delete_style_for(theme))
        } else if text.starts_with("@@") {
            Some(appearance::accent_style_for(theme))
        } else {
            None
        };
        if let Some(diff_style) = diff_style {
            return RatLine::from(text.to_owned()).style(diff_style);
        }
    }
    if kind != LineKind::Assistant {
        return RatLine::from(text.to_owned()).style(style);
    }
    styled_assistant_line(text, style)
}

const RUNNING_BULLETS: [&str; 4] = ["⋅ ", ": ", "⸬ ", "⁙ "];

fn running_bullet(frame: usize) -> &'static str {
    RUNNING_BULLETS[frame % RUNNING_BULLETS.len()]
}

fn styled_thought_summary(text: &str, style: Style) -> RatLine<'static> {
    let start = text.find("◆ Thought").expect("thought marker");
    let bold_start = start + "◆ ".len();
    let end = bold_start + "Thought".len();
    RatLine::from(vec![
        Span::styled(text[..bold_start].to_owned(), style),
        Span::styled(
            text[bold_start..end].to_owned(),
            style.add_modifier(Modifier::BOLD),
        ),
        Span::styled(text[end..].to_owned(), style),
    ])
}

fn styled_activity_line(text: &str, style: Style) -> RatLine<'static> {
    let Some(label_start) = text.find("◈ ") else {
        return RatLine::from(text.to_owned()).style(style);
    };
    let split = label_start + "◈ ".len();
    RatLine::from(vec![
        Span::styled(text[..split].to_owned(), style),
        Span::styled(text[split..].to_owned(), style.add_modifier(Modifier::BOLD)),
    ])
}

fn styled_assistant_line(text: &str, style: Style) -> RatLine<'static> {
    let (prefix, body) = text.split_at(
        text.find(|c: char| !c.is_whitespace())
            .unwrap_or(text.len()),
    );
    let mut spans = vec![Span::styled(prefix.to_owned(), style)];
    let body = if let Some(body) = body.strip_prefix("┃ ") {
        spans.push(Span::styled("┃ ".to_owned(), style));
        body
    } else if !body.is_empty() {
        // Wrapped/newline continuation rows retain the gutter as spaces, not
        // a second vertical marker. They still carry assistant markdown.
        body
    } else {
        return RatLine::from(text.to_owned()).style(style);
    };
    let mut body_spans = markdown_spans(body, style);
    spans.append(&mut body_spans);
    RatLine::from(spans)
}

fn markdown_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    if is_table_row(text) {
        return table_spans(text, base);
    }
    if text.starts_with("> ") {
        return blockquote_spans(text, base);
    }
    if text.starts_with("```") {
        return vec![Span::styled(
            text.to_owned(),
            base.add_modifier(Modifier::DIM | Modifier::UNDERLINED),
        )];
    }
    if let Some(title) = atx_heading(text) {
        return vec![Span::styled(
            title.to_owned(),
            base.add_modifier(Modifier::BOLD),
        )];
    }
    let (bullet, content) = text
        .strip_prefix("- ")
        .map(|rest| ("• ", rest))
        .or_else(|| text.strip_prefix("* ").map(|rest| ("• ", rest)))
        .or_else(|| {
            let split = text.find(". ")?;
            if text[..split].chars().all(|ch| ch.is_ascii_digit()) {
                Some(("• ", &text[split + 2..]))
            } else {
                None
            }
        })
        .unwrap_or(("", text));
    let mut spans = vec![Span::styled(bullet.to_owned(), base)];
    if let Some(inline) = inline_markdown(content, base) {
        spans.extend(inline);
        return spans;
    }
    spans.extend(bold_markdown(content, base));
    spans
}

fn is_table_row(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

fn table_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let cells: Vec<_> = text
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect();
    let separator = cells
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')));
    let (left, right) = if separator {
        ('├', '┤')
    } else {
        ('│', '│')
    };
    let body = if separator {
        cells
            .iter()
            .map(|cell| "─".repeat(cell.chars().count().max(1) + 2))
            .collect::<Vec<_>>()
            .join("┼")
    } else {
        cells.join(" │ ")
    };
    let rendered = if separator {
        format!("{left}{body}{right}")
    } else {
        format!("{left} {body} {right}")
    };
    vec![Span::styled(rendered, base.add_modifier(Modifier::DIM))]
}

fn is_table_separator(text: &str) -> bool {
    text.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

fn table_bottom_border(text: &str) -> String {
    let widths = text
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| "─".repeat(cell.trim().chars().count() + 2))
        .collect::<Vec<_>>();
    format!("└{}┘", widths.join("┴"))
}

fn atx_heading(text: &str) -> Option<&str> {
    let hashes = text.chars().take_while(|ch| *ch == '#').count();
    (1..=6)
        .contains(&hashes)
        .then(|| text.get(hashes..)?.strip_prefix(' '))
        .flatten()
}

fn blockquote_spans(text: &str, base: Style) -> Vec<Span<'static>> {
    let quote = text.strip_prefix("> ").unwrap_or(text);
    let mut spans = vec![Span::styled(
        "│ ".to_owned(),
        base.add_modifier(Modifier::DIM),
    )];
    spans.extend(markdown_spans(quote, base));
    spans
}

#[allow(
    clippy::too_many_lines,
    reason = "the inline markdown grammar stays together as one pure parser"
)]
fn inline_markdown(text: &str, base: Style) -> Option<Vec<Span<'static>>> {
    if let Some(start) = text.find('`') {
        if let Some(end) = text[start + 1..].find('`') {
            let end = start + end + 1;
            let mut spans = markdown_spans(&text[..start], base);
            spans.push(Span::styled(
                text[start + 1..end].to_owned(),
                base.add_modifier(Modifier::UNDERLINED),
            ));
            spans.extend(markdown_spans(&text[end + 1..], base));
            return Some(spans);
        }
    }
    for (delimiter, modifier) in [
        ("~~", Modifier::CROSSED_OUT),
        ("_", Modifier::ITALIC),
        ("*", Modifier::ITALIC),
    ] {
        if delimiter == "*" && text.contains("**") {
            continue;
        }
        if let Some(start) = text.find(delimiter) {
            if let Some(relative_end) = text[start + delimiter.len()..].find(delimiter) {
                let end = start + delimiter.len() + relative_end;
                let mut spans = markdown_spans(&text[..start], base);
                spans.push(Span::styled(
                    text[start + delimiter.len()..end].to_owned(),
                    base.add_modifier(modifier),
                ));
                spans.extend(markdown_spans(&text[end + delimiter.len()..], base));
                return Some(spans);
            }
        }
    }
    let start = text.find('[')?;
    let close = start + text[start + 1..].find(']')? + 1;
    if !text[close + 1..].starts_with('(') {
        return None;
    }
    let end = close + text[close + 2..].find(')')? + 2;
    let mut spans = markdown_spans(&text[..start], base);
    spans.push(Span::styled(
        text[start + 1..close].to_owned(),
        base.add_modifier(Modifier::UNDERLINED),
    ));
    spans.extend(markdown_spans(&text[end + 1..], base));
    Some(spans)
}

fn bold_markdown(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("**") {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_owned(), base));
        }
        let after = &rest[start + 2..];
        let Some(end) = after.find("**") else {
            spans.push(Span::styled(rest[start..].to_owned(), base));
            return spans;
        };
        spans.push(Span::styled(
            after[..end].to_owned(),
            base.add_modifier(Modifier::BOLD),
        ));
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_owned(), base));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollback_messages_reduce_explicit_feed_state() {
        let mut scrollback = Scrollback::new();
        let index = scrollback
            .apply(ScrollbackMsg::Append(Line::new(
                LineKind::Assistant,
                "hello",
            )))
            .expect("append returns its owned row");
        scrollback.apply(ScrollbackMsg::ReplaceLine(index, "updated".into()));
        assert_eq!(scrollback.lines()[index].text, "updated");
        scrollback.apply(ScrollbackMsg::Clear);
        assert!(scrollback.is_empty());
    }

    #[test]
    fn vpad_is_entry_metadata_not_line_kind_inference() {
        let user = Line::new(LineKind::User, "hi").with_vpad(true);
        let system = Line::new(LineKind::System, "system");
        assert!(user.has_vpad());
        assert!(!system.has_vpad());
    }

    #[test]
    fn terminal_height_controls_user_vpad_projection() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "hi").with_vpad(true));
        scrollback.append(Line::new(LineKind::Assistant, "answer"));

        let mut full = Buffer::empty(Rect::new(0, 0, 80, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 4), 24, &mut full);
        assert_eq!(full.cell((0, 0)).expect("full vpad row").symbol(), " ");
        assert_eq!(full.cell((0, 1)).expect("full user row").symbol(), " ");

        let mut compact = Buffer::empty(Rect::new(0, 0, 80, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 4), 16, &mut compact);
        assert_eq!(
            compact.cell((0, 0)).expect("compact user row").symbol(),
            " "
        );
        assert_eq!(
            compact
                .cell((0, 2))
                .expect("compact assistant row")
                .symbol(),
            "┃"
        );

        let mut clipped = Buffer::empty(Rect::new(0, 0, 80, 3));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 3), 24, &mut clipped);
        assert_eq!(
            clipped.cell((0, 0)).expect("clipped user row").symbol(),
            " "
        );
        assert_eq!(
            clipped
                .cell((0, 2))
                .expect("clipped assistant row")
                .symbol(),
            "┃"
        );
    }

    #[test]
    fn completed_assistant_timestamp_wraps_at_a_word_boundary() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(
            LineKind::CompletedAssistant,
            "Hey — what are you working on? I can help with code, tests, debugging, or anything else in this repo.",
        ));
        scrollback.set_prompt_timestamp(Some("2:11 AM".to_owned()));

        let rows = scrollback.physical_rows(58, false, 32);
        let first = rows
            .iter()
            .find(|(_, text, _)| text.contains("2:11 AM"))
            .expect("timestamp row")
            .1
            .clone();
        assert!(first.ends_with("2:11 AM"));
        assert!(!first.contains("help 2:11"));
        assert!(rows.iter().any(|(_, text, _)| text.contains("with code")));
    }
    use ratatui::style::Color;

    #[test]
    fn append_and_len() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn clear_empties() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::User, "hi"));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn find_first_containing() {
        let mut s = Scrollback::new();
        s.append(Line::new(LineKind::Assistant, "Hello world"));
        s.append(Line::new(LineKind::Assistant, "Goodbye world"));
        assert_eq!(s.find_first_containing("world"), Some(0));
    }

    #[test]
    fn visual_styles_are_stable() {
        assert_eq!(LineKind::User.style().fg, Some(Color::Rgb(225, 225, 225)));
        assert!(!LineKind::User.style().add_modifier.contains(Modifier::BOLD));
        assert_eq!(
            LineKind::Assistant.style().fg,
            Some(Color::Rgb(225, 225, 225))
        );
        assert_eq!(LineKind::Tool.style().fg, Some(Color::Rgb(225, 225, 225)));
        assert_eq!(
            LineKind::ToolError.style().fg,
            appearance::error_style_for(ThemeKind::GrokNight).fg
        );
        assert_eq!(
            LineKind::ToolResult.style().fg,
            Some(Color::Rgb(158, 206, 106))
        );
        assert!(LineKind::System
            .style()
            .add_modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn user_prompt_paints_grok_panel_background_across_the_row() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::SessionStart, "◆ session_start"));
        scrollback.append(Line::new(LineKind::User, "Hey"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 80, 4), 24, &mut buffer);
        let row = (0..4)
            .find(|row| {
                buffer
                    .cell((5, *row))
                    .is_some_and(|cell| cell.symbol() == "H")
            })
            .expect("user row");
        assert_eq!(
            buffer.cell((0, row)).expect("user row background").bg,
            Color::Rgb(36, 36, 36)
        );
        assert_eq!(
            buffer.cell((79, row)).expect("full user row background").bg,
            Color::Rgb(36, 36, 36)
        );
    }

    #[test]
    fn first_user_prompt_alone_follows_to_the_top_of_a_long_feed() {
        let mut scrollback = Scrollback::new();
        for index in 0..2 {
            scrollback.append(Line::new(LineKind::Assistant, format!("old {index}")));
        }
        scrollback.append(Line::new(LineKind::User, "Hey"));

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 4));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 40, 4), 24, &mut buffer);
        let first_row = (0..4)
            .find(|row| {
                buffer
                    .cell((5, *row))
                    .is_some_and(|cell| cell.symbol() == "H")
            })
            .expect("first submitted prompt remains visible");
        assert_eq!(first_row, 1);
    }

    #[test]
    fn newest_user_prompt_is_the_follow_anchor() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "first"));
        scrollback.append(Line::new(LineKind::Assistant, "answer"));
        scrollback.append(Line::new(LineKind::User, "second"));

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 40, 3), 24, &mut buffer);
        let mut visible = String::new();
        for row in 0..3 {
            for column in 0..40 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("second"),
            "newest prompt missing: {visible:?}"
        );
        assert!(
            !visible.contains("first"),
            "stale prompt anchored: {visible:?}"
        );
    }

    #[test]
    fn incoming_content_hands_following_from_prompt_to_tail() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "Hey"));
        for index in 0..6 {
            scrollback.append(Line::new(LineKind::Assistant, format!("reply {index}")));
        }

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 40, 3), 24, &mut buffer);
        let mut visible = String::new();
        for row in 0..3 {
            for column in 0..40 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("reply 5"),
            "tail was not followed: {visible:?}"
        );
        assert!(
            !visible.contains("Hey"),
            "prompt anchor blocked tail: {visible:?}"
        );
    }

    #[test]
    fn live_submission_sequence_keeps_timestamped_user_row_visible() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(LineKind::SessionStart, "◆ session_start"));
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(LineKind::User, "Hey").with_vpad(true));
        scrollback.set_prompt_timestamp(Some("6:20 AM".into()));
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(LineKind::ThinkingStatus, "◆ Thinking…"));
        scrollback.append(Line::new(LineKind::Separator, ""));
        scrollback.append(Line::new(
            LineKind::Assistant,
            "Hey — what are you working on? I can help with code.",
        ));
        scrollback.apply(ScrollbackMsg::FinalizeAssistant {
            has_reasoning: false,
            reasoning_expanded: false,
            summary: "Thought for 0.2s".into(),
            settled_no_tool_phase: true,
        });
        scrollback.remove_kind(LineKind::SessionStart);
        scrollback.normalize_live_completed_assistants();

        let mut buffer = Buffer::empty(Rect::new(0, 0, 76, 15));
        scrollback.render_with_terminal_height(Rect::new(0, 0, 76, 15), 24, &mut buffer);
        let mut visible = String::new();
        for row in 0..15 {
            for column in 0..76 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("❯ Hey"),
            "live user row missing: {visible:?}"
        );
        assert!(
            visible.contains("6:20 AM"),
            "user timestamp missing: {visible:?}"
        );
    }

    #[test]
    fn feed_style_uses_selected_theme_tokens() {
        assert_eq!(
            LineKind::Assistant.style_for(ThemeKind::GrokDay).fg,
            Some(Color::Rgb(38, 38, 38))
        );
        assert_eq!(
            LineKind::Activity.style_for(ThemeKind::GrokDay).fg,
            Some(Color::Rgb(125, 75, 198))
        );
        let mut scrollback = Scrollback::new();
        scrollback.set_theme(ThemeKind::GrokDay);
        assert_eq!(scrollback.theme(), ThemeKind::GrokDay);
    }

    #[test]
    fn edit_diff_rows_use_semantic_theme_tokens() {
        let inserted = styled_line_for(LineKind::ToolResult, "+new", ThemeKind::GrokDay);
        let deleted = styled_line_for(LineKind::ToolResult, "-old", ThemeKind::GrokDay);
        let hunk = styled_line_for(LineKind::ToolOutput, "@@ -1 +1 @@", ThemeKind::GrokDay);
        assert_eq!(
            inserted.style.fg,
            appearance::success_style_for(ThemeKind::GrokDay).fg
        );
        assert_eq!(inserted.style.bg, Some(Color::Rgb(218, 242, 220)));
        assert_eq!(
            deleted.style.fg,
            appearance::error_style_for(ThemeKind::GrokDay).fg
        );
        assert_eq!(deleted.style.bg, Some(Color::Rgb(245, 218, 222)));
        assert_eq!(
            hunk.style.fg,
            appearance::accent_style_for(ThemeKind::GrokDay).fg
        );
    }

    #[test]
    fn structured_tool_header_bolds_only_the_action_name() {
        let rendered = styled_line(LineKind::Tool, "   ◆ List .");
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(!rendered.spans[2]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn grok_activity_label_bolds_the_grouped_summary() {
        let rendered = styled_line(LineKind::Activity, "❙  ◈ Listed 1 dir, Read 1 file");
        assert!(!rendered.spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn embedded_newlines_render_as_separate_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "first\nsecond"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 4));
        scrollback.render(Rect::new(0, 0, 30, 4), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("first row").symbol(), "┃");
        assert_eq!(buffer.cell((0, 1)).expect("second row").symbol(), " ");
        let rendered: String = (0..30)
            .flat_map(|y| (0..30).map(move |x| (x, y)))
            .filter_map(|(x, y)| buffer.cell((x, y)))
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(rendered.contains("second"));
    }

    #[test]
    fn assistant_markdown_preserves_grok_bullets_and_bold_spans() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "# runie"));
        scrollback.append(Line::new(LineKind::Assistant, "- **fast** replay"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 3));
        scrollback.render(Rect::new(0, 0, 30, 3), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("heading prefix").symbol(), "┃");
        assert_eq!(buffer.cell((1, 0)).expect("heading").symbol(), "#");
        assert_eq!(buffer.cell((1, 1)).expect("bullet prefix").symbol(), "-");
        let bullet_row: String = (0..30)
            .filter_map(|column| buffer.cell((column, 1)))
            .map(|cell| cell.symbol().to_owned())
            .collect();
        assert!(bullet_row.contains("fast"));
    }

    #[test]
    fn assistant_markdown_styles_code_and_links() {
        let rendered = styled_line(LineKind::Assistant, "┃ use `cargo test` [docs](https://x)");
        assert!(rendered.spans.iter().any(|span| {
            span.content == "cargo test" && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
        assert!(rendered.spans.iter().any(|span| {
            span.content == "docs" && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
    }

    #[test]
    fn assistant_markdown_renders_blockquote_gutter() {
        let line = styled_assistant_line("> quoted **text**", Style::default());
        let rendered = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert_eq!(rendered, "│ quoted text");
        assert!(line.spans[1].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn assistant_markdown_bolds_all_atx_heading_levels() {
        for level in 1..=6 {
            let text = format!("{} title", "#".repeat(level));
            let line = styled_assistant_line(&text, Style::default());
            let heading = line.spans.last().expect("heading span");
            assert_eq!(heading.content, "title");
            assert!(heading.style.add_modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn assistant_markdown_renders_table_box_drawing_rows() {
        let header = styled_assistant_line("| Name | Type |", Style::default());
        let separator = styled_assistant_line("| ---- | ---- |", Style::default());
        assert_eq!(
            header.spans.last().expect("table header").content,
            "│ Name │ Type │"
        );
        assert_eq!(
            separator.spans.last().expect("table separator").content,
            "├──────┼──────┤"
        );

        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(
            LineKind::Assistant,
            "| Name | Type |\n| ---- | ---- |\n| runie | tool |",
        ));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 5));
        scrollback.render(Rect::new(0, 0, 30, 5), &mut buffer);
        let row = (0..30)
            .map(|column| {
                buffer
                    .cell((column, 3))
                    .expect("table border cell")
                    .symbol()
            })
            .collect::<String>();
        assert!(row.starts_with(" └───────┴──────┘"), "{row:?}");
    }

    #[test]
    fn assistant_markdown_styles_ordered_lists_and_emphasis() {
        let rendered = styled_line(LineKind::Assistant, "┃ 1. _quiet_ ~~old~~");
        assert!(rendered.spans.iter().any(|span| span.content == "• "));
        assert!(rendered.spans.iter().any(|span| {
            span.content == "quiet" && span.style.add_modifier.contains(Modifier::ITALIC)
        }));
        assert!(rendered.spans.iter().any(|span| {
            span.content == "old" && span.style.add_modifier.contains(Modifier::CROSSED_OUT)
        }));
    }

    #[test]
    fn assistant_markdown_styles_fence_markers() {
        let rendered = styled_line(LineKind::Assistant, "   ┃ ```rust");
        assert!(rendered.spans.iter().any(|span| {
            span.content == "```rust"
                && span.style.add_modifier.contains(Modifier::DIM)
                && span.style.add_modifier.contains(Modifier::UNDERLINED)
        }));
    }

    #[test]
    fn fenced_code_styles_interior_lines() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "```rust"));
        scrollback.append(Line::new(LineKind::Assistant, "cargo test"));
        scrollback.append(Line::new(LineKind::Assistant, "```"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 4));
        scrollback.render(Rect::new(0, 0, 30, 4), &mut buffer);
        assert!(buffer
            .cell((0, 1))
            .expect("code row")
            .modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn grok_user_feed_cursor_is_at_column_five() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::User, "Please list files"));
        let mut buffer = Buffer::empty(Rect::new(2, 0, 76, 2));
        scrollback.render(Rect::new(2, 0, 76, 2), &mut buffer);
        assert_eq!(buffer.cell((2, 0)).expect("gutter").symbol(), " ");
        assert_eq!(buffer.cell((5, 1)).expect("Grok user cursor").symbol(), "❯");
        assert_eq!(
            buffer.cell((7, 1)).expect("first user letter").symbol(),
            "P"
        );
    }

    #[test]
    fn turn_summary_uses_groks_column_six_gutter() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::TurnSummary, "Worked for 2.3s"));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        scrollback.render(Rect::new(0, 0, 30, 1), &mut buffer);
        assert_eq!(buffer.cell((0, 0)).expect("first gutter").symbol(), " ");
        assert_eq!(buffer.cell((2, 0)).expect("fifth gutter").symbol(), " ");
        assert_eq!(buffer.cell((3, 0)).expect("summary start").symbol(), "W");
    }

    #[test]
    fn live_completed_assistant_keeps_primary_body_style() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Assistant, "Hello from Runie"));
        scrollback.normalize_live_completed_assistants();

        assert_eq!(scrollback.lines()[0].kind, LineKind::CompletedAssistant);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        scrollback.render(Rect::new(0, 0, 30, 1), &mut buffer);
        let cell = buffer.cell((3, 0)).expect("assistant body");
        assert_eq!(cell.fg, Color::Rgb(225, 225, 225));
        assert!(!cell.modifier.contains(Modifier::DIM));
    }

    #[test]
    fn grok_user_pointer_uses_blue_accent_without_bold_body() {
        let rendered = styled_line(LineKind::User, "   ❯ hello");
        assert_eq!(rendered.spans[0].style.fg, Some(Color::Rgb(225, 225, 225)));
        assert!(!rendered.spans[0]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
        assert!(!rendered.spans[1]
            .style
            .add_modifier
            .contains(Modifier::BOLD));
    }

    #[test]
    fn reasoning_uses_grok_dim_italic_transcript_style() {
        let style = LineKind::Reasoning.style();
        assert!(style.add_modifier.contains(Modifier::DIM));
        assert!(style.add_modifier.contains(Modifier::ITALIC));
        assert_eq!(LineKind::Reasoning.prefix(), "┃  ");
    }

    #[test]
    fn reasoning_fold_has_deterministic_collapsed_and_expanded_cells() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Reasoning, "checking the request"));
        let mut collapsed = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut collapsed);
        assert_eq!(
            collapsed.cell((0, 0)).expect("collapsed gutter").symbol(),
            "┃"
        );
        assert_eq!(
            collapsed.cell((1, 0)).expect("collapsed label").symbol(),
            "T"
        );

        scrollback.set_reasoning_expanded(true);
        let mut expanded = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut expanded);
        assert_eq!(expanded.cell((1, 0)).expect("expanded body").symbol(), "c");
        assert!(expanded
            .cell((1, 0))
            .expect("expanded style")
            .modifier
            .contains(Modifier::DIM));
        assert!(expanded
            .cell((3, 0))
            .expect("expanded style")
            .modifier
            .contains(Modifier::ITALIC));
    }

    #[test]
    fn typed_tool_blocks_rebuild_from_parallel_actor_rows() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "a".into(),
            header: "Read a.txt".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "b".into(),
            header: "Run tests".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "b".into(),
            header: "Run tests (ok)".into(),
            activity: None,
            output: vec![(LineKind::ToolResult, "passed".into())],
        });
        scrollback.apply(ScrollbackMsg::SetToolMode(
            "a".into(),
            runie_core::types::ToolDisplayMode::Truncated,
        ));

        let blocks = scrollback.tool_blocks();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].tool_call_id, "a");
        assert_eq!(
            blocks[0].mode,
            runie_core::types::ToolDisplayMode::Truncated
        );
        assert!(!blocks[0].is_running);
        assert_eq!(blocks[1].output, vec!["passed"]);
        assert!(!blocks[1].is_running);
    }

    #[test]
    fn typed_tool_name_survives_header_rewrite() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::SetToolName("call-1".into(), "read".into()));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "call-1".into(),
            header: "Completed README.md".into(),
            activity: None,
            output: Vec::new(),
        });
        assert_eq!(scrollback.tool_blocks()[0].kind, ToolCardKind::Read);
    }

    #[test]
    fn selected_tool_fold_cycles_collapsed_and_expanded_modes() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Expanded
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Collapsed
        );
        scrollback.apply(ScrollbackMsg::ToggleToolMode("call-1".into()));
        assert_eq!(
            scrollback.tool_blocks()[0].mode,
            runie_core::types::ToolDisplayMode::Expanded
        );
    }

    #[test]
    fn tool_selection_wraps_in_transcript_order() {
        let mut scrollback = Scrollback::new();
        for id in ["first", "second"] {
            scrollback.apply(ScrollbackMsg::ToolStart {
                tool_call_id: id.into(),
                header: format!("Read {id}"),
                activity: None,
            });
        }
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("first"));
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("second"));
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("first"));
        scrollback.apply(ScrollbackMsg::SelectPreviousTool);
        assert_eq!(scrollback.selected_tool_id(), Some("second"));
    }

    #[test]
    fn entry_selection_navigates_semantic_rows_and_projects_tool_id() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::Append(Line::new(LineKind::User, "Hey")));
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::Append(Line::new(
            LineKind::Assistant,
            "Done",
        )));
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        assert_eq!(scrollback.selected_entry(), Some(0));
        assert_eq!(scrollback.selected_tool_id(), None);
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        assert_eq!(scrollback.selected_tool_id(), Some("call-1"));
        scrollback.apply(ScrollbackMsg::SelectPreviousEntry);
        assert_eq!(scrollback.selected_entry(), Some(0));
    }

    #[test]
    fn selected_non_tool_entry_paints_the_theme_selection_surface() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::Append(Line::new(LineKind::User, "Hey")));
        scrollback.apply(ScrollbackMsg::SelectNextEntry);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 2));
        scrollback.render(Rect::new(0, 0, 40, 2), &mut buffer);
        assert_eq!(
            buffer.cell((39, 0)).expect("selected row").bg,
            ratatui::style::Color::Rgb(28, 28, 28)
        );
        assert_eq!(buffer.cell((0, 0)).expect("selection border").symbol(), "│");
    }

    #[test]
    fn entry_navigation_reveals_selected_row_in_small_viewport() {
        let mut scrollback = Scrollback::new();
        for text in ["one", "two", "three", "four", "five"] {
            scrollback.apply(ScrollbackMsg::Append(Line::new(LineKind::User, text)));
        }
        for _ in 0..5 {
            scrollback.apply(ScrollbackMsg::SelectNextEntry);
        }
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
        scrollback.render(Rect::new(0, 0, 20, 2), &mut buffer);
        let mut visible = String::new();
        for row in 0..2 {
            for column in 0..20 {
                if let Some(cell) = buffer.cell((column, row)) {
                    visible.push_str(cell.symbol());
                }
            }
        }
        assert!(
            visible.contains("five"),
            "selected row not revealed: {visible:?}"
        );
    }

    #[test]
    fn explicit_scroll_intent_hands_off_from_autoscroll() {
        let mut scrollback = Scrollback::new();
        for index in 0..8 {
            scrollback.append(Line::new(LineKind::Assistant, format!("row {index}")));
        }
        scrollback.apply(ScrollbackMsg::ScrollBy(2));
        assert!(!scrollback.autoscroll);
        assert!(!scrollback.follow_latest_user);
        assert_eq!(scrollback.scroll_offset, 10);
        scrollback.apply(ScrollbackMsg::ScrollBy(-1));
        assert_eq!(scrollback.scroll_offset, 9);
    }

    #[test]
    fn clear_resets_actor_owned_tool_selection() {
        let mut scrollback = Scrollback::new();
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "call-1".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        assert_eq!(scrollback.selected_tool_id(), Some("call-1"));
        scrollback.apply(ScrollbackMsg::Clear);
        assert_eq!(scrollback.selected_tool_id(), None);
    }

    #[test]
    fn selected_tool_header_uses_grok_fold_indicator() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "selected".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render(Rect::new(0, 0, 40, 3), &mut buffer);
        let selected_cell = (0..3)
            .flat_map(|row| (0..40).map(move |column| (column, row)))
            .find_map(|(column, row)| {
                buffer
                    .cell((column, row))
                    .filter(|cell| cell.symbol() == "⌄")
                    .map(|cell| cell.bg)
            })
            .expect("selected fold indicator");
        assert_eq!(selected_cell, ratatui::style::Color::Rgb(28, 28, 28));
        assert_eq!(
            buffer
                .cell((39, selected_cell_row(&buffer)))
                .expect("row")
                .bg,
            ratatui::style::Color::Rgb(28, 28, 28)
        );
    }

    #[test]
    fn collapsed_selected_tool_header_uses_grok_right_chevron() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "collapsed".into(),
            header: "Read README.md".into(),
            activity: None,
        });
        scrollback.apply(ScrollbackMsg::SetToolMode(
            "collapsed".into(),
            runie_core::types::ToolDisplayMode::Collapsed,
        ));
        scrollback.apply(ScrollbackMsg::SelectNextTool);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 3));
        scrollback.render(Rect::new(0, 0, 40, 3), &mut buffer);
        assert!((0..3).any(|row| {
            (0..40).any(|column| {
                buffer
                    .cell((column, row))
                    .is_some_and(|cell| cell.symbol() == "›")
            })
        }));
    }

    fn selected_cell_row(buffer: &Buffer) -> u16 {
        (0..3)
            .find(|row| {
                (0..40).any(|column| {
                    buffer
                        .cell((column, *row))
                        .is_some_and(|cell| cell.symbol() == "⌄")
                })
            })
            .expect("selected row")
    }

    #[test]
    fn running_tool_bullet_advances_as_actor_owned_animation_state() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.apply(ScrollbackMsg::ToolStart {
            tool_call_id: "worker".into(),
            header: "Subagent running: \"inspect\"".into(),
            activity: None,
        });
        let mut first = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut first);
        assert_eq!(first.cell((0, 0)).expect("running bullet").symbol(), "⋅");
        scrollback.apply(ScrollbackMsg::AdvanceAnimation);
        let mut second = Buffer::empty(Rect::new(0, 0, 40, 1));
        scrollback.render(Rect::new(0, 0, 40, 1), &mut second);
        assert_eq!(
            second.cell((0, 0)).expect("next running bullet").symbol(),
            ":"
        );
        assert!(scrollback.animation_demand());
        scrollback.apply(ScrollbackMsg::ToolEnd {
            tool_call_id: "worker".into(),
            header: "Subagent completed: \"inspect\"".into(),
            activity: None,
            output: Vec::new(),
        });
        assert!(!scrollback.animation_demand());
    }

    #[test]
    fn expanded_tool_member_remains_visible_inside_collapsed_activity() {
        let mut scrollback = Scrollback::new();
        scrollback.append(Line::new(LineKind::Activity, "❙  ◈ Read 1 file"));
        scrollback.append(Line::new(LineKind::Tool, "Read README.md").for_tool("read-1"));
        scrollback.set_tool_mode("read-1", runie_core::types::ToolDisplayMode::Expanded);

        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 2));
        scrollback.render(Rect::new(0, 0, 40, 2), &mut buffer);
        assert_eq!(buffer.cell((0, 1)).expect("tool bullet").symbol(), "◆");
        assert_eq!(buffer.cell((2, 1)).expect("tool label").symbol(), "R");
    }

    #[test]
    fn dense_tool_groups_preserve_breaks_and_member_positions() {
        let ids = [Some("a"), Some("b"), Some("c"), None, Some("d"), Some("e")];
        assert_eq!(
            dense_tool_group_members(&ids),
            vec![
                Some((0, 3)),
                Some((1, 3)),
                Some((2, 3)),
                None,
                Some((0, 2)),
                Some((1, 2)),
            ]
        );
    }

    #[test]
    fn grok_group_budget_is_named_and_source_defaulted() {
        assert_eq!(GROK_GROUP_MAX_VISIBLE, 10);
    }

    #[test]
    fn selecting_hidden_dense_member_reveals_the_whole_group() {
        let mut scrollback = Scrollback::new();
        scrollback.set_activity_expanded(true);
        scrollback.append(Line::new(LineKind::Activity, "❙  ◈ Ran 12 commands"));
        for index in 1..=12 {
            scrollback.append(
                Line::new(LineKind::Tool, format!("Run command-{index}"))
                    .for_tool(format!("call-{index}")),
            );
        }
        let mut before = Buffer::empty(Rect::new(0, 0, 40, 30));
        scrollback.render(Rect::new(0, 0, 40, 30), &mut before);
        let row_text = |buffer: &Buffer| {
            (0..30)
                .map(|row| {
                    (0..40)
                        .filter_map(|column| buffer.cell((column, row)))
                        .map(|cell| cell.symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let _ = row_text(&before);
        for _ in 0..11 {
            scrollback.apply(ScrollbackMsg::SelectNextTool);
        }
        let mut after = Buffer::empty(Rect::new(0, 0, 40, 30));
        scrollback.render(Rect::new(0, 0, 40, 30), &mut after);
        assert!(row_text(&after).contains("Run command-1"));
    }
}
