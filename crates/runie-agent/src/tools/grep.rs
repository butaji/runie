use super::apply_truncation;
use crate::path_utils::resolve_path;
use crate::truncate::TruncationPolicy;
use runie_core::tool::which_tool;

fn build_grep_args(
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--line-number".into(),
        "--color=never".into(),
        "--hidden".into(),
    ];
    if ignore_case {
        args.push("--ignore-case".into());
    }
    if literal {
        args.push("--fixed-strings".into());
    }
    if let Some(g) = glob {
        args.push("--glob".into());
        args.push(g.into());
    }
    if context > 0 {
        args.push("--context".into());
        args.push(context.to_string());
    }
    args.push("--max-count".into());
    args.push(limit.to_string());
    args.push("--".into());
    args.push(pattern.into());
    args.push(path.into());
    args
}

fn parse_grep_output(
    output: std::process::Output,
    limit: usize,
    policy: &TruncationPolicy,
) -> (String, bool) {
    let text = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    if text.trim().is_empty() {
        if output.status.code() == Some(1) {
            return ("No matches found".to_string(), true);
        }
        return (format!("Error: {}", err.trim()), false);
    }
    let mut result = text.to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() >= limit {
        result.push_str(&format!("\n\n[{} matches limit reached]", limit));
    }
    (
        apply_truncation(result, crate::accumulator::TruncateStrategy::Head, policy),
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_grep(
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
    policy: &TruncationPolicy,
) -> (String, bool) {
    let path = resolve_path(path);
    let args = build_grep_args(
        pattern,
        &path.to_string_lossy(),
        glob,
        ignore_case,
        literal,
        context,
        limit,
    );
    let tool = if which_tool("rg").is_some() {
        "rg"
    } else {
        "grep"
    };
    match std::process::Command::new(tool).args(&args).output() {
        Ok(output) => parse_grep_output(output, limit, policy),
        Err(e) => (format!("Error running grep: {}", e), false),
    }
}
