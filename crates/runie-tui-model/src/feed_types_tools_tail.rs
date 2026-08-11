/// Bounded, renderer-neutral output facts for one logical tool card.
const TOOL_CARD_PREVIEW_MAX_CHARS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCardSummary {
    pub tool_call_id: String,
    pub member_index: usize,
    pub card_kind: ToolCardKind,
    pub output_lines: usize,
    pub output_bytes: usize,
    pub output_preview: Option<String>,
    pub truncated: bool,
    pub is_running: bool,
    pub is_error: bool,
}

impl ToolCardSummary {
    /// Stable terminal projection for non-renderer consumers and replay tools.
    pub fn terminal_line(&self) -> String {
        let state = if self.is_running {
            "running"
        } else if self.is_error {
            "error"
        } else {
            "completed"
        };
        format!(
            "{} · {:?} · {state} · output={} lines/{} bytes{}{}",
            self.tool_call_id,
            self.card_kind,
            self.output_lines,
            self.output_bytes,
            if self.truncated { " truncated" } else { "" },
            self.output_preview
                .as_deref()
                .map(|preview| format!(" · preview={preview:?}"))
                .unwrap_or_default(),
        )
    }
}

pub fn tool_card_summaries(lines: &[Line], tool_names: &dyn ToolNameLookup) -> Vec<ToolCardSummary> {
    let rows = project_tool_card_rows(lines, tool_names, &HashMap::new());
    let mut summaries = Vec::new();
    let mut indices = HashMap::<(String, usize), usize>::new();
    for row in rows {
        let key = (row.tool_call_id.clone(), row.member_index);
        let index = match indices.get(&key).copied() {
            Some(index) => index,
            None => {
                summaries.push(ToolCardSummary {
                    tool_call_id: row.tool_call_id.clone(),
                    member_index: row.member_index,
                    card_kind: row.card_kind,
                    output_lines: 0,
                    output_bytes: 0,
                    output_preview: None,
                    truncated: false,
                    is_running: row.is_running,
                    is_error: row.is_error,
                });
                let index = summaries.len() - 1;
                indices.insert(key, index);
                index
            }
        };
        let summary = &mut summaries[index];
        if row.row_kind == ToolCardRowKind::Content {
            append_tool_output(summary, &row.text);
        }
        summary.is_running |= row.is_running;
        summary.is_error |= row.is_error;
    }
    summaries
}

fn append_tool_output(summary: &mut ToolCardSummary, text: &str) {
    summary.output_lines += 1;
    summary.output_bytes += text.len();
    let combined = summary
        .output_preview
        .as_deref()
        .map_or_else(|| text.to_owned(), |preview| format!("{preview}\n{text}"));
    summary.output_preview = runie_core::output::bounded_preview(&combined, TOOL_CARD_PREVIEW_MAX_CHARS);
    summary.truncated |= text.contains("[output truncated]");
}

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
    kind.is_tool_line()
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
