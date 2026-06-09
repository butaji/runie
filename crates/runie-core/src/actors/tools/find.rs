//! Find tool implementation

use std::process::Command;
use super::ToolOutput;

/// Execute find tool
pub fn execute_find(params: &serde_json::Value) -> ToolOutput {
    let pattern = get_str(params, "pattern");
    let path = get_str(params, "path");
    let limit = get_usize(params, "limit", 100);

    let tool = if which_tool("fd").is_some() { "fd" } else { "find" };

    let output = if tool == "fd" {
        let args = &[
            "--glob", "--color=never", "--hidden", "--no-require-git",
            "--max-results", &limit.to_string(),
            "--", &pattern, &path,
        ];
        Command::new("fd").args(args).output()
    } else {
        let args = &[&path, "-maxdepth", "10", "-path", &format!("*/{}", pattern)];
        Command::new("find").args(args).output()
    };

    match output {
        Ok(output) => parse_find_output(output, limit),
        Err(e) => ToolOutput {
            success: false,
            output: format!("Error running find: {}", e),
        },
    }
}

fn parse_find_output(output: std::process::Output, limit: usize) -> ToolOutput {
    let text = String::from_utf8_lossy(&output.stdout);
    if text.trim().is_empty() {
        return ToolOutput {
            success: true,
            output: "No files found matching pattern".to_string(),
        };
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out = lines[..limit.min(lines.len())].join("\n");
    if lines.len() > limit {
        out.push_str(&format!("\n\n[{} results limit reached]", limit));
    }
    ToolOutput { success: true, output: out }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_finds_files() {
        let temp_dir = std::env::temp_dir().join("runie_test_find");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("test.txt"), "").unwrap();
        std::fs::write(temp_dir.join("other.txt"), "").unwrap();

        let result = execute_find(&serde_json::json!({
            "pattern": "test*",
            "path": temp_dir.to_string_lossy()
        }));
        assert!(result.success);
        assert!(result.output.contains("test.txt"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn find_no_matches() {
        let temp_dir = std::env::temp_dir().join("runie_test_find2");
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::write(temp_dir.join("hello.txt"), "").unwrap();

        let result = execute_find(&serde_json::json!({
            "pattern": "notfound*",
            "path": temp_dir.to_string_lossy()
        }));
        assert!(result.success);
        assert!(result.output.contains("No files found"));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
