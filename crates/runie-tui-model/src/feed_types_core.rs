pub const GROK_GROUP_MAX_VISIBLE: usize = 10;

/// Viewport-relative terminal cell coordinate used by Grok's text selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPosition {
    pub row: u16,
    pub column: u16,
}

/// A committed transcript-cell selection. Coordinates are retained in their
/// input order; `normalized` provides the paint/copy rectangle deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellSelection {
    pub anchor: CellPosition,
    pub head: CellPosition,
}

impl CellSelection {
    pub const fn normalized(self) -> (CellPosition, CellPosition) {
        let start = if self.anchor.row < self.head.row
            || (self.anchor.row == self.head.row && self.anchor.column <= self.head.column)
        {
            self.anchor
        } else {
            self.head
        };
        let end = if start.row == self.anchor.row && start.column == self.anchor.column {
            self.head
        } else {
            self.anchor
        };
        (start, end)
    }
}

/// Project a committed terminal-cell selection into clipboard text without
/// touching a platform clipboard. Rows are clamped to the feed, columns use
/// terminal-cell widths, and line breaks are preserved for downstream
/// clipboard adapters.
pub fn selected_cell_text(lines: &[Line], selection: CellSelection) -> String {
    let (start, end) = selection.normalized();
    if lines.is_empty() || usize::from(start.row) >= lines.len() {
        return String::new();
    }
    let last_row = usize::from(end.row).min(lines.len().saturating_sub(1));
    (usize::from(start.row)..=last_row)
        .map(|row| {
            let from = if row == usize::from(start.row) {
                usize::from(start.column)
            } else {
                0
            };
            let to = if row == last_row {
                usize::from(end.column)
            } else {
                usize::MAX
            };
            let mut column = 0usize;
            lines[row]
                .text
                .chars()
                .filter(|character| {
                    let width = unicode_width::UnicodeWidthChar::width(*character).unwrap_or(0);
                    let next_column = column.saturating_add(width);
                    let selected = width > 0 && next_column > from && column < to;
                    column = next_column;
                    selected
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Project the latest assistant output for the platform clipboard boundary.
pub fn last_assistant_text(lines: &[Line]) -> String {
    let mut output = Vec::new();
    for line in lines.iter().rev() {
        if line.kind == LineKind::Assistant {
            if !line.text.is_empty() {
                output.push(line.text.as_str());
            }
        } else if !output.is_empty() {
            break;
        }
    }
    output.reverse();
    output.join("\n")
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
    /// Render the prefix glyphs for a transcript line. Centralized
    /// here so the actor-owned transcript projection and the
    /// renderer share one vocabulary.
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
    /// Canonical non-navigation facts projected from reducer-owned state.
    pub facts: FeedFacts,
    pub tool_blocks: Vec<ToolBlock>,
    pub autoscroll: bool,
    pub scroll_offset: usize,
    pub reasoning_expanded: bool,
    pub activity_expanded: bool,
    pub prompt_timestamp: Option<String>,
    pub revealed_dense_groups: HashSet<String>,
    pub center_revealed_entry: bool,
    pub follow_latest_user: bool,
    pub selected_tool_id: Option<String>,
    pub selected_tool_row_id: Option<u64>,
    pub selected_entry: Option<usize>,
    pub selected_member_index: Option<usize>,
    pub selection_anchor: Option<usize>,
    pub selection_head: Option<usize>,
    pub cell_selection: Option<CellSelection>,
    pub copy_selection: Option<CellSelection>,
    pub theme: ThemeKind,
    pub animation_frame: usize,
    /// Last renderer measurement delivered through `LayoutMeasured`.
    pub measured_content_rows: usize,
    pub measured_viewport_rows: usize,
    pub measured_anchor_row: Option<usize>,
}

/// Normalized feed facts independent of viewport and selection controls.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeedFacts {
    pub tools: HashMap<String, ToolRecord>,
    pub activity_dirs: usize,
    pub activity_files: usize,
    pub activity_commands: usize,
    pub activity_subagents: usize,
    pub activity_failures: usize,
    pub settled_no_tool_phase: bool,
    pub live_grok_layout: bool,
    pub next_tool_row_id: u64,
    pub workflow_headers: HashMap<String, String>,
    pub workflow_phases: HashMap<String, Vec<(String, String)>>,
    pub turn_started: bool,
    pub assistant_stream_open: bool,
}

impl ToolRecord {
    pub fn named(name: String) -> Self { Self { name: Some(name), args: None } }
    pub fn set_args(&mut self, args: serde_json::Value) { self.args = Some(args); }
    pub fn clear_args(&mut self) { self.args = None; }
}

impl FeedFacts {
    pub fn tool_name(&self, id: &str) -> Option<&str> {
        self.tools.get(id).and_then(|record| record.name.as_deref())
    }

    pub fn tool_args(&self, id: &str) -> Option<&serde_json::Value> {
        self.tools.get(id).and_then(|record| record.args.as_ref())
    }

    pub fn reset_activity(&mut self) {
        self.activity_dirs = 0;
        self.activity_files = 0;
        self.activity_commands = 0;
        self.activity_subagents = 0;
        self.activity_failures = 0;
    }

    pub fn reset_workflows(&mut self) {
        self.workflow_headers.clear();
        self.workflow_phases.clear();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

}

/// Renderer-independent navigation and animation facts for a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedNavigation {
    pub facts: FeedFacts,
    pub autoscroll: bool,
    pub scroll_offset: usize,
    pub follow_latest_user: bool,
    pub selected_tool_id: Option<String>,
    pub selected_tool_row_id: Option<u64>,
    pub selected_entry: Option<usize>,
    pub selection_anchor: Option<usize>,
    pub selection_head: Option<usize>,
    pub cell_selection: Option<CellSelection>,
    pub copy_selection: Option<CellSelection>,
    pub cell_selection_anchor: Option<CellPosition>,
    pub animation_frame: usize,
    pub reasoning_expanded: bool,
    pub activity_expanded: bool,
    pub tool_modes: HashMap<String, ToolDisplayMode>,
    pub theme: ThemeKind,
    pub prompt_timestamp: Option<String>,
    pub revealed_dense_groups: HashSet<String>,
    pub center_revealed_entry: bool,
    pub measured_content_rows: usize,
    pub measured_viewport_rows: usize,
    pub measured_anchor_row: Option<usize>,
}

impl From<&FeedNavigation> for FeedFacts {
    fn from(navigation: &FeedNavigation) -> Self {
        navigation.facts.clone()
    }
}

impl Default for FeedNavigation {
    fn default() -> Self {
        Self {
            autoscroll: true,
            scroll_offset: 0,
            follow_latest_user: false,
            selected_tool_id: None,
            selected_tool_row_id: None,
            selected_entry: None,
            selection_anchor: None,
            selection_head: None,
            cell_selection: None,
            copy_selection: None,
            cell_selection_anchor: None,
            animation_frame: 0,
            reasoning_expanded: false,
            activity_expanded: false,
            tool_modes: HashMap::new(),
            theme: ThemeKind::GrokNight,
            prompt_timestamp: None,
            revealed_dense_groups: HashSet::new(),
            center_revealed_entry: false,
            facts: FeedFacts::default(),
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
