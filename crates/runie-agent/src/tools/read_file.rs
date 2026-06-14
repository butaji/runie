//! Read-file tool implementation.

pub fn read_file(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    _policy: &crate::truncate::TruncationPolicy,
) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    match std::fs::read_to_string(&path) {
        Ok(content) => read_file_content(&content, offset, limit),
        Err(e) => (format!("Error reading {}: {}", path.display(), e), false),
    }
}

fn read_file_content(content: &str, offset: Option<usize>, limit: Option<usize>) -> (String, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let start = offset.unwrap_or(0).min(total_lines);
    let end = limit
        .map(|l| (start + l).min(total_lines))
        .unwrap_or(total_lines);
    if start >= total_lines {
        return ("(end of file)".to_string(), true);
    }
    let selected: String = lines[start..end].join("\n");
    let output = format_file_output(&selected, offset, limit, start, end, total_lines);
    if end < total_lines {
        (
            format!("{}\n[{} more lines]", output, total_lines - end),
            true,
        )
    } else {
        (output, true)
    }
}

fn format_file_output(
    selected: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    start: usize,
    end: usize,
    total_lines: usize,
) -> String {
    if offset.is_some() || limit.is_some() {
        format!(
            "[Lines {}-{} of {}]\n{}",
            start + 1,
            end,
            total_lines,
            selected
        )
    } else {
        selected.to_string()
    }
}
