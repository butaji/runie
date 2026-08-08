//! Renderer-independent transcript line vocabulary and reducer intents.

use std::collections::{HashMap, HashSet};

use runie_core::types::{ThemeKind, ToolDisplayMode};

/// Semantic category for grouped activity-tool counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Dir,
    File,
    Command,
    Subagent,
}

/// Classify a tool name for grouped activity presentation.
pub fn classify_activity_tool(tool_name: &str) -> Option<ActivityKind> {
    match tool_name {
        "list_dir" | "list_files" | "ls" => Some(ActivityKind::Dir),
        "read" | "read_file" => Some(ActivityKind::File),
        "bash"
        | "shell"
        | "exec"
        | "run"
        | "execute"
        | "run_terminal_command"
        | "run_terminal_cmd" => Some(ActivityKind::Command),
        "subagent" | "agent" | "task" => Some(ActivityKind::Subagent),
        _ => None,
    }
}

/// Return `true` when a tool's result should be projected as the
/// structured `LineKind::ToolOutput` rather than the textual
/// `LineKind::ToolResult`. Output-style tools render their content
/// directly; result-style tools keep their headers and prose.
pub fn is_output_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "list_dir"
            | "list_files"
            | "read"
            | "read_file"
            | "web_fetch"
            | "web-fetch"
            | "fetch"
            | "memory_search"
            | "memory-search"
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "the semantic tool-header DSL keeps Grok aliases together"
)]
pub fn tool_header(tool_name: &str, args: &serde_json::Value, workspace: &str) -> String {
    let path = |value: &str| -> String {
        value
            .strip_prefix(workspace)
            .and_then(|rest| rest.strip_prefix('/'))
            .filter(|rest| !rest.is_empty())
            .unwrap_or(value)
            .to_owned()
    };
    let string = |keys: &[&str], fallback: &str| -> String {
        keys.iter()
            .find_map(|key| args.get(*key).and_then(serde_json::Value::as_str))
            .unwrap_or(fallback)
            .to_owned()
    };
    match tool_name {
        "list_dir" | "list_files" | "ls" => {
            format!("List {}", path(&string(&["path"], ".")))
        }
        "read" | "read_file" => format!("Read {}", path(&string(&["path"], ""))),
        "edit" | "write" | "write_file" | "search_replace" | "apply_patch" | "strreplace" => {
            format!("Edit {}", path(&string(&["path", "file_path"], "")))
        }
        "search" | "grep" | "find" | "glob" => {
            let pattern = string(&["pattern", "query"], "");
            match args
                .get("path")
                .or_else(|| args.get("cwd"))
                .and_then(serde_json::Value::as_str)
            {
                Some(value) if !value.is_empty() => {
                    format!("Search {pattern:?} in {}", path(value))
                }
                _ => format!("Search {pattern:?}"),
            }
        }
        "web_search" | "web-search" => format!("Web Search {}", string(&["query", "q"], "")),
        "web_fetch" | "web-fetch" | "fetch" => {
            format!("Fetch {}", string(&["url"], ""))
        }
        "bash"
        | "shell"
        | "exec"
        | "run"
        | "execute"
        | "run_terminal_command"
        | "run_terminal_cmd" => format!("Run {}", string(&["command", "cmd"], "")),
        "subagent" | "agent" | "task" => {
            format!(
                "Subagent started: {:?}",
                string(&["description", "task", "prompt"], "")
            )
        }
        "workflow" | "run_workflow" | "run-workflow" => {
            format!("Workflow {}", string(&["name", "workflow"], ""))
        }
        "use" | "use_tool" | "use-tool" => {
            format!("Use {}", string(&["tool", "name"], ""))
        }
        "memory_search" | "memory-search" => {
            format!("Memory Search {}", string(&["query", "q"], ""))
        }
        "search_tools" | "search-tools" | "search_tool" => {
            format!("Search Tools {}", string(&["query", "pattern"], ""))
        }
        "todo" | "todo_write" | "todo-write" => {
            format!("Todo {}", string(&["title", "task"], "Update todos"))
        }
        _ => format!(
            "{tool_name} {}",
            serde_json::to_string(args).unwrap_or_default()
        ),
    }
}

/// Project a streaming tool-update envelope to the user-visible partial text
/// when the provider ships a single string payload. Returns `None` for
/// envelopes that only carry lifecycle metadata, so callers can keep their
/// own transport-only path open.
pub fn structured_update_text(result: &serde_json::Value) -> Option<String> {
    result
        .get("output")
        .and_then(serde_json::Value::as_str)
        .or_else(|| result.get("content").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}

/// Return `true` when a streaming tool-update envelope only carries
/// lifecycle metadata (for example `{status: "running"}`) and no
/// user-visible payload. Callers use this to short-circuit specialized
/// card projections so transport-only events stay block state rather than
/// transcript text.
pub fn is_transport_only_update(partial_result: &serde_json::Value) -> bool {
    partial_result.get("status").is_some() && structured_update_text(partial_result).is_none()
}

/// Extract the user-visible text from a Pi tool result without exposing its
/// transport envelope to the feed actor.
pub fn tool_result_text(result: &serde_json::Value) -> String {
    result
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            result
                .get("content")
                .and_then(serde_json::Value::as_array)
                .and_then(|content| content.iter().find_map(|item| item.get("text")))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            result
                .get("output")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            result
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            result
                .get("content")
                .filter(|content| content.as_array().is_some_and(Vec::is_empty))
                .map(|_| String::new())
        })
        .unwrap_or_else(|| serde_json::to_string(result).unwrap_or_default())
}

/// Count the unique hostnames surfaced by a web-search result payload.
///
/// The projection is URL-first: each `https://`/`http://` token contributes its
/// hostname, lowercased, with URL punctuation (`/`, `?`, `#`, `)`, `]`, `,`)
/// terminating the host. When the payload contains no URLs the count falls back
/// to the number of non-empty lines, preserving the renderer contract for
/// URL-free web-search outputs.
pub fn web_search_site_count(output: &str) -> usize {
    let mut domains = std::collections::HashSet::new();
    for token in output.split_whitespace() {
        let Some(url) = token
            .strip_prefix("https://")
            .or_else(|| token.strip_prefix("http://"))
        else {
            continue;
        };
        if let Some(domain) = url.split(['/', '?', '#', ')', ']', ',']).next() {
            if !domain.is_empty() {
                domains.insert(domain.to_ascii_lowercase());
            }
        }
    }
    if domains.is_empty() {
        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    } else {
        domains.len()
    }
}

/// Render the canonical `  Sources: …` summary line for a successful web-search
/// completion. Returns `None` when no URL hostname can be extracted, so the
/// caller can skip the row entirely. The first three unique hostnames are
/// listed in first-seen order; any remainder is summarized as `(+N more)`.
pub fn web_search_sources_line(output: &str) -> Option<String> {
    let mut domains = Vec::new();
    for token in output.split_whitespace() {
        let Some(url) = token
            .strip_prefix("https://")
            .or_else(|| token.strip_prefix("http://"))
        else {
            continue;
        };
        let Some(domain) = url
            .split(['/', '?', '#', ')', ']', ','])
            .next()
            .filter(|domain| !domain.is_empty())
        else {
            continue;
        };
        if !domains.iter().any(|seen| seen == domain) {
            domains.push(domain.to_owned());
        }
    }
    if domains.is_empty() {
        return None;
    }
    let shown = domains
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let remaining = domains.len().saturating_sub(3);
    Some(if remaining == 0 {
        format!("  Sources: {shown}")
    } else {
        format!("  Sources: {shown} (+{remaining} more)")
    })
}

/// Project Grok's grouped tool activity label without terminal concerns.
#[allow(
    clippy::cognitive_complexity,
    reason = "the activity vocabulary is one declarative projection"
)]
pub fn activity_text(
    dirs: usize,
    files: usize,
    commands: usize,
    subagents: usize,
    failures: usize,
    running: bool,
) -> String {
    let dir_verb = if running { "Listing" } else { "Listed" };
    let file_verb = if running { "Reading" } else { "Read" };
    let command_verb = if running { "Running" } else { "Ran" };
    let subagent_verb = if running { "Running" } else { "Ran" };
    let mut parts = Vec::new();
    if dirs > 0 {
        parts.push(format!(
            "{dir_verb} {dirs} dir{}",
            if dirs == 1 { "" } else { "s" }
        ));
    }
    if files > 0 {
        parts.push(format!(
            "{file_verb} {files} file{}",
            if files == 1 { "" } else { "s" }
        ));
    }
    if commands > 0 {
        parts.push(format!(
            "{command_verb} {commands} command{}",
            if commands == 1 { "" } else { "s" }
        ));
    }
    if subagents > 0 {
        parts.push(format!(
            "{subagent_verb} {subagents} subagent{}",
            if subagents == 1 { "" } else { "s" }
        ));
    }
    let mut text = format!("◈ {}", parts.join(", "));
    if failures > 0 && !running {
        text.push_str(&format!(" · {failures} failed"));
    }
    text
}

/// Add Grok's result cardinality/range suffix to a retained tool header.
#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "tool-card completion variants are one semantic DSL"
)]
pub fn completed_tool_header_with_args(
    pending_header: &str,
    tool_name: &str,
    args: &serde_json::Value,
    result: &serde_json::Value,
) -> String {
    let output = tool_result_text(result);
    if matches!(tool_name, "read" | "read_file") {
        if result
            .get("content")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| {
                items.iter().any(|item| {
                    item.get("type") == Some(&serde_json::Value::String("image".into()))
                })
            })
        {
            return format!("{pending_header} (image)");
        }
        if let Some(offset) = args.get("offset").and_then(serde_json::Value::as_u64) {
            let lines = output
                .lines()
                .take_while(|line| !line.starts_with('['))
                .count() as u64;
            let end = offset.saturating_add(lines.max(1));
            let total = result
                .get("details")
                .and_then(|v| v.get("truncation"))
                .and_then(|v| v.get("totalLines"))
                .and_then(serde_json::Value::as_u64)
                .or_else(|| {
                    output.lines().find_map(|line| {
                        line.split(" of ")
                            .nth(1)
                            .and_then(|part| part.split(|c: char| !c.is_ascii_digit()).next())
                            .and_then(|value| value.parse().ok())
                    })
                });
            return match total {
                Some(total) => format!("{pending_header} ({}-{} of {total})", offset + 1, end),
                None => format!("{pending_header} ({}-{end})", offset + 1),
            };
        }
    }
    let count = |nonempty: bool| {
        output
            .lines()
            .filter(|line| !nonempty || !line.trim().is_empty())
            .count()
    };
    match tool_name {
        "list_dir" | "list_files" | "ls" => {
            let n = count(true);
            format!(
                "{pending_header} ({n} entr{})",
                if n == 1 { "y" } else { "ies" }
            )
        }
        "read" | "read_file" => format!("{pending_header} ({} lines)", output.lines().count()),
        "search" | "grep" | "find" | "glob" => {
            let n = count(true);
            format!(
                "{pending_header} ({n} match{})",
                if n == 1 { "" } else { "es" }
            )
        }
        "web_search" | "web-search" => {
            let n = web_search_site_count(&output);
            format!(
                "{pending_header} ({n} site{})",
                if n == 1 { "" } else { "s" }
            )
        }
        "search_tools" | "search-tools" | "search_tool" => {
            let n = count(true);
            format!(
                "{pending_header} ({n} result{})",
                if n == 1 { "" } else { "s" }
            )
        }
        "memory_search" | "memory-search" => {
            let n = crate::memory::parse_memory_results(&output).len();
            format!(
                "{pending_header} ({n} result{})",
                if n == 1 { "" } else { "s" }
            )
        }
        "todo" | "todo_write" | "todo-write" => {
            let n = count(true);
            if n == 0 {
                pending_header.to_owned()
            } else {
                format!(
                    "{pending_header} ({n} item{})",
                    if n == 1 { "" } else { "s" }
                )
            }
        }
        "workflow" | "run_workflow" | "run-workflow" => pending_header
            .strip_prefix("Workflow ")
            .map(|n| format!("Workflow completed: {n}"))
            .unwrap_or_else(|| pending_header.to_owned()),
        "use" | "use_tool" | "use-tool" => pending_header
            .strip_prefix("Use ")
            .map(|n| format!("Used {n}"))
            .unwrap_or_else(|| pending_header.to_owned()),
        "subagent" | "agent" | "task" => pending_header
            .strip_prefix("Subagent started: ")
            .map(|n| format!("Subagent completed: {n}"))
            .unwrap_or_else(|| pending_header.to_owned()),
        "edit" | "write" | "write_file" | "search_replace" => {
            let n = count(true);
            if n == 0 {
                pending_header.to_owned()
            } else {
                format!(
                    "{pending_header} ({n} edit{})",
                    if n == 1 { "" } else { "s" }
                )
            }
        }
        _ => format!("{pending_header} → ✓"),
    }
}

/// Render the elapsed-time suffix for completed background work. `None`
/// resolves to an empty fragment so the host string stays identical to the
/// pre-elapsed form.
pub fn format_elapsed(elapsed_ms: Option<u64>) -> String {
    elapsed_ms
        .map(|millis| format!(" in {:.1}s", millis as f64 / 1_000.0))
        .unwrap_or_default()
}

/// Render the trailing error fragment for background-work completions. The
/// suffix is suppressed when the work did not error, so success messages
/// stay identical regardless of whether an error payload is present.
pub fn format_error(is_error: bool, error: Option<&str>) -> String {
    if is_error {
        error.map(|value| format!(" ({value})")).unwrap_or_default()
    } else {
        String::new()
    }
}

/// Fallback thinking-window duration used when the status actor never
/// observed a reasoning turn. Pinned here so the "Thought for X.Xs" line
/// stays renderer-independent and reproducible across replay paths.
pub const DEFAULT_THINKING_ELAPSED_MS: u64 = 900;

/// Render the Grok "Thought for …" summary line. `None` resolves to the
/// pinned [`DEFAULT_THINKING_ELAPSED_MS`] so callers can rely on a stable
/// label regardless of whether the status actor observed a reasoning turn.
pub fn thinking_summary(elapsed_ms: Option<u64>) -> String {
    let elapsed_ms = elapsed_ms.unwrap_or(DEFAULT_THINKING_ELAPSED_MS);
    format!("◆ Thought for {:.1}s", elapsed_ms as f64 / 1_000.0)
}

/// Animation frames for the running tool bullet. The first three characters
/// are non-breaking whitespace followed by a single trailing space so the
/// bullet occupies the same terminal width as Grok's source-backed default
/// prefix; the fourth frame is a Braille dot-cluster for the same width.
pub const RUNNING_BULLETS: [&str; 4] = ["⋅ ", ": ", "⸬ ", "⁙ "];

/// Render the running tool bullet for a given animation frame. Centralized
/// here so the actor-owned animation frame and any replay path share one
/// vocabulary; the frame index wraps via modular arithmetic.
pub fn running_bullet(frame: usize) -> &'static str {
    RUNNING_BULLETS[frame % RUNNING_BULLETS.len()]
}

/// Detect CommonMark fenced code blocks in assistant text. The recognized
/// opening fence is three backticks after the renderer prefix (`┃ `) so the
/// Grok transcript parses a code block opened in the same line that the
/// renderer already prefixed. Centralized here so the markdown classifier
/// stays renderer-independent and reproducible across replay paths.
pub fn is_fence(text: &str) -> bool {
    text.trim_start()
        .strip_prefix("┃ ")
        .unwrap_or(text)
        .starts_with("```")
}

/// Detect a Grok-flavored table row. A row starts and ends with `|` and
/// contains at least two `|` separators so the renderer can split a header
/// from a body row without ambiguity.
pub fn is_table_row(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

/// Detect the separator row beneath a Grok table header. The cells must be
/// non-empty and contain only `-`, `:`, or whitespace; this matches the
/// `<cells>` slice shown after `is_table_row` for a header line.
pub fn is_table_separator(text: &str) -> bool {
    text.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

/// Extract the heading title from a CommonMark ATX heading, returning only
/// the body text after the leading `#` run and one optional space. Levels
/// are clamped to `1..=6` to match the CommonMark specification.
pub fn atx_heading(text: &str) -> Option<&str> {
    let hashes = text.chars().take_while(|ch| *ch == '#').count();
    (1..=6)
        .contains(&hashes)
        .then(|| text.get(hashes..)?.strip_prefix(' '))
        .flatten()
}

/// Render the Grok bottom border row that closes a markdown table. The
/// column widths are derived from the cell characters plus two padding
/// cells on each side, matching the renderer's existing border shape.
pub fn table_bottom_border(text: &str) -> String {
    let widths = text
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| "─".repeat(cell.trim().chars().count() + 2))
        .collect::<Vec<_>>();
    format!("└{}┘", widths.join("┴"))
}

/// Append a wrapped line of text to the row buffer, splitting at the
/// given `width` so the renderer can project the result onto a wider
/// terminal geometry. The `code` flag lets callers mark the row as a
/// formatted code block (`true`) or normal text (`false`). Centralized
/// here so the actor-owned text projection and the renderer share one
/// wrapping rule.
pub fn append_wrapped(
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

/// Append word-wrapped text to the row buffer. Whitespace acts as the
/// break point; the leading whitespace of the source line is preserved
/// on each emitted row so the projected widget keeps its original
/// indentation.
pub fn append_wrapped_words(
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

/// Position variant for the Grok welcome surface version badge. The full
/// badge is the long `v0.1.0 · Beta` label, the hero footer appears as
/// the right-aligned footer on the wide hero, and the inline variant is
/// the compact `v0.1.0` form used in compact widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionBadgeVariant {
    Full,
    HeroFooter,
    HeroInline,
}

/// Render the welcome version badge for the given variant. Centralized
/// here so the actor-owned welcome payload and the renderer agree on the
/// exact `runie v{version} · Beta` shapes.
pub fn version_badge(variant: VersionBadgeVariant) -> String {
    let version = env!("CARGO_PKG_VERSION");
    match variant {
        VersionBadgeVariant::Full => format!("runie v{version} · Beta"),
        VersionBadgeVariant::HeroFooter => format!("runie Beta · v{version}"),
        VersionBadgeVariant::HeroInline => format!("runie v{version}"),
    }
}

/// Whether a submitted prompt text is an immediate quit command. The
/// trim/lowercase normalization matches the Grok-style `exit` / `quit`
/// / `:q` vocabulary so the keymap and any replay path share one
/// definition.
pub fn is_quit_command(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "exit" | "quit" | ":q"
    )
}

/// Render the welcome modal chrome as a sequence of `LineKind::System`
/// rows. Centralized here so the actor-owned welcome payload and the
/// renderer share the same idle chrome projection; the `env!` macro
/// resolves to the workspace version at compile time.
pub fn welcome_modal_lines() -> Vec<Line> {
    let version = env!("CARGO_PKG_VERSION");
    vec![
        Line::new(LineKind::System, format!("╭─ Runie  v{version} ─")),
        Line::new(LineKind::System, String::from("│ main runie")),
        Line::new(LineKind::System, String::from("│ Model · runie-core")),
        Line::new(LineKind::System, String::from("│ /help for commands")),
        Line::new(LineKind::System, String::from("╰─")),
        Line::new(LineKind::System, String::from("◆ session_start")),
    ]
}

/// Wrapping scrollback messages that bracket the `◆ session_start`
/// marker. Centralized here so the actor-owned session-start projection
/// and the renderer share the same `[hooks: 1]` count and the
/// surrounding separator rows.
pub fn session_start_messages() -> Vec<ScrollbackMsg> {
    vec![
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
        ScrollbackMsg::Append(Line::new(
            LineKind::SessionStart,
            "◆ session_start  [hooks: 1]",
        )),
        ScrollbackMsg::Append(Line::new(LineKind::Separator, "")),
    ]
}

/// Render a user prompt row with a right-aligned timestamp gutter. The
/// Grok transcript reserves `PROMPT_TIMESTAMP_WRAP_GUTTER` columns for
/// the timestamp before deciding where the prompt wraps, then the
/// timestamp is right-aligned to the feed's terminal edge. Centralized
/// here so the actor-owned user-prompt projection and the renderer
/// share one wrap rule.
pub fn append_user_with_timestamp(
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
    let indent = " ".repeat(USER_PREFIX_INDENT);
    let rest: String = chars.into_iter().collect();
    append_wrapped_words(
        rows,
        LineKind::User,
        format!("{indent}{}", rest.trim_start()),
        first_width,
    );
}

/// Minimum unix-timestamp value (seconds) treated as a live prompt timestamp.
/// Values below this are either absent or fixtures; values at or above are
/// rendered with the short clock format. Centralized here so the renderer
/// and any replay path share one threshold.
pub const PROMPT_TIMESTAMP_LIVE_THRESHOLD: i64 = 1_000_000_000;

/// Number of columns the Grok user-prompt prefix occupies (`   ❯ ` —
/// three spaces, the `❯` glyph, and one trailing space). Centralized
/// here so the actor-owned user-prompt wrap helper and the renderer
/// share one indent width.
pub const USER_PREFIX_INDENT: usize = 5;

/// Strip an absolute `workspace` prefix from a tool-supplied path so
/// the rendered header shows a workspace-relative path. The relative
/// path is normalized to a single leading separator and the empty
/// case collapses to `.` so the renderer never sees `<workspace>/`.
/// Centralized here so the actor-owned workspace anchor and the
/// renderer share one path-projection rule.
pub fn make_relative_path(workspace: &str, path: &str) -> String {
    let path_string = path.strip_prefix(workspace).map_or_else(
        || path.to_owned(),
        |relative| relative.strip_prefix('/').unwrap_or(relative).to_owned(),
    );
    if path_string.is_empty() || path_string == "." {
        ".".to_owned()
    } else {
        path_string
    }
}

/// Largest terminal height Grok treats as automatic compact mode (rows
/// `<= GROK_AUTO_COMPACT_MAX_ROWS`). Centralized here so the
/// model/view boundary agrees on the canonical threshold.
pub const GROK_AUTO_COMPACT_MAX_ROWS: u16 = 20;

/// Largest terminal height at which Grok still shows the small-screen
/// tip. The band `(GROK_AUTO_COMPACT_MAX_ROWS, GROK_SMALL_SCREEN_TIP_MAX_ROWS]`
/// is the pre-compact ambient window. Centralized here so the
/// visibility predicate and the renderer share one source-backed
/// threshold.
pub const GROK_SMALL_SCREEN_TIP_MAX_ROWS: u16 = 30;

/// Grok derives compact mode from full terminal height; an unmeasured
/// height must not force compact mode. Centralized here so the
/// actor-owned layout projection and the renderer share one
/// compact-mode decision.
pub const fn grok_effective_compact(user_compact: bool, terminal_rows: u16) -> bool {
    user_compact || (terminal_rows > 0 && terminal_rows <= GROK_AUTO_COMPACT_MAX_ROWS)
}

/// Grok keeps the compact-mode tip in the small-screen band immediately
/// above auto-compact. The predicate is pure so event/replay renderers
/// can make the same decision as the live terminal renderer.
pub const fn grok_small_screen_tip_visible(terminal_rows: u16) -> bool {
    terminal_rows > GROK_AUTO_COMPACT_MAX_ROWS && terminal_rows <= GROK_SMALL_SCREEN_TIP_MAX_ROWS
}

/// Render the model-selector row labels for a `ModelCatalogSnapshot`.
/// Each row is the canonical `provider/model` shape, so the actor-owned
/// selector projection and the renderer agree on the displayed text.
pub fn model_selector_rows(
    snapshot: &runie_core::model_catalog::ModelCatalogSnapshot,
) -> Vec<String> {
    snapshot
        .results
        .iter()
        .map(|model| format!("{}/{}", model.provider, model.id))
        .collect()
}

/// Render a unix-timestamp (seconds) as Grok's short clock label (e.g.
/// `3:07 PM`). Falls back to a UTC-derived 12-hour clock when libc cannot
/// resolve the local timezone, so the label is always well-formed.
pub fn format_clock_timestamp(timestamp: i64) -> String {
    let (hour24, minute) = local_clock_parts(timestamp).unwrap_or_else(|| {
        const SECONDS_PER_DAY: i64 = 86_400;
        const SECONDS_PER_HOUR: i64 = 3_600;
        const SECONDS_PER_MINUTE: i64 = 60;
        let seconds = timestamp.rem_euclid(SECONDS_PER_DAY);
        (
            seconds / SECONDS_PER_HOUR,
            (seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE,
        )
    });
    let hour12 = match hour24 % 12 {
        0 => 12,
        hour => hour,
    };
    let meridiem = if hour24 < 12 { "AM" } else { "PM" };
    format!("{hour12}:{minute:02} {meridiem}")
}

/// Resolve the local 24-hour clock parts for a unix-timestamp. Returns
/// `None` when libc cannot produce a `tm` for the input (e.g. out-of-range
/// year on the host), letting callers fall back to a UTC-derived clock.
pub(crate) fn local_clock_parts(timestamp: i64) -> Option<(i64, i64)> {
    let raw = timestamp as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: `localtime_r` writes a complete `tm` into the valid pointer or
    // returns null. No global libc timezone state is exposed to the caller.
    let result = unsafe { libc::localtime_r(&raw, local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }
    // SAFETY: a non-null result means libc initialized the structure.
    let local = unsafe { local.assume_init() };
    Some((i64::from(local.tm_hour), i64::from(local.tm_min)))
}

/// Append the streaming tool-update fragment to a retained tool header. The
/// serialized partial result is the transport payload verbatim; a payload that
/// cannot be serialized degrades to an empty fragment so the header stays
/// well-formed.
pub fn tool_update_header_text(current_header: &str, partial_result: &serde_json::Value) -> String {
    format!(
        "{current_header} | update: {}",
        serde_json::to_string(partial_result).unwrap_or_default()
    )
}

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
    pub tool_args: HashMap<String, serde_json::Value>,
    pub activity_dirs: usize,
    pub activity_files: usize,
    pub activity_commands: usize,
    pub activity_subagents: usize,
    pub activity_failures: usize,
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
    pub selection_anchor: Option<usize>,
    pub selection_head: Option<usize>,
    pub cell_selection: Option<CellSelection>,
    pub copy_selection: Option<CellSelection>,
    pub theme: ThemeKind,
    pub animation_frame: usize,
    pub tool_modes: HashMap<String, ToolDisplayMode>,
    pub turn_started: bool,
    pub assistant_stream_open: bool,
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
    pub workflow_headers: HashMap<String, String>,
    pub workflow_phases: HashMap<String, Vec<(String, String)>>,
    pub tool_names: HashMap<String, String>,
    pub tool_args: HashMap<String, serde_json::Value>,
    pub activity_dirs: usize,
    pub activity_files: usize,
    pub activity_commands: usize,
    pub activity_subagents: usize,
    pub activity_failures: usize,
    pub settled_no_tool_phase: bool,
    pub live_grok_layout: bool,
    pub next_tool_row_id: u64,
    pub turn_started: bool,
    pub assistant_stream_open: bool,
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
            workflow_headers: HashMap::new(),
            workflow_phases: HashMap::new(),
            tool_names: HashMap::new(),
            tool_args: HashMap::new(),
            activity_dirs: 0,
            activity_files: 0,
            activity_commands: 0,
            activity_subagents: 0,
            activity_failures: 0,
            settled_no_tool_phase: false,
            live_grok_layout: false,
            next_tool_row_id: 0,
            turn_started: false,
            assistant_stream_open: false,
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
        classify_activity_tool, default_tool_display_mode, is_transport_only_update,
        project_tool_blocks, project_tool_card_rows, structured_update_text, ActivityKind,
        FeedState, Line, LineKind, ToolCardKind, ToolCardPaintIntent, ToolCardRow, ToolCardRowKind,
    };
    use runie_core::types::ToolDisplayMode;
    use std::collections::HashMap;

    #[test]
    fn is_output_tool_pins_every_alias_and_excludes_others() {
        for alias in [
            "list_dir",
            "list_files",
            "read",
            "read_file",
            "web_fetch",
            "web-fetch",
            "fetch",
            "memory_search",
            "memory-search",
        ] {
            assert!(super::is_output_tool(alias), "alias: {alias}");
        }
        for name in ["bash", "subagent"] {
            assert!(!super::is_output_tool(name), "name: {name}");
        }
        assert!(!super::is_output_tool("unknown"));
    }

    #[test]
    fn activity_classifier_pins_every_alias() {
        let cases = [
            (
                ActivityKind::Dir,
                ["list_dir", "list_files", "ls"].as_slice(),
            ),
            (ActivityKind::File, ["read", "read_file"].as_slice()),
            (
                ActivityKind::Command,
                [
                    "bash",
                    "shell",
                    "exec",
                    "run",
                    "execute",
                    "run_terminal_command",
                    "run_terminal_cmd",
                ]
                .as_slice(),
            ),
            (
                ActivityKind::Subagent,
                ["subagent", "agent", "task"].as_slice(),
            ),
        ];
        for (kind, aliases) in cases {
            for alias in aliases {
                assert_eq!(classify_activity_tool(alias), Some(kind), "alias: {alias}");
            }
        }
        assert_eq!(classify_activity_tool("unknown"), None);
    }

    #[test]
    fn structured_update_prefers_output_over_content() {
        let value = serde_json::json!({
            "output": "from-output",
            "content": "from-content",
        });
        assert_eq!(
            structured_update_text(&value).as_deref(),
            Some("from-output")
        );
    }

    #[test]
    fn structured_update_falls_back_to_content_when_output_missing() {
        let value = serde_json::json!({"content": "from-content"});
        assert_eq!(
            structured_update_text(&value).as_deref(),
            Some("from-content")
        );
    }

    #[test]
    fn structured_update_returns_none_for_non_string_envelope() {
        assert!(structured_update_text(&serde_json::json!({"status": "running"})).is_none());
        assert!(structured_update_text(&serde_json::json!({"output": 7})).is_none());
        assert!(structured_update_text(&serde_json::json!({"content": ["line"]})).is_none());
        assert!(structured_update_text(&serde_json::Value::Null).is_none());
    }

    #[test]
    fn is_transport_only_update_flags_status_only_envelopes() {
        assert!(is_transport_only_update(
            &serde_json::json!({"status": "running"})
        ));
        assert!(!is_transport_only_update(
            &serde_json::json!({"status": "running", "output": "hi"})
        ));
        assert!(!is_transport_only_update(&serde_json::json!({"step": 2})));
    }

    #[test]
    fn clear_event_resets_turn_lifecycle_state() {
        let mut state = FeedState::default();
        state.reduce(super::ScrollbackMsg::TurnStart);
        assert!(state.snapshot().turn_started);
        state.reduce(super::ScrollbackMsg::AssistantStreamStart);
        assert!(state.snapshot().assistant_stream_open);
        state.reduce(super::ScrollbackMsg::Clear);
        assert!(!state.snapshot().turn_started);
        assert!(!state.snapshot().assistant_stream_open);
    }

    #[test]
    fn assistant_stream_lifecycle_is_reducer_owned() {
        let mut state = FeedState::default();
        state.reduce(super::ScrollbackMsg::AssistantStreamStart);
        assert!(state.snapshot().assistant_stream_open);
        state.reduce(super::ScrollbackMsg::AssistantStreamEnd);
        assert!(!state.snapshot().assistant_stream_open);
    }

    #[test]
    fn mouse_selection_normalizes_reversed_cells_and_commits_through_events() {
        let mut state = FeedState::default();
        state.reduce(super::ScrollbackMsg::MouseSelectionStart(
            super::CellPosition {
                row: 10,
                column: 18,
            },
        ));
        state.reduce(super::ScrollbackMsg::MouseSelectionExtend(
            super::CellPosition { row: 8, column: 4 },
        ));
        let selection = state.snapshot().cell_selection.expect("selection");
        assert_eq!(
            selection.normalized(),
            (
                super::CellPosition { row: 8, column: 4 },
                super::CellPosition {
                    row: 10,
                    column: 18
                }
            )
        );
        state.reduce(super::ScrollbackMsg::MouseSelectionCommit);
        assert!(state.snapshot().cell_selection.is_some());
        state.reduce(super::ScrollbackMsg::RequestCopySelection);
        assert!(state.snapshot().copy_selection.is_some());
        state.reduce(super::ScrollbackMsg::ClearCopyRequest);
        assert!(state.snapshot().copy_selection.is_none());
        state.reduce(super::ScrollbackMsg::ClearCellSelection);
        assert!(state.snapshot().cell_selection.is_none());
    }

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
    fn format_elapsed_emits_empty_when_missing() {
        assert_eq!(super::format_elapsed(None), String::new());
        assert!(super::format_elapsed(None).is_empty());
    }

    #[test]
    fn format_elapsed_renders_seconds_for_some_value() {
        assert_eq!(super::format_elapsed(Some(1_500)), " in 1.5s");
        assert_eq!(super::format_elapsed(Some(0)), " in 0.0s");
    }

    #[test]
    fn format_error_with_error_flag_and_no_message_yields_empty() {
        assert_eq!(super::format_error(true, None), String::new());
        assert!(super::format_error(true, None).is_empty());
    }

    #[test]
    fn format_error_with_error_flag_and_message_renders_parenthesised_text() {
        assert_eq!(super::format_error(true, Some("boom")), " (boom)");
    }

    #[test]
    fn format_error_suppresses_suffix_when_not_error() {
        assert_eq!(super::format_error(false, None), String::new());
        assert_eq!(super::format_error(false, Some("ignored")), String::new());
    }

    #[test]
    fn thinking_summary_pins_default_and_observed_elapsed() {
        // Pin the fallback path: when no reasoning elapsed is observed, the
        // summary still renders the pinned default rather than an empty or
        // missing label, so replay and live paths share one identity.
        assert_eq!(
            super::thinking_summary(None),
            format!(
                "◆ Thought for {:.1}s",
                super::DEFAULT_THINKING_ELAPSED_MS as f64 / 1_000.0
            )
        );
        // Pin the observed path: an explicit elapsed value overrides the
        // default and renders the same "◆ Thought for …" shape.
        assert_eq!(super::thinking_summary(Some(2_500)), "◆ Thought for 2.5s");
    }

    #[test]
    fn running_bullet_pins_grok_frame_vocabulary_and_wraps() {
        // Pin the four source-backed Grok frames in order; the renderer
        // depends on the exact glyphs and trailing space.
        assert_eq!(super::RUNNING_BULLETS, ["⋅ ", ": ", "⸬ ", "⁙ "]);
        // Pin the frame projection: index 0..4 yields the vocabulary in
        // order, and index 4 wraps back to the first frame.
        assert_eq!(super::running_bullet(0), "⋅ ");
        assert_eq!(super::running_bullet(1), ": ");
        assert_eq!(super::running_bullet(2), "⸬ ");
        assert_eq!(super::running_bullet(3), "⁙ ");
        assert_eq!(super::running_bullet(4), "⋅ ");
        // Pin the wrap-around for a large frame index so the actor-owned
        // animation frame never panics on overflow.
        assert_eq!(super::running_bullet(usize::MAX), "⁙ ");
    }

    #[test]
    fn is_fence_detects_three_backtick_marker_with_or_without_grok_prefix() {
        // Pin the smoke path: a plain triple-backtick opening fence is
        // detected regardless of the renderer prefix.
        assert!(super::is_fence("```rust"));
        assert!(super::is_fence("```"));
        // Pin the Grok-prefix path: the renderer prefix must not hide the
        // fence marker so the actor-owned markdown classifier agrees.
        assert!(super::is_fence("┃ ```rust"));
        // Pin the negative paths: blank lines, single backticks, and prose
        // must not be misclassified as a code fence.
        assert!(!super::is_fence(""));
        assert!(!super::is_fence("`inline`"));
        assert!(!super::is_fence("hello world"));
    }

    #[test]
    fn is_table_row_requires_leading_trailing_pipe_and_two_separators() {
        // Pin the smoke path: a header row with three pipes is detected.
        assert!(super::is_table_row("| a | b | c |"));
        // Pin the body path: a row with surrounding whitespace still counts.
        assert!(super::is_table_row("  | x | y |  "));
        // Pin the single-cell row: a row with two pipes (start/end) is also
        // a table row, matching the existing renderer predicate.
        assert!(super::is_table_row("| single cell |"));
        // Pin the negative paths: an opening pipe only, a trailing pipe
        // only, and prose must not be misclassified as a table row.
        assert!(!super::is_table_row("| only opening"));
        assert!(!super::is_table_row("only trailing |"));
        assert!(!super::is_table_row("no pipes here"));
    }

    #[test]
    fn is_table_separator_accepts_only_dash_colon_and_whitespace_cells() {
        // Pin the smoke path: a Markdown table separator is detected.
        assert!(super::is_table_separator("| --- | :---: | ---: |"));
        assert!(super::is_table_separator("|---|---|"));
        // Pin the negative paths: cells with prose or non-alignment glyphs
        // must not be misclassified as a separator.
        assert!(!super::is_table_separator("| a | b | c |"));
        assert!(!super::is_table_separator("| — | — |")); // em-dash not allowed
        assert!(!super::is_table_separator(""));
    }

    #[test]
    fn atx_heading_returns_title_only_within_commonmark_levels() {
        // Pin the smoke path: a level-1 heading returns the title body.
        assert_eq!(super::atx_heading("# Title"), Some("Title"));
        // Pin the level range: levels 1..=6 are accepted, 0 and 7+ are not.
        assert_eq!(super::atx_heading("###### Title"), Some("Title"));
        assert_eq!(super::atx_heading("####### Title"), None);
        assert_eq!(super::atx_heading("Title"), None);
        // Pin the missing-space edge case: a hash run without a space is not
        // a heading under the CommonMark spec.
        assert_eq!(super::atx_heading("#Title"), None);
        // Pin the empty-title edge case: a heading mark with no body still
        // returns an empty title rather than `None`.
        assert_eq!(super::atx_heading("# "), Some(""));
    }

    #[test]
    fn table_bottom_border_aligns_with_separator_widths() {
        // Pin the smoke path: a three single-char header drives three
        // 3-char border segments (cell width + 2 padding) joined with `┴`.
        assert_eq!(super::table_bottom_border("| a | b | c |"), "└───┴───┴───┘");
        // Pin the wide-cell path: a four-cell header produces four border
        // segments sized to `cell_width + 2` so each column aligns with
        // the header text.
        assert_eq!(
            super::table_bottom_border("| a | bb | ccc | dddd |"),
            "└───┴────┴─────┴──────┘"
        );
        // Pin the noise-tolerance path: surrounding whitespace is trimmed
        // and does not change the border shape.
        assert_eq!(super::table_bottom_border("  | x | y |  "), "└───┴───┘");
    }

    #[test]
    fn append_wrapped_splits_long_lines_at_width_boundary() {
        // Pin the smoke path: a short string fits in a single row with the
        // supplied `code` flag preserved.
        let mut rows = Vec::new();
        super::append_wrapped(
            &mut rows,
            super::LineKind::Assistant,
            "hello".into(),
            true,
            10,
        );
        assert_eq!(
            rows,
            vec![(super::LineKind::Assistant, "hello".to_owned(), true)]
        );
        // Pin the wrap path: a long string splits into fixed-width chunks
        // until the source is exhausted.
        rows.clear();
        super::append_wrapped(
            &mut rows,
            super::LineKind::Assistant,
            "abcdefghij".into(),
            false,
            3,
        );
        assert_eq!(
            rows,
            vec![
                (super::LineKind::Assistant, "abc".to_owned(), false),
                (super::LineKind::Assistant, "def".to_owned(), false),
                (super::LineKind::Assistant, "ghi".to_owned(), false),
                (super::LineKind::Assistant, "j".to_owned(), false),
            ]
        );
        // Pin the zero-width edge case: a zero width yields a single row
        // holding the original text so the caller can decide how to handle
        // unbounded feeds.
        rows.clear();
        super::append_wrapped(&mut rows, super::LineKind::User, "x".into(), false, 0);
        assert_eq!(rows, vec![(super::LineKind::User, "x".to_owned(), false)]);
    }

    #[test]
    fn append_wrapped_words_breaks_on_whitespace_and_preserves_indent() {
        // Pin the word-break path: a long phrase wraps at the most recent
        // whitespace without breaking a word in half.
        let mut rows = Vec::new();
        super::append_wrapped_words(
            &mut rows,
            super::LineKind::Assistant,
            "the quick brown fox jumps over the lazy dog".into(),
            10,
        );
        let projected: Vec<&str> = rows.iter().map(|(_, text, _)| text.as_str()).collect();
        assert_eq!(
            projected,
            vec!["the quick", "brown fox", "jumps over", "the lazy", "dog"]
        );
        // Pin the leading-indent path: a leading whitespace run is
        // preserved across the wrap so the projected widget keeps its
        // original indentation.
        rows.clear();
        super::append_wrapped_words(
            &mut rows,
            super::LineKind::User,
            "    indented prompt".into(),
            8,
        );
        let projected: Vec<&str> = rows.iter().map(|(_, text, _)| text.as_str()).collect();
        assert_eq!(projected, vec!["    indented", "    prompt"]);
    }

    #[test]
    fn version_badge_pins_three_grok_welcome_variants() {
        // Pin the full variant: the long `v{version} · Beta` label that
        // the wide hero footer renders right-aligned.
        let full = super::version_badge(super::VersionBadgeVariant::Full);
        assert!(full.starts_with("runie v"), "{full}");
        assert!(full.ends_with(" · Beta"), "{full}");
        // Pin the hero-footer variant: the same version appears in the
        // `Beta · v{version}` order for the right-aligned wide hero.
        let footer = super::version_badge(super::VersionBadgeVariant::HeroFooter);
        assert!(footer.starts_with("runie Beta · v"), "{footer}");
        // Pin the inline variant: the compact `v{version}` form used in
        // compact widgets.
        let inline = super::version_badge(super::VersionBadgeVariant::HeroInline);
        assert!(inline.starts_with("runie v"), "{inline}");
        assert!(!inline.contains("Beta"), "{inline}");
    }

    #[test]
    fn is_quit_command_pins_grok_vocab_with_trim_and_lowercase() {
        // Pin the smoke path: the three Grok quit commands are detected.
        assert!(super::is_quit_command("exit"));
        assert!(super::is_quit_command("quit"));
        assert!(super::is_quit_command(":q"));
        // Pin the normalization path: leading/trailing whitespace and
        // mixed-case input are accepted as quit commands.
        assert!(super::is_quit_command("  QUIT  "));
        assert!(super::is_quit_command("Exit"));
        assert!(super::is_quit_command(":Q"));
    }

    #[test]
    fn is_quit_command_rejects_non_quit_inputs() {
        // Pin the negative paths: prose, partial matches, and empty input
        // are not quit commands so the router treats them as regular text.
        assert!(!super::is_quit_command(""));
        assert!(!super::is_quit_command("hello"));
        assert!(!super::is_quit_command("exiting"));
        assert!(!super::is_quit_command("quitting"));
        assert!(!super::is_quit_command(":quit"));
    }

    #[test]
    fn welcome_modal_lines_pins_idle_chrome_shape() {
        // Pin the smoke path: the modal emits exactly six `LineKind::System`
        // rows so the actor-owned welcome payload and the renderer agree
        // on the chrome line count.
        let lines = super::welcome_modal_lines();
        assert_eq!(lines.len(), 6);
        for line in &lines {
            assert_eq!(line.kind, super::LineKind::System);
        }
        // Pin the chrome shape: the surrounding `╭─` and `╰─` glyphs mark
        // the modal borders, `◆ session_start` closes the modal, and the
        // middle rows carry the model/help breadcrumb.
        let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
        assert!(texts[0].starts_with("╭─ Runie  v"), "{}", texts[0]);
        assert_eq!(texts[1], "│ main runie");
        assert_eq!(texts[2], "│ Model · runie-core");
        assert_eq!(texts[3], "│ /help for commands");
        assert_eq!(texts[4], "╰─");
        assert_eq!(texts[5], "◆ session_start");
    }

    #[test]
    fn session_start_messages_emits_three_bracket_rows() {
        // Pin the smoke path: the projection emits exactly three messages
        // so the actor-owned session-start projection and the renderer
        // agree on the wrapping shape.
        let messages = super::session_start_messages();
        assert_eq!(messages.len(), 3);
        assert!(matches!(&messages[0], super::ScrollbackMsg::Append(_)));
        assert!(matches!(&messages[1], super::ScrollbackMsg::Append(_)));
        assert!(matches!(&messages[2], super::ScrollbackMsg::Append(_)));
    }

    #[test]
    fn session_start_messages_pins_separator_and_hooks_content() {
        // Pin the wrapping shape: the outer rows are blank `Separator`
        // lines and the middle row is the `SessionStart` marker with the
        // `[hooks: 1]` count.
        let messages = super::session_start_messages();
        let first = match &messages[0] {
            super::ScrollbackMsg::Append(line) => line,
            other => panic!("expected separator append, got {other:?}"),
        };
        assert_eq!(first.kind, super::LineKind::Separator);
        assert!(first.text.is_empty());
        let middle = match &messages[1] {
            super::ScrollbackMsg::Append(line) => line,
            other => panic!("expected session start append, got {other:?}"),
        };
        assert_eq!(middle.kind, super::LineKind::SessionStart);
        assert_eq!(middle.text, "◆ session_start  [hooks: 1]");
        let last = match &messages[2] {
            super::ScrollbackMsg::Append(line) => line,
            other => panic!("expected separator append, got {other:?}"),
        };
        assert_eq!(last.kind, super::LineKind::Separator);
        assert!(last.text.is_empty());
    }

    #[test]
    fn append_user_with_timestamp_right_aligns_timestamp_into_first_row() {
        // Pin the gutter path: the timestamp appears at the right edge of
        // the first row, with the prompt text filling the leading
        // columns and the trailing ` TIMESTAMP_EDGE_OFFSET` slack.
        let mut rows = Vec::new();
        super::append_user_with_timestamp(&mut rows, "hello world".into(), "3:07 PM", 40);
        let first = &rows[0];
        assert_eq!(first.0, super::LineKind::User);
        assert!(first.1.starts_with("hello world"), "{}", first.1);
        assert!(first.1.ends_with("3:07 PM"), "{}", first.1);
        assert!(!first.2);
    }

    #[test]
    fn append_user_with_timestamp_wraps_remaining_text_with_indent() {
        // Pin the wrap path: a long prompt that exceeds the gutter width
        // emits a continuation row indent matching the `LineKind::User`
        // prefix so the projected widget keeps its indentation.
        let mut rows = Vec::new();
        super::append_user_with_timestamp(
            &mut rows,
            "the quick brown fox jumps over the lazy dog".into(),
            "3:07 PM",
            10,
        );
        assert!(rows.len() >= 2);
        // Pin the smoke path: the first row holds the timestamp and the
        // remaining rows wrap the rest of the prompt text.
        assert!(rows[0].1.contains("3:07 PM"));
        for row in &rows[1..] {
            assert_eq!(row.0, super::LineKind::User);
        }
    }

    #[test]
    fn make_relative_path_strips_workspace_and_collapses_to_dot() {
        // Pin the smoke path: the workspace-only path collapses to `.`
        // so the rendered header is a clean directory anchor.
        assert_eq!(super::make_relative_path("/work", "/work"), ".");
        // Pin the workspace-relative path: a leading separator is
        // stripped so the rendered header never shows `<workspace>/`.
        assert_eq!(super::make_relative_path("/work", "/work/file"), "file");
        // Pin the nested path: a deeper workspace-relative path keeps
        // its directory structure intact.
        assert_eq!(
            super::make_relative_path("/work", "/work/dir/sub/file"),
            "dir/sub/file"
        );
        // Pin the negative path: a path outside the workspace is
        // returned verbatim so the renderer can decide how to label it.
        assert_eq!(
            super::make_relative_path("/work", "/tmp/other/file"),
            "/tmp/other/file"
        );
    }

    #[test]
    fn grok_effective_compact_pins_user_and_terminal_signal() {
        // Pin the user signal: an explicit user compact override always
        // wins, regardless of measured terminal height.
        assert!(super::grok_effective_compact(true, 0));
        assert!(super::grok_effective_compact(true, 80));
        // Pin the terminal signal: an unmeasured height (zero rows)
        // does not force compact mode so the renderer can wait for a
        // real measurement.
        assert!(!super::grok_effective_compact(false, 0));
        // Pin the auto-compact band: heights at or below
        // `GROK_AUTO_COMPACT_MAX_ROWS` force compact mode.
        assert!(super::grok_effective_compact(
            false,
            super::GROK_AUTO_COMPACT_MAX_ROWS
        ));
        // Pin the full-mode range: heights above the auto-compact band
        // do not force compact mode.
        assert!(!super::grok_effective_compact(
            false,
            super::GROK_AUTO_COMPACT_MAX_ROWS + 1
        ));
    }

    #[test]
    fn grok_small_screen_tip_visible_targets_the_pre_compact_band() {
        // Pin the boundary: the tip is hidden at and below the
        // auto-compact threshold.
        assert!(!super::grok_small_screen_tip_visible(
            super::GROK_AUTO_COMPACT_MAX_ROWS
        ));
        // Pin the smoke path: heights strictly above the auto-compact
        // threshold and at or below the tip max are visible.
        assert!(super::grok_small_screen_tip_visible(
            super::GROK_AUTO_COMPACT_MAX_ROWS + 1
        ));
        assert!(super::grok_small_screen_tip_visible(
            super::GROK_SMALL_SCREEN_TIP_MAX_ROWS
        ));
        // Pin the upper bound: the tip is hidden above the max.
        assert!(!super::grok_small_screen_tip_visible(
            super::GROK_SMALL_SCREEN_TIP_MAX_ROWS + 1
        ));
    }

    #[test]
    fn model_selector_rows_renders_provider_slash_model_pairs() {
        use runie_core::model_catalog::ModelCatalogSnapshot;
        use runie_core::types::Model;
        let snapshot = ModelCatalogSnapshot {
            catalog: runie_core::model_catalog::ModelCatalog::new(Vec::new(), Vec::new()),
            query: String::new(),
            scoped_only: false,
            results: vec![
                Model {
                    id: "gpt-4o".into(),
                    name: "GPT-4o".into(),
                    api: "openai".into(),
                    provider: "openai".into(),
                    ..Default::default()
                },
                Model {
                    id: "claude-3-5-sonnet".into(),
                    name: "Claude".into(),
                    api: "anthropic".into(),
                    provider: "anthropic".into(),
                    ..Default::default()
                },
            ],
            selected: None,
            last_event: None,
        };
        let rows = super::model_selector_rows(&snapshot);
        assert_eq!(rows, vec!["openai/gpt-4o", "anthropic/claude-3-5-sonnet"]);
    }

    #[test]
    fn model_selector_rows_returns_empty_for_empty_snapshot() {
        use runie_core::model_catalog::ModelCatalogSnapshot;
        let snapshot = ModelCatalogSnapshot::default();
        assert!(super::model_selector_rows(&snapshot).is_empty());
    }

    #[test]
    fn format_clock_timestamp_pins_short_clock_shape() {
        // Pin the fallback path: when libc cannot resolve the local clock,
        // the UTC-derived 12-hour shape with a zero-padded minute is still
        // emitted so the label stays well-formed for replay and live paths.
        for timestamp in [0, 13 * 3_600 + 7 * 60, 12 * 3_600] {
            let formatted = super::format_clock_timestamp(timestamp);
            assert!(formatted.contains(':'), "{formatted}");
            assert!(
                formatted.ends_with(" AM") || formatted.ends_with(" PM"),
                "{formatted}"
            );
            // Pin the zero-padded minute shape: the colon-aligned header
            // must keep minutes as exactly two digits regardless of
            // meridiem or 12-hour rollover.
            let minute_segment = formatted
                .split(':')
                .nth(1)
                .unwrap_or_else(|| panic!("missing minute segment: {formatted}"));
            let minute_part = minute_segment
                .split(' ')
                .next()
                .unwrap_or_else(|| panic!("missing minute digits: {formatted}"));
            assert_eq!(
                minute_part.len(),
                2,
                "minute must be zero-padded to two digits: {formatted}"
            );
        }
    }

    #[test]
    fn tool_update_header_text_appends_serialized_json_fragment() {
        assert_eq!(
            super::tool_update_header_text(
                "Run ls",
                &serde_json::json!({"status": "running", "step": 2})
            ),
            "Run ls | update: {\"status\":\"running\",\"step\":2}"
        );
        assert_eq!(
            super::tool_update_header_text("Read src/lib.rs", &serde_json::Value::Null),
            "Read src/lib.rs | update: null"
        );
    }

    #[test]
    fn tool_update_header_text_keeps_separator_for_empty_serialization() {
        // `serde_json::Value` always serializes, so the `unwrap_or_default()`
        // fallback degrades to an empty fragment rather than a panic. Pin the
        // header shape around a minimal payload and around the empty default.
        let fragment = serde_json::to_string(&serde_json::json!({})).unwrap_or_default();
        assert_eq!(fragment, "{}");
        assert_eq!(
            super::tool_update_header_text("Run ls", &serde_json::json!({})),
            format!("Run ls | update: {fragment}")
        );
        assert_eq!(
            super::tool_update_header_text("", &serde_json::json!({})),
            " | update: {}"
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
            super::workflow_text(
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
        assert_eq!(state.snapshot().scroll_offset, 4);
        state.reduce(super::ScrollbackMsg::LayoutMeasured {
            content_rows: 34,
            viewport_rows: 6,
            anchor_row: Some(21),
        });
        assert_eq!(state.snapshot().scroll_offset, 8);
        assert!(!state.snapshot().autoscroll);
    }

    #[test]
    fn web_search_sources_line_dedups_and_keeps_first_seen_order() {
        assert_eq!(
            super::web_search_sources_line(
                "https://docs.rs/runie https://docs.rs/ratatui https://rust-lang.org/learn https://github.com/runie https://docs.rs/extra"
            ),
            Some("  Sources: docs.rs, rust-lang.org, github.com".to_owned())
        );
    }

    #[test]
    fn web_search_sources_line_returns_none_for_empty_source_line() {
        assert_eq!(super::web_search_sources_line(""), None);
        assert_eq!(super::web_search_sources_line("   \n  "), None);
        assert_eq!(super::web_search_sources_line("no citations"), None);
    }

    #[test]
    fn web_search_sources_line_paginates_with_plus_n_more() {
        assert_eq!(
            super::web_search_sources_line(
                "https://a.example https://b.example https://c.example https://d.example https://e.example"
            ),
            Some("  Sources: a.example, b.example, c.example (+2 more)".to_owned())
        );
    }

    #[test]
    fn web_search_sources_line_trims_url_terminators_and_punctuation() {
        assert_eq!(
            super::web_search_sources_line(
                "see https://docs.rs/runie/page, https://crates.io?q=foo#bar and https://github.com/path) (also https://github.com/path] more"
            ),
            Some("  Sources: docs.rs, crates.io, github.com".to_owned())
        );
    }

    #[test]
    fn web_search_site_count_dedups_case_insensitively() {
        assert_eq!(
            super::web_search_site_count(
                "https://docs.rs/a\nhttps://DOCS.RS/b\nhttps://rust-lang.org/learn"
            ),
            2
        );
    }

    #[test]
    fn web_search_site_count_trims_url_terminators_and_punctuation() {
        assert_eq!(
            super::web_search_site_count(
                "see https://docs.rs/a), https://crates.io?q=foo#bar, https://github.com/b"
            ),
            3
        );
    }

    #[test]
    fn web_search_site_count_falls_back_to_non_empty_lines_when_url_free() {
        assert_eq!(super::web_search_site_count("one\ntwo\n\nthree\n"), 3);
        assert_eq!(super::web_search_site_count("plain prose only"), 1);
    }

    #[test]
    fn completed_tool_header_with_args_pins_search_tools_aliases_and_cardinality() {
        let empty_args = serde_json::json!({});
        assert_eq!(
            super::completed_tool_header_with_args(
                "Search tools",
                "search_tools",
                &empty_args,
                &serde_json::json!("tool_alpha"),
            ),
            "Search tools (1 result)"
        );
        assert_eq!(
            super::completed_tool_header_with_args(
                "Search tools",
                "search-tools",
                &empty_args,
                &serde_json::json!("tool_alpha\ntool_beta\ntool_gamma"),
            ),
            "Search tools (3 results)"
        );
        assert_eq!(
            super::completed_tool_header_with_args(
                "Search tools",
                "search_tool",
                &empty_args,
                &serde_json::json!("tool_alpha\n\ntool_beta"),
            ),
            "Search tools (2 results)"
        );
    }

    #[test]
    fn tool_header_pins_search_tools_aliases_and_workspace_anchor() {
        let workspace = "/repo/root";
        // All three Grok aliases route through the same semantic header.
        for alias in ["search_tools", "search-tools", "search_tool"] {
            assert_eq!(
                super::tool_header(alias, &serde_json::json!({"query": "alpha"}), workspace),
                "Search Tools alpha",
                "alias: {alias}"
            );
        }
        // The `pattern` key is the documented fallback when `query` is missing.
        assert_eq!(
            super::tool_header(
                "search_tools",
                &serde_json::json!({"pattern": "alpha"}),
                workspace,
            ),
            "Search Tools alpha"
        );
        // Missing both keys falls back to the empty placeholder.
        assert_eq!(
            super::tool_header("search_tools", &serde_json::json!({}), workspace),
            "Search Tools "
        );
        // `query` wins over `pattern` when both keys are present.
        assert_eq!(
            super::tool_header(
                "search_tools",
                &serde_json::json!({"query": "first", "pattern": "second"}),
                workspace,
            ),
            "Search Tools first"
        );
        // The workspace anchor is threaded through alongside the path closure
        // even when the alias does not project it, so the renderer can keep a
        // single call site.
        assert_eq!(
            super::tool_header(
                "search_tools",
                &serde_json::json!({"query": "alpha"}),
                "/different/anchor",
            ),
            "Search Tools alpha"
        );
    }

    #[test]
    fn completed_tool_header_with_args_routes_read_file_image_content() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Read src/diagram.png",
                "read_file",
                &serde_json::json!({"path": "src/diagram.png"}),
                &serde_json::json!({
                    "content": [
                        {"type": "image", "data": "ZmFrZQ=="}
                    ]
                })
            ),
            "Read src/diagram.png (image)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_renders_read_file_offset_range_with_total() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Read src/lib.rs",
                "read_file",
                &serde_json::json!({"offset": 40, "limit": 20}),
                &serde_json::json!({
                    "content": [{"text": "line 41\nline 42\n[18 more lines in file. Use offset=61 to continue.]"}],
                    "details": {"truncation": {"totalLines": 100}}
                })
            ),
            "Read src/lib.rs (41-42 of 100)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_projects_list_dir_cardinality() {
        let args = serde_json::json!({});
        assert_eq!(
            super::completed_tool_header_with_args(
                "List .",
                "list_dir",
                &args,
                &serde_json::json!("Cargo.toml"),
            ),
            "List . (1 entry)"
        );
        assert_eq!(
            super::completed_tool_header_with_args(
                "List .",
                "list_files",
                &args,
                &serde_json::json!("Cargo.toml\nsrc\ncrates"),
            ),
            "List . (3 entries)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_projects_read_line_count() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Read README.md",
                "read",
                &serde_json::json!({}),
                &serde_json::json!("a\nb"),
            ),
            "Read README.md (2 lines)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_projects_search_match_cardinality() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Search \"TODO\"",
                "search",
                &serde_json::json!({}),
                &serde_json::json!("a\nb"),
            ),
            "Search \"TODO\" (2 matches)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_projects_edit_count() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Edit src/main.rs",
                "edit",
                &serde_json::json!({}),
                &serde_json::json!("hunk"),
            ),
            "Edit src/main.rs (1 edit)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_routes_workflow_to_completed_label() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Workflow release",
                "workflow",
                &serde_json::json!({}),
                &serde_json::json!("done"),
            ),
            "Workflow completed: release"
        );
    }

    #[test]
    fn completed_tool_header_with_args_routes_use_to_used_label() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Use git_status",
                "use",
                &serde_json::json!({}),
                &serde_json::json!("{}"),
            ),
            "Used git_status"
        );
    }

    #[test]
    fn completed_tool_header_with_args_routes_subagent_to_completed_label() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Subagent started: research",
                "subagent",
                &serde_json::json!({}),
                &serde_json::json!("done"),
            ),
            "Subagent completed: research"
        );
    }

    #[test]
    fn completed_tool_header_with_args_projects_web_search_site_count() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Web Search rust",
                "web_search",
                &serde_json::json!({}),
                &serde_json::json!("see https://docs.rs/a and https://crates.io/b"),
            ),
            "Web Search rust (2 sites)"
        );
    }

    #[test]
    fn completed_tool_header_with_args_projects_memory_search_results() {
        assert_eq!(
            super::completed_tool_header_with_args(
                "Memory Search actors",
                "memory_search",
                &serde_json::json!({}),
                &serde_json::json!(
                    "### Result 1 (score: 0.72, source: global)\n**File:** /memory/MEMORY.md (lines 0-1)\n```\none\n```\n### Result 2 (score: 0.42, source: session)\n**File:** /memory/session.md (lines 2-3)\n```\ntwo\n```"
                ),
            ),
            "Memory Search actors (2 results)"
        );
    }
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

/// Pure actor-owned feed state. It contains transcript facts and navigation
/// only; terminal geometry, styles, and Ratatui buffers remain outside this
/// crate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedState {
    pub lines: Vec<Line>,
    pub navigation: FeedNavigation,
}

impl FeedState {
    #[allow(
        clippy::too_many_lines,
        reason = "the immutable feed projection keeps every actor-owned fact explicit"
    )]
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
            tool_args: self.navigation.tool_args.clone(),
            activity_dirs: self.navigation.activity_dirs,
            activity_files: self.navigation.activity_files,
            activity_commands: self.navigation.activity_commands,
            activity_subagents: self.navigation.activity_subagents,
            activity_failures: self.navigation.activity_failures,
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
            selection_anchor: self.navigation.selection_anchor,
            selection_head: self.navigation.selection_head,
            cell_selection: self.navigation.cell_selection,
            copy_selection: self.navigation.copy_selection,
            selected_member_index,
            theme: self.navigation.theme,
            animation_frame: self.navigation.animation_frame,
            tool_modes: self.navigation.tool_modes.clone(),
            turn_started: self.navigation.turn_started,
            assistant_stream_open: self.navigation.assistant_stream_open,
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
            ScrollbackMsg::AssistantStreamStart => self.navigation.assistant_stream_open = true,
            ScrollbackMsg::AssistantStreamEnd => self.navigation.assistant_stream_open = false,
            ScrollbackMsg::Clear => self.clear(),
            ScrollbackMsg::SetTheme(theme) => self.navigation.theme = theme,
            ScrollbackMsg::SetToolArgs(id, args) => {
                self.navigation.tool_args.insert(id, args);
            }
            ScrollbackMsg::RemoveToolArgs(id) => {
                self.navigation.tool_args.remove(&id);
            }
            ScrollbackMsg::ActivityReset => {
                self.navigation.activity_dirs = 0;
                self.navigation.activity_files = 0;
                self.navigation.activity_commands = 0;
                self.navigation.activity_subagents = 0;
                self.navigation.activity_failures = 0;
            }
            ScrollbackMsg::ActivityToolStart(name) => match classify_activity_tool(name.as_str()) {
                Some(ActivityKind::Dir) => self.navigation.activity_dirs += 1,
                Some(ActivityKind::File) => self.navigation.activity_files += 1,
                Some(ActivityKind::Command) => self.navigation.activity_commands += 1,
                Some(ActivityKind::Subagent) => self.navigation.activity_subagents += 1,
                None => {}
            },
            ScrollbackMsg::ActivityToolEnd { is_error } => {
                if is_error {
                    self.navigation.activity_failures += 1;
                }
            }
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
            ScrollbackMsg::SelectRange { anchor, head } => {
                self.navigation.selection_anchor = Some(anchor);
                self.navigation.selection_head = Some(head);
            }
            ScrollbackMsg::ClearSelection => {
                self.navigation.selection_anchor = None;
                self.navigation.selection_head = None;
            }
            ScrollbackMsg::MouseSelectionStart(position) => {
                self.navigation.cell_selection_anchor = Some(position);
                self.navigation.cell_selection = None;
            }
            ScrollbackMsg::MouseSelectionExtend(position) => {
                if let Some(anchor) = self.navigation.cell_selection_anchor {
                    self.navigation.cell_selection = Some(CellSelection {
                        anchor,
                        head: position,
                    });
                }
            }
            ScrollbackMsg::MouseSelectionCommit => {
                self.navigation.cell_selection_anchor = None;
            }
            ScrollbackMsg::ClearCellSelection => {
                self.navigation.cell_selection_anchor = None;
                self.navigation.cell_selection = None;
            }
            ScrollbackMsg::RequestCopySelection => {
                self.navigation.copy_selection = self.navigation.cell_selection;
            }
            ScrollbackMsg::ClearCopyRequest => {
                self.navigation.copy_selection = None;
            }
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
                if !self.navigation.autoscroll {
                    if let (Some(previous), Some(current)) =
                        (self.navigation.measured_anchor_row, anchor_row)
                    {
                        if current >= previous {
                            self.navigation.scroll_offset = self
                                .navigation
                                .scroll_offset
                                .saturating_add(current - previous);
                        } else {
                            self.navigation.scroll_offset = self
                                .navigation
                                .scroll_offset
                                .saturating_sub(previous - current);
                        }
                    }
                }
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
                    workflow_text(&header, &phases, "active", None, active_agents),
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
                    workflow_text(&header, &phases, &status, elapsed_ms, 0),
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
        self.navigation.tool_args.clear();
        self.navigation.activity_dirs = 0;
        self.navigation.activity_files = 0;
        self.navigation.activity_commands = 0;
        self.navigation.activity_subagents = 0;
        self.navigation.activity_failures = 0;
        self.navigation.tool_modes.clear();
        self.navigation.workflow_headers.clear();
        self.navigation.workflow_phases.clear();
        self.navigation.revealed_dense_groups.clear();
        self.navigation.next_tool_row_id = 0;
        self.navigation.selected_tool_id = None;
        self.navigation.selected_entry = None;
        self.navigation.scroll_offset = 0;
        self.navigation.follow_latest_user = false;
        self.navigation.turn_started = false;
        self.navigation.assistant_stream_open = false;
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
        let latest_user = self
            .lines
            .iter()
            .rposition(|line| line.kind == LineKind::User);
        let latest_activity = self
            .lines
            .iter()
            .enumerate()
            .rev()
            .find(|(_, line)| line.kind == LineKind::Activity)
            .map(|(index, _)| index);
        if let Some(index) =
            latest_activity.filter(|index| latest_user.is_none_or(|user_index| *index > user_index))
        {
            self.lines[index].text = activity;
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
