use super::apply_truncation;
use crate::path_utils::resolve_path;
use crate::truncate::TruncationPolicy;
use runie_core::tool::which_tool;

fn build_fd_args(pattern: &str, path: &str, limit: usize) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--glob".into(),
        "--color=never".into(),
        "--hidden".into(),
        "--no-require-git".into(),
    ];
    if pattern.contains("/") {
        args.push("--full-path".into());
    }
    args.push("--max-results".into());
    args.push(limit.to_string());
    args.push("--".into());
    args.push(pattern.into());
    args.push(path.into());
    args
}

fn build_find_args(pattern: &str, path: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![path.into(), "-maxdepth".into(), "10".into()];
    if pattern.contains("*") || pattern.contains("?") {
        args.push("-name".into());
        args.push(pattern.into());
    } else {
        args.push("-path".into());
        args.push(format!("*/{}", pattern));
    }
    args
}

fn parse_find_output(
    output: std::process::Output,
    limit: usize,
    policy: &TruncationPolicy,
) -> (String, bool) {
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return ("No files found matching pattern".to_string(), true);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out = lines[..limit.min(lines.len())].join("\n");
    if lines.len() > limit {
        out.push_str(&format!("\n\n[{} results limit reached]", limit));
    }
    (
        apply_truncation(out, crate::accumulator::TruncateStrategy::Head, policy),
        true,
    )
}

pub(crate) fn run_find(
    pattern: &str,
    path: &str,
    limit: usize,
    policy: &TruncationPolicy,
) -> (String, bool) {
    let path = resolve_path(path);
    let tool = if which_tool("fd").is_some() {
        "fd"
    } else {
        "find"
    };
    let path_str = path.to_string_lossy();
    let result = if tool == "fd" {
        std::process::Command::new("fd")
            .args(build_fd_args(pattern, &path_str, limit))
            .output()
    } else {
        std::process::Command::new("find")
            .args(build_find_args(pattern, &path_str))
            .output()
    };

    match result {
        Ok(output) => parse_find_output(output, limit, policy),
        Err(e) => (format!("Error running find: {}", e), false),
    }
}
