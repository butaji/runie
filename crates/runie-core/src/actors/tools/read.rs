//! Read file tool implementation

use super::ToolOutput;

/// Execute read_file tool
pub fn execute_read_file(params: &serde_json::Value) -> ToolOutput {
    let path = get_str(params, "path");
    let offset = params.get("offset").and_then(|v| v.as_u64()).map(|v| v as usize);
    let limit = params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

    match std::fs::read_to_string(&path) {
        Ok(content) => read_file_content(&path, &content, offset, limit),
        Err(e) => ToolOutput {
            success: false,
            output: format!("Error reading {}: {}", path, e),
        },
    }
}

fn read_file_content(path: &str, content: &str, offset: Option<usize>, limit: Option<usize>) -> ToolOutput {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let start = offset.unwrap_or(0).min(total_lines);
    let end = limit.map(|l| (start + l).min(total_lines)).unwrap_or(total_lines);

    if start >= total_lines {
        return ToolOutput { success: true, output: "(end of file)".to_string() };
    }

    let selected: String = lines[start..end].join("\n");
    let output = if offset.is_some() || limit.is_some() {
        format!("[Lines {}-{} of {}]\n{}", start + 1, end, total_lines, selected)
    } else {
        selected
    };

    let output = if end < total_lines {
        format!("{}\n[{} more lines]", output, total_lines - end)
    } else {
        output
    };

    ToolOutput { success: true, output }
}

fn get_str(params: &serde_json::Value, key: &str) -> String {
    params.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_tool() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_read.txt");
        std::fs::write(&test_file, "line 1\nline 2\nline 3\n").unwrap();

        let result = execute_read_file(&serde_json::json!({"path": test_file.to_string_lossy()}));
        assert!(result.success);
        assert!(result.output.contains("line 1"));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn read_file_with_offset_limit() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_offset.txt");
        std::fs::write(&test_file, "line 1\nline 2\nline 3\nline 4\n").unwrap();

        let result = execute_read_file(&serde_json::json!({
            "path": test_file.to_string_lossy(),
            "offset": 1,
            "limit": 2
        }));
        assert!(result.success);
        assert!(result.output.contains("line 2"));
        assert!(result.output.contains("line 3"));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn read_nonexistent_file() {
        let result = execute_read_file(&serde_json::json!({"path": "/nonexistent/file.txt"}));
        assert!(!result.success);
        assert!(result.output.contains("Error"));
    }
}
