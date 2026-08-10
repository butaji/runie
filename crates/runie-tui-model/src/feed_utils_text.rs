/// Semantic category for grouped activity-tool counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Dir,
    File,
    Command,
    Subagent,
}

macro_rules! activity_tool_groups {
    ($name:expr => {
        $( $kind:ident: [$($alias:literal),+ $(,)?] ),+ $(,)?
    }) => {
        match $name {
            $( $($alias)|+ => Some(ActivityKind::$kind), )+
            _ => None,
        }
    };
}

/// Classify a tool name for grouped activity presentation.
pub fn classify_activity_tool(tool_name: &str) -> Option<ActivityKind> {
    activity_tool_groups!(tool_name => {
        Dir: ["list_dir", "list_files", "ls"],
        File: ["read", "read_file"],
        Command: [
            "bash", "shell", "exec", "run", "execute",
            "run_terminal_command", "run_terminal_cmd"
        ],
        Subagent: ["subagent", "agent", "task"],
    })
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

pub fn tool_header(tool_name: &str, args: &serde_json::Value, workspace: &str) -> String {
    tool_header_path(tool_name, args, workspace)
        .or_else(|| tool_header_search(tool_name, args, workspace))
        .or_else(|| tool_header_runtime(tool_name, args))
        .unwrap_or_else(|| {
            format!(
                "{tool_name} {}",
                serde_json::to_string(args).unwrap_or_default()
            )
        })
}

fn string_value(args: &serde_json::Value, keys: &[&str], fallback: &str) -> String {
    keys.iter()
        .find_map(|key| args.get(*key).and_then(serde_json::Value::as_str))
        .unwrap_or(fallback)
        .to_owned()
}

fn relative_path(value: &str, workspace: &str) -> String {
    value
        .strip_prefix(workspace)
        .and_then(|rest| rest.strip_prefix('/'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(value)
        .to_owned()
}

fn tool_header_path(name: &str, args: &serde_json::Value, workspace: &str) -> Option<String> {
    let header = match name {
        "list_dir" | "list_files" | "ls" => format!(
            "List {}",
            relative_path(&string_value(args, &["path"], "."), workspace)
        ),
        "read" | "read_file" => format!(
            "Read {}",
            relative_path(&string_value(args, &["path"], ""), workspace)
        ),
        "edit" | "write" | "write_file" | "search_replace" | "apply_patch" | "strreplace" => {
            format!(
                "Edit {}",
                relative_path(&string_value(args, &["path", "file_path"], ""), workspace)
            )
        }
        _ => return None,
    };
    Some(header)
}

fn tool_header_search(name: &str, args: &serde_json::Value, workspace: &str) -> Option<String> {
    if matches!(name, "web_search" | "web-search") {
        return Some(format!(
            "Web Search {}",
            string_value(args, &["query", "q"], "")
        ));
    }
    if matches!(name, "web_fetch" | "web-fetch" | "fetch") {
        return Some(format!("Fetch {}", string_value(args, &["url"], "")));
    }
    if !matches!(name, "search" | "grep" | "find" | "glob") {
        return None;
    }
    let pattern = string_value(args, &["pattern", "query"], "");
    let location = args
        .get("path")
        .or_else(|| args.get("cwd"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty());
    Some(
        location
            .map(|value| format!("Search {pattern:?} in {}", relative_path(value, workspace)))
            .unwrap_or_else(|| format!("Search {pattern:?}")),
    )
}

fn tool_header_runtime(name: &str, args: &serde_json::Value) -> Option<String> {
    let header = match name {
        "bash"
        | "shell"
        | "exec"
        | "run"
        | "execute"
        | "run_terminal_command"
        | "run_terminal_cmd" => format!("Run {}", string_value(args, &["command", "cmd"], "")),
        "subagent" | "agent" | "task" => format!(
            "Subagent started: {:?}",
            string_value(args, &["description", "task", "prompt"], "")
        ),
        "workflow" | "run_workflow" | "run-workflow" => {
            format!("Workflow {}", string_value(args, &["name", "workflow"], ""))
        }
        "use" | "use_tool" | "use-tool" => {
            format!("Use {}", string_value(args, &["tool", "name"], ""))
        }
        "memory_search" | "memory-search" => {
            format!("Memory Search {}", string_value(args, &["query", "q"], ""))
        }
        "search_tools" | "search-tools" | "search_tool" => format!(
            "Search Tools {}",
            string_value(args, &["query", "pattern"], "")
        ),
        "todo" | "todo_write" | "todo-write" => format!(
            "Todo {}",
            string_value(args, &["title", "task"], "Update todos")
        ),
        _ => return None,
    };
    Some(header)
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
pub fn activity_text(
    dirs: usize,
    files: usize,
    commands: usize,
    subagents: usize,
    failures: usize,
    running: bool,
) -> String {
    let verbs = if running {
        [
            ("Listing", "dir"),
            ("Reading", "file"),
            ("Running", "command"),
            ("Running", "subagent"),
        ]
    } else {
        [
            ("Listed", "dir"),
            ("Read", "file"),
            ("Ran", "command"),
            ("Ran", "subagent"),
        ]
    };
    let parts = [dirs, files, commands, subagents]
        .into_iter()
        .zip(verbs)
        .filter_map(|(count, (verb, noun))| activity_part(count, verb, noun))
        .collect::<Vec<_>>();
    let mut text = format!("◈ {}", parts.join(", "));
    if failures > 0 && !running {
        text.push_str(&format!(" · {failures} failed"));
    }
    text
}

fn activity_part(count: usize, verb: &str, noun: &str) -> Option<String> {
    (count > 0).then(|| format!("{verb} {count} {noun}{}", if count == 1 { "" } else { "s" }))
}

/// Add Grok's result cardinality/range suffix to a retained tool header.
pub fn completed_tool_header_with_args(
    pending_header: &str,
    tool_name: &str,
    args: &serde_json::Value,
    result: &serde_json::Value,
) -> String {
    let output = tool_result_text(result);
    if let Some(header) = read_completion_header(tool_name, pending_header, args, result, &output) {
        return header;
    }
    let count = |nonempty: bool| {
        output
            .lines()
            .filter(|line| !nonempty || !line.trim().is_empty())
            .count()
    };
    completion_header_for_tool(pending_header, tool_name, &output, count)
}

fn completion_header_for_tool(
    pending_header: &str,
    tool_name: &str,
    output: &str,
    count: impl Fn(bool) -> usize,
) -> String {
    completion_cardinality(pending_header, tool_name, output, &count)
        .or_else(|| completion_state_change(pending_header, tool_name))
        .or_else(|| completion_edit(pending_header, tool_name, &count))
        .unwrap_or_else(|| format!("{pending_header} → ✓"))
}

fn completion_cardinality(
    header: &str,
    tool: &str,
    output: &str,
    count: &impl Fn(bool) -> usize,
) -> Option<String> {
    let (n, noun) = match tool {
        "list_dir" | "list_files" | "ls" => (count(true), "entr"),
        "read" | "read_file" => (output.lines().count(), "line"),
        "search" | "grep" | "find" | "glob" => (count(true), "match"),
        "web_search" | "web-search" => (web_search_site_count(output), "site"),
        "search_tools" | "search-tools" | "search_tool" => (count(true), "result"),
        "memory_search" | "memory-search" => {
            (crate::memory::parse_memory_results(output).len(), "result")
        }
        _ => return None,
    };
    let suffix = match noun {
        "entr" if n != 1 => "ies",
        "entr" => "y",
        "match" if n != 1 => "es",
        _ if n != 1 => "s",
        _ => "",
    };
    Some(format!("{header} ({n} {noun}{suffix})"))
}

fn completion_state_change(header: &str, tool: &str) -> Option<String> {
    let (prefix, replacement) = match tool {
        "workflow" | "run_workflow" | "run-workflow" => ("Workflow ", "Workflow completed: "),
        "use" | "use_tool" | "use-tool" => ("Use ", "Used "),
        "subagent" | "agent" | "task" => ("Subagent started: ", "Subagent completed: "),
        _ => return None,
    };
    Some(
        header
            .strip_prefix(prefix)
            .map_or_else(|| header.to_owned(), |name| format!("{replacement}{name}")),
    )
}

fn completion_edit(header: &str, tool: &str, count: &impl Fn(bool) -> usize) -> Option<String> {
    if !matches!(
        tool,
        "edit" | "write" | "write_file" | "search_replace" | "todo" | "todo_write" | "todo-write"
    ) {
        return None;
    }
    let n = count(true);
    if n == 0 {
        return Some(header.to_owned());
    }
    let noun = if tool.starts_with("todo") {
        "item"
    } else {
        "edit"
    };
    Some(format!(
        "{header} ({n} {noun}{})",
        if n == 1 { "" } else { "s" }
    ))
}

fn read_completion_header(
    tool_name: &str,
    pending_header: &str,
    args: &serde_json::Value,
    result: &serde_json::Value,
    output: &str,
) -> Option<String> {
    if !matches!(tool_name, "read" | "read_file") {
        return None;
    }
    if result
        .get("content")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| {
            items
                .iter()
                .any(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("image"))
        })
    {
        return Some(format!("{pending_header} (image)"));
    }
    let offset = args.get("offset").and_then(serde_json::Value::as_u64)?;
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
        .or_else(|| output.lines().find_map(parse_total_lines));
    Some(match total {
        Some(total) => format!("{pending_header} ({}-{} of {total})", offset + 1, end),
        None => format!("{pending_header} ({}-{end})", offset + 1),
    })
}

fn parse_total_lines(line: &str) -> Option<u64> {
    line.split(" of ")
        .nth(1)
        .and_then(|part| part.split(|ch: char| !ch.is_ascii_digit()).next())
        .and_then(|value| value.parse().ok())
}
