use crate::tools::read_file::read_file;
use crate::tools::{run_find, run_grep, Tool};
use crate::truncate::TruncationPolicy;
use runie_core::tool::{ToolOutput, ToolStatus};
use std::time::Instant;

pub(crate) fn run_inner(tool: &Tool, policy: &TruncationPolicy) -> ToolOutput {
    let start = Instant::now();
    let name = tool.name();
    let args = tool.to_args();
    match tool {
        Tool::ReadFile { .. } => run_read_file_tool(tool, policy, &name, &args, start),
        Tool::ListDir { .. } => super::list_dir(tool, policy),
        Tool::WriteFile { .. } => write_file(tool, start),
        Tool::EditFile { .. } => super::edit_file(tool, policy),
        Tool::Bash { .. } => super::bash::run_bash_legacy(tool, policy),
        Tool::Grep { .. } => run_grep_tool(tool, policy, &name, &args, start),
        Tool::Find { .. } => run_find_tool(tool, policy, &name, &args, start),
        Tool::FetchDocs { .. } => super::run_fetch_docs(tool, start),
    }
}

fn run_read_file_tool(tool: &Tool, policy: &TruncationPolicy, name: &str, args: &serde_json::Value, start: Instant) -> ToolOutput {
    if let Tool::ReadFile {
        path,
        offset,
        limit,
    } = tool
    {
        let (content, success) = read_file(path, *offset, *limit, policy);
        ToolOutput {
            tool_name: name.to_string(),
            tool_args: args.clone(),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: if success { ToolStatus::Success } else { ToolStatus::Error },
        }
    } else {
        unreachable!()
    }
}

fn run_grep_tool(tool: &Tool, policy: &TruncationPolicy, name: &str, args: &serde_json::Value, start: Instant) -> ToolOutput {
    if let Tool::Grep {
        pattern,
        path,
        glob,
        ignore_case,
        literal,
        context,
        limit,
    } = tool
    {
        let (content, success) = run_grep(
            pattern,
            path,
            glob.as_deref(),
            *ignore_case,
            *literal,
            *context,
            *limit,
            policy,
        );
        ToolOutput {
            tool_name: name.to_string(),
            tool_args: args.clone(),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: if success { ToolStatus::Success } else { ToolStatus::Error },
        }
    } else {
        unreachable!()
    }
}

fn run_find_tool(tool: &Tool, policy: &TruncationPolicy, name: &str, args: &serde_json::Value, start: Instant) -> ToolOutput {
    if let Tool::Find {
        pattern,
        path,
        limit,
    } = tool
    {
        let (content, success) = run_find(pattern, path, *limit, policy);
        ToolOutput {
            tool_name: name.to_string(),
            tool_args: args.clone(),
            content,
            bytes_transferred: None,
            duration: start.elapsed(),
            status: if success { ToolStatus::Success } else { ToolStatus::Error },
        }
    } else {
        unreachable!()
    }
}

fn write_file(tool: &Tool, start: Instant) -> ToolOutput {
    let (path, content) = if let Tool::WriteFile { path, content } = tool {
        (path, content)
    } else {
        unreachable!()
    };
    let path = crate::path_utils::resolve_path(path);
    let name = tool.name();
    let args = tool.to_args();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolOutput {
                    tool_name: name.to_string(),
                    tool_args: args,
                    content: format!("Error creating parent directories: {}", e),
                    bytes_transferred: None,
                    duration: start.elapsed(),
                    status: ToolStatus::Error,
                };
            }
        }
    }
    match std::fs::write(&path, content) {
        Ok(()) => ToolOutput {
            tool_name: name.to_string(),
            tool_args: args,
            content: format!("Wrote {} bytes to {}", content.len(), path.display()),
            bytes_transferred: Some(content.len() as u64),
            duration: start.elapsed(),
            status: ToolStatus::Success,
        },
        Err(e) => ToolOutput {
            tool_name: name.to_string(),
            tool_args: args,
            content: format!("Error writing {}: {}", path.display(), e),
            bytes_transferred: None,
            duration: start.elapsed(),
            status: ToolStatus::Error,
        },
    }
}
