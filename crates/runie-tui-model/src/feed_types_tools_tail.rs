fn workflow_trail(phases: &[(String, String)]) -> String {
    phases
        .iter()
        .map(|(title, state)| format!("{title} {}", phase_mark(state)))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn phase_mark(state: &str) -> char {
    match state {
        "active" | "running" => '●',
        "done" | "completed" => '✓',
        "failed" | "error" | "interrupted" => '✗',
        _ => '○',
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

    /// Detect whether a `Line` is blank — i.e. has no rendered text.
    /// Centralized here so the actor-owned transcript projection and
    /// the renderer agree on the blank-line definition.
    pub fn is_blank(&self) -> bool {
        self.text.is_empty()
    }

    pub fn settle_tool_row(&mut self) {
        self.tool_row_active = false;
    }
}

/// Free-function predicate for whether a `Line` is blank. Centralized
/// here so the actor-owned transcript projection and the renderer
/// share the blank-line vocabulary.
pub fn line_is_blank(line: &Line) -> bool {
    line.is_blank()
}

fn is_tool_line(kind: LineKind) -> bool {
    matches!(
        kind,
        LineKind::Tool
            | LineKind::ToolRunning
            | LineKind::ToolError
            | LineKind::ToolOutput
            | LineKind::ToolResult
    )
}

/// Find the first index in `lines` whose `text` contains the
/// `needle`. Centralized here so the actor-owned transcript projection
/// and the renderer share the search predicate.
pub fn find_first_containing(lines: &[Line], needle: &str) -> Option<usize> {
    lines.iter().position(|l| l.text.contains(needle))
}

/// Find all line indices in `lines` whose `text` contains the
/// `needle`. Centralized here so the actor-owned transcript projection
/// and the renderer share the search predicate.
pub fn find_all_containing(lines: &[Line], needle: &str) -> Vec<usize> {
    lines
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

/// Return the physical-row correction needed to keep a manually selected
/// logical row anchored while responsive wrapping changes its position.
fn measured_anchor_delta(previous: Option<usize>, current: Option<usize>) -> isize {
    match (previous, current) {
        (Some(previous), Some(current)) => current as isize - previous as isize,
        _ => 0,
    }
}
