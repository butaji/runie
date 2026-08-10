#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ToolRecord {
    pub name: Option<String>,
    pub args: Option<serde_json::Value>,
}

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
    Metadata,
    Content,
    Status,
}

/// Renderer-neutral semantic paint role for a typed Grok card row.
///
/// This is deliberately not a terminal colour or Ratatui style: theme and
/// capability resolution belongs to the renderer boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
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
    /// Actor-issued identity of the live tool row, when available.
    pub tool_row_id: Option<u64>,
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
    lines
        .iter()
        .position(|line| line.tool_call_id.as_deref() == Some(tool_call_id))
        .and_then(|line_index| logical_tool_member_index_at(lines, line_index))
}

/// Return the logical member ordinal for the exact transcript line. Live rows
/// use the actor-issued row identity, while compatibility-seeded rows retain
/// the historical call-ID grouping.
pub fn logical_tool_member_index_at(lines: &[Line], line_index: usize) -> Option<usize> {
    let target = lines.get(line_index)?;
    let mut indices = HashMap::new();
    let mut next = 0usize;
    for (index, line) in lines.iter().enumerate() {
        if line.tool_call_id.is_none() {
            continue;
        }
        let key = tool_member_key(lines, index);
        let index = if let Some(index) = indices.get(&key) {
            *index
        } else {
            let index = next;
            next += 1;
            indices.insert(key, index);
            index
        };
        if std::ptr::eq(line, target) {
            return Some(index);
        }
    }
    None
}

fn tool_member_key(lines: &[Line], line_index: usize) -> (String, Option<u64>) {
    let line = &lines[line_index];
    let Some(id) = line.tool_call_id.as_deref() else {
        return (String::new(), None);
    };
    let header = lines[..=line_index]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, candidate)| {
            candidate.tool_call_id.as_deref() == Some(id)
                && matches!(candidate.kind, LineKind::Tool | LineKind::ToolRunning)
        });
    (
        id.to_owned(),
        header.and_then(|(_, candidate)| candidate.tool_row_id),
    )
}

impl ToolCardRow {
    pub fn paint_intent(&self) -> ToolCardPaintIntent {
        match self.row_kind {
            ToolCardRowKind::Header if self.is_running => ToolCardPaintIntent::Running,
            ToolCardRowKind::Header => ToolCardPaintIntent::Header,
            ToolCardRowKind::Metadata => ToolCardPaintIntent::Muted,
            ToolCardRowKind::Content if self.card_kind == ToolCardKind::MemorySearch => {
                ToolCardPaintIntent::Muted
            }
            ToolCardRowKind::Content => ToolCardPaintIntent::Content,
            ToolCardRowKind::Status if self.is_error => ToolCardPaintIntent::Error,
            ToolCardRowKind::Status => ToolCardPaintIntent::Success,
        }
    }
}

fn is_memory_metadata(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.as_bytes().first().is_some_and(u8::is_ascii_digit) && trimmed.contains(". ")
}

fn is_web_fetch_metadata(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("status:")
        || trimmed.starts_with("content_type:")
        || trimmed.starts_with("title:")
}

/// Project transcript rows into semantic card rows in transcript order.
pub fn project_tool_card_rows(
    lines: &[Line],
    tool_names: &dyn ToolNameLookup,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> Vec<ToolCardRow> {
    let mut rows = Vec::new();
    let mut member_indices: HashMap<(String, Option<u64>), usize> = HashMap::new();
    let mut next_member_index = 0usize;
    for (line_index, line) in lines.iter().enumerate() {
        if let Some(row) = project_tool_card_row(
            line,
            line_index,
            lines,
            tool_names,
            tool_modes,
            &mut member_indices,
            &mut next_member_index,
        ) {
            rows.push(row);
        }
    }
    rows
}

fn project_tool_card_row(
    line: &Line,
    line_index: usize,
    lines: &[Line],
    tool_names: &dyn ToolNameLookup,
    tool_modes: &HashMap<String, ToolDisplayMode>,
    member_indices: &mut HashMap<(String, Option<u64>), usize>,
    next_member_index: &mut usize,
) -> Option<ToolCardRow> {
    let tool_call_id = line.tool_call_id.as_deref()?;
    let header = tool_names.tool_name(tool_call_id).unwrap_or(&line.text);
    let card_kind = ToolCardKind::from_header(header);
    let row_kind = card_row_kind(line, card_kind)?;
    let member_key = tool_member_key(lines, line_index);
    let member_index = *member_indices.entry(member_key).or_insert_with(|| {
        let index = *next_member_index;
        *next_member_index += 1;
        index
    });
    Some(ToolCardRow {
        tool_call_id: tool_call_id.to_owned(),
        tool_row_id: line.tool_row_id,
        member_index,
        card_kind,
        row_kind,
        text: line.text.clone(),
        mode: tool_mode_for_line(line, tool_modes),
        is_running: line.kind == LineKind::ToolRunning,
        is_error: line.kind == LineKind::ToolError,
    })
}

fn card_row_kind(line: &Line, card_kind: ToolCardKind) -> Option<ToolCardRowKind> {
    match line.kind {
        LineKind::Tool | LineKind::ToolRunning if !line.text.trim_end().ends_with('✗') => {
            Some(ToolCardRowKind::Header)
        }
        LineKind::ToolError | LineKind::Tool => Some(ToolCardRowKind::Status),
        LineKind::ToolOutput | LineKind::ToolResult if card_metadata(line, card_kind) => {
            Some(ToolCardRowKind::Metadata)
        }
        LineKind::ToolOutput | LineKind::ToolResult => Some(ToolCardRowKind::Content),
        _ => None,
    }
}

fn card_metadata(line: &Line, card_kind: ToolCardKind) -> bool {
    (card_kind == ToolCardKind::MemorySearch && is_memory_metadata(&line.text))
        || (card_kind == ToolCardKind::WebSearch && line.text.trim_start().starts_with("Sources:"))
        || (card_kind == ToolCardKind::WebFetch && is_web_fetch_metadata(&line.text))
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
pub fn project_tool_blocks(
    lines: &[Line],
    tool_names: &dyn ToolNameLookup,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> Vec<ToolBlock> {
    let mut blocks = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        project_tool_block_line(&mut blocks, lines, line_index, line, tool_names, tool_modes);
    }
    blocks
}

fn project_tool_block_line(
    blocks: &mut Vec<ToolBlock>,
    lines: &[Line],
    line_index: usize,
    line: &Line,
    tool_names: &dyn ToolNameLookup,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) {
    let Some(id) = line.tool_call_id.as_deref() else {
        return;
    };
    let is_header = is_tool_header_line(line);
    let index = find_tool_block(blocks, lines, line_index, line, id, is_header);
    let Some(index) = index else {
        if is_header {
            blocks.push(new_tool_block(line, id, tool_names, tool_modes));
        }
        return;
    };
    update_tool_block(&mut blocks[index], line, tool_names, tool_modes);
}

fn is_tool_header_line(line: &Line) -> bool {
    matches!(
        line.kind,
        LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError
    )
}

fn find_tool_block(
    blocks: &[ToolBlock],
    lines: &[Line],
    line_index: usize,
    line: &Line,
    id: &str,
    is_header: bool,
) -> Option<usize> {
    if is_header {
        return line.tool_row_id.map_or_else(
            || blocks.iter().position(|block| block.tool_call_id == id),
            |row_id| {
                blocks
                    .iter()
                    .position(|block| block.tool_row_id == Some(row_id))
            },
        );
    }
    let row_id = tool_member_key(lines, line_index).1;
    blocks.iter().rposition(|block| {
        block.tool_call_id == id && (row_id.is_none() || block.tool_row_id == row_id)
    })
}

fn new_tool_block(
    line: &Line,
    id: &str,
    tool_names: &dyn ToolNameLookup,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) -> ToolBlock {
    let header = tool_names.tool_name(id).unwrap_or(&line.text);
    ToolBlock {
        tool_call_id: id.to_owned(),
        header: line.text.clone(),
        kind: ToolCardKind::from_header(header),
        output: Vec::new(),
        mode: tool_mode_for_line(line, tool_modes),
        is_running: line.kind == LineKind::ToolRunning,
        is_error: line.kind == LineKind::ToolError,
        tool_row_id: line.tool_row_id,
    }
}

fn update_tool_block(
    block: &mut ToolBlock,
    line: &Line,
    tool_names: &dyn ToolNameLookup,
    tool_modes: &HashMap<String, ToolDisplayMode>,
) {
    match line.kind {
        LineKind::Tool | LineKind::ToolRunning | LineKind::ToolError => {
            block.header = line.text.clone();
            let name = tool_names
                .tool_name(&block.tool_call_id)
                .unwrap_or(&line.text);
            block.kind = ToolCardKind::from_header(name);
            block.mode = tool_mode_for_line(line, tool_modes);
            block.is_running = line.kind == LineKind::ToolRunning;
            block.is_error = line.kind == LineKind::ToolError;
        }
        LineKind::ToolOutput | LineKind::ToolResult => block.output.push(line.text.clone()),
        _ => {}
    }
}

impl ToolCardKind {
    pub fn from_header(header: &str) -> Self {
        let lower = header.trim_start().to_ascii_lowercase();
        exact_tool_kind(&lower)
            .or_else(|| prefixed_tool_kind(&lower))
            .unwrap_or(Self::Generic)
    }
}


fn exact_tool_kind(name: &str) -> Option<ToolCardKind> {
    Some(match name {
        "bash"
        | "shell"
        | "exec"
        | "run"
        | "execute"
        | "run_terminal_command"
        | "run_terminal_cmd" => ToolCardKind::Execute,
        "read" | "read_file" => ToolCardKind::Read,
        "edit" | "write" | "write_file" | "search_replace" | "apply_patch" | "strreplace" => {
            ToolCardKind::Edit
        }
        "list_dir" | "list_files" | "ls" => ToolCardKind::ListDir,
        "web_search" | "web-search" => ToolCardKind::WebSearch,
        "search" | "grep" | "find" | "glob" => ToolCardKind::Search,
        "web_fetch" | "web-fetch" | "fetch" => ToolCardKind::WebFetch,
        "memory_search" | "memory-search" => ToolCardKind::MemorySearch,
        "workflow" | "run_workflow" | "run-workflow" => ToolCardKind::Workflow,
        "todo" | "todo_write" | "todo-write" => ToolCardKind::Todo,
        "use" | "use_tool" | "use-tool" => ToolCardKind::Use,
        "search_tools" | "search-tools" | "search_tool" => ToolCardKind::SearchTools,
        "subagent" | "agent" | "task" => ToolCardKind::Background,
        _ => return None,
    })
}

fn prefixed_tool_kind(name: &str) -> Option<ToolCardKind> {
    [
        (ToolCardKind::Execute, &["run ", "execute "][..]),
        (ToolCardKind::Read, &["read "][..]),
        (
            ToolCardKind::Edit,
            &["edit ", "write ", "apply_patch ", "strreplace "][..],
        ),
        (ToolCardKind::ListDir, &["list ", "ls "][..]),
        (ToolCardKind::WebSearch, &["web search "][..]),
        (ToolCardKind::Search, &["search "][..]),
        (ToolCardKind::WebFetch, &["fetch "][..]),
        (ToolCardKind::MemorySearch, &["memory search "][..]),
        (ToolCardKind::Workflow, &["workflow "][..]),
        (ToolCardKind::Todo, &["todo "][..]),
        (ToolCardKind::Use, &["use "][..]),
        (ToolCardKind::SearchTools, &["search tools "][..]),
        (ToolCardKind::Background, &["subagent "][..]),
    ]
    .into_iter()
    .find_map(|(kind, prefixes)| {
        prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix))
            .then_some(kind)
    })
}

/// Pure formatter for the Grok "Workflow name: objective" transcript row.
/// Renderers and replay fixtures share this projection so the live and
/// legacy reducers cannot drift on phase glyphs, duration punctuation,
/// or the trailing agent-count badge.
pub fn workflow_text(
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
    let verb = workflow_verb(name, status, &duration);
    let objective = objective.split_whitespace().collect::<Vec<_>>().join(" ");
    let trail = workflow_trail(phases);
    let mut result = format!("Workflow {verb}{objective}");
    if !trail.is_empty() {
        result.push_str(&format!("  [{trail}]"));
    }
    if status == "active" && active_agents > 0 {
        result.push_str(&format!("  ({active_agents} agents)"));
    }
    result
}

fn workflow_verb(name: &str, status: &str, duration: &str) -> String {
    let elapsed = if duration.is_empty() {
        String::new()
    } else {
        format!(" in {duration}")
    };
    match status {
        "active" => format!("{name}: "),
        "cancelled" => format!("{name} ◌ cancelled after {duration}: "),
        "paused" => format!("{name} paused at {duration}: "),
        "failed" | "interrupted" => format!("{name} failed{elapsed}: "),
        _ => format!("{name} done{elapsed}: "),
    }
}
