use super::{apply_truncation, Tool};
use crate::diff::Diff;
use crate::path_utils::resolve_path;
use crate::truncate::TruncationPolicy;
use runie_core::tool::{ToolOutput, ToolStatus};
use std::time::Instant;

pub(crate) fn list_dir(tool: &Tool, policy: &TruncationPolicy) -> ToolOutput {
    let start = Instant::now();
    let name = tool.name();
    let args = tool.to_args();
    let path = if let Tool::ListDir { path } = tool {
        path
    } else {
        unreachable!()
    };
    let resolved = resolve_path(path);
    match std::fs::read_dir(&resolved) {
        Ok(entries) => build_dir_output(name, &args, entries, policy, start),
        Err(e) => ToolOutput {
            tool_name: name.to_string(),
            tool_args: args,
            content: format!("Error listing {}: {}", resolved.display(), e),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Error,
        },
    }
}

fn build_dir_output(
    name: &str,
    args: &serde_json::Value,
    entries: std::fs::ReadDir,
    policy: &TruncationPolicy,
    start: Instant,
) -> ToolOutput {
    let mut lines = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let typ = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            "dir"
        } else {
            "file"
        };
        lines.push(format!("{} ({})", name, typ));
    }
    let content = if lines.is_empty() {
        "(empty directory)".to_string()
    } else {
        lines.join("\n")
    };
    ToolOutput {
        tool_name: name.to_string(),
        tool_args: args.clone(),
        content: apply_truncation(content, crate::accumulator::TruncateStrategy::Head, policy),
        bytes_transferred: None,
        duration: start.elapsed(),
        status: ToolStatus::Success,
    }
}

pub(crate) fn edit_file(tool: &Tool, _policy: &TruncationPolicy) -> ToolOutput {
    let start = Instant::now();
    let name = tool.name().to_string();
    let args = tool.to_args();
    let (path, search, replace) = if let Tool::EditFile {
        path,
        search,
        replace,
    } = tool
    {
        (path, search, replace)
    } else {
        unreachable!()
    };
    let resolved = resolve_path(path);

    if search.is_empty() {
        return edit_error(&name, &args, "search text cannot be empty", start.elapsed());
    }
    match std::fs::read_to_string(&resolved) {
        Ok(content) => apply_edit(
            &name,
            &args,
            &resolved,
            &content,
            search,
            replace,
            start.elapsed(),
        ),
        Err(e) => edit_error(
            &name,
            &args,
            &format!("Error reading {}: {}", resolved.display(), e),
            start.elapsed(),
        ),
    }
}

fn apply_edit(
    name: &str,
    args: &serde_json::Value,
    path: &std::path::Path,
    content: &str,
    search: &str,
    replace: &str,
    elapsed: std::time::Duration,
) -> ToolOutput {
    let count = content.matches(search).count();
    if count == 0 {
        return edit_error(
            name,
            args,
            &format!("search text not found in {}", path.display()),
            elapsed,
        );
    }
    if count > 1 {
        return edit_error(
            name,
            args,
            &format!("search text appears {} times. Be more specific.", count),
            elapsed,
        );
    }
    let new_content = content.replacen(search, replace, 1);
    write_edited_content(name, args, path, content, &new_content, elapsed)
}

fn write_edited_content(
    name: &str,
    args: &serde_json::Value,
    path: &std::path::Path,
    old_content: &str,
    new_content: &str,
    elapsed: std::time::Duration,
) -> ToolOutput {
    match std::fs::write(path, new_content) {
        Ok(()) => {
            let diff = Diff::generate(old_content, new_content);
            let diff_output = diff.to_unified_string();
            ToolOutput {
                tool_name: name.to_string(),
                tool_args: args.clone(),
                content: diff_output,
                bytes_transferred: Some(new_content.len() as u64),
                duration: elapsed,
                status: ToolStatus::Success,
            }
        }
        Err(e) => edit_error(
            name,
            args,
            &format!("Error writing {}: {}", path.display(), e),
            elapsed,
        ),
    }
}

fn edit_error(
    name: &str,
    args: &serde_json::Value,
    msg: &str,
    elapsed: std::time::Duration,
) -> ToolOutput {
    ToolOutput {
        tool_name: name.to_string(),
        tool_args: args.clone(),
        content: msg.to_string(),
        bytes_transferred: None,
        duration: elapsed,
        status: ToolStatus::Error,
    }
}
