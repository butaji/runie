//! Grep tool implementation

use std::process::Command;
use super::ToolOutput;

/// Execute grep tool
pub fn execute_grep(params: &serde_json::Value) -> ToolOutput {
    let pattern = get_str(params, "pattern");
    let path = get_str(params, "path");
    let ignore_case = get_bool(params, "ignore_case");
    let literal = get_bool(params, "literal");
    let context = get_usize(params, "context", 0);
    let limit = get_usize(params, "limit", 100);

    let tool = which_tool("rg").unwrap_or_else(|| "grep".to_string());
    let args = build_grep_args(&pattern, &path, ignore_case, literal, context, limit);

    match Command::new(&tool).args(&args).output() {
        Ok(output) => parse_grep_output(output, limit),
        Err(e) => ToolOutput {
            success: false,
            output: format!("Error running grep: {}", e),
        },
    }
}

fn build_grep_args(pattern: &str, path: &str, ignore_case: bool, literal: bool, context: usize, limit: usize) -> Vec<String> {
    let mut args = vec!["--line-number".to_string(), "--color=never".to_string(), "--hidden".to_string()];
    if ignore_case { args.push("--ignore-case".to_string()); }
    if literal { args.push("--fixed-strings".to_string()); }
    if context > 0 {
        args.push("--context".to_string());
        args.push(context.to_string());
    }
    args.push("--max-count".to_string());
    args.push(limit.to_string());
    args.push("--".to_string());
    args.push(pattern.to_string());
    args.push(path.to_string());
    args
}

fn parse_grep_output(output: std::process::Output, limit: usize) -> ToolOutput {
    let text = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    if text.trim().is_empty() {
        if output.status.code() == Some(1) {
            return ToolOutput { success: true, output: "No matches found".to_string() };
        }
        return ToolOutput {
            success: false,
            output: format!("Error: {}", err.trim()),
        };
    }
    let mut result = text.to_string();
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() >= limit {
        result.push_str(&format!("\n\n[{} matches limit reached]", limit));
    }
    ToolOutput { success: true, output: result }
}

fn which_tool(name: &str) -> Option<String> {
    Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn get_str(params: &serde_json::Value, key: &str) -> String {
    params.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn get_usize(params: &serde_json::Value, key: &str, default: usize) -> usize {
    params.get(key).and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(default)
}

fn get_bool(params: &serde_json::Value, key: &str) -> bool {
    params.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grep_finds_match() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_grep.txt");
        std::fs::write(&test_file, "line 1: hello\nline 2: world\nline 3: hello again").unwrap();

        let result = execute_grep(&serde_json::json!({
            "pattern": "hello",
            "path": test_file.to_string_lossy()
        }));
        assert!(result.success);
        assert!(result.output.contains("hello"));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn grep_no_match() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_grep2.txt");
        std::fs::write(&test_file, "line 1: hello\nline 2: world").unwrap();

        let result = execute_grep(&serde_json::json!({
            "pattern": "notfound",
            "path": test_file.to_string_lossy()
        }));
        assert!(result.success);
        assert!(result.output.contains("No matches"));

        let _ = std::fs::remove_file(test_file);
    }
}
