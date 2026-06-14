use crate::tools::read_file::read_file;
use crate::tools::{run_find, run_grep, Tool};
use crate::truncate::TruncationPolicy;

pub(crate) fn run_inner(tool: &Tool, policy: &TruncationPolicy) -> (String, bool) {
    match tool {
        Tool::ReadFile { .. } => run_read_file_tool(tool, policy),
        Tool::ListDir { path } => super::list_dir(path, policy),
        Tool::WriteFile { path, content } => write_file(path, content),
        Tool::EditFile {
            path,
            search,
            replace,
        } => super::edit_file(path, search, replace),
        Tool::Bash { command } => super::run_bash(command, policy),
        Tool::Grep { .. } => run_grep_tool(tool, policy),
        Tool::Find { .. } => run_find_tool(tool, policy),
        Tool::FetchDocs { library } => super::run_fetch_docs(library),
    }
}

fn run_read_file_tool(tool: &Tool, policy: &TruncationPolicy) -> (String, bool) {
    if let Tool::ReadFile {
        path,
        offset,
        limit,
    } = tool
    {
        read_file(path, *offset, *limit, policy)
    } else {
        unreachable!()
    }
}

fn run_grep_tool(tool: &Tool, policy: &TruncationPolicy) -> (String, bool) {
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
        run_grep(
            pattern,
            path,
            glob.as_deref(),
            *ignore_case,
            *literal,
            *context,
            *limit,
            policy,
        )
    } else {
        unreachable!()
    }
}

fn run_find_tool(tool: &Tool, policy: &TruncationPolicy) -> (String, bool) {
    if let Tool::Find {
        pattern,
        path,
        limit,
    } = tool
    {
        run_find(pattern, path, *limit, policy)
    } else {
        unreachable!()
    }
}

fn write_file(path: &str, content: &str) -> (String, bool) {
    let path = crate::path_utils::resolve_path(path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return (format!("Error creating parent directories: {}", e), false);
            }
        }
    }
    match std::fs::write(&path, content) {
        Ok(()) => (
            format!("Wrote {} bytes to {}", content.len(), path.display()),
            true,
        ),
        Err(e) => (format!("Error writing {}: {}", path.display(), e), false),
    }
}
