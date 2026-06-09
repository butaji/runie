//! Edit file tool implementation

use super::ToolOutput;

/// Truncate text to max length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

/// Execute edit_file tool
pub fn execute_edit_file(params: &serde_json::Value) -> ToolOutput {
    let path = get_str(params, "path");
    let search = get_str(params, "search");
    let replace = get_str(params, "replace");

    if search.is_empty() {
        return ToolOutput {
            success: false,
            output: "Error: search text cannot be empty".to_string(),
        };
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => apply_edit(&path, &content, &search, &replace),
        Err(e) => ToolOutput {
            success: false,
            output: format!("Error reading {}: {}", path, e),
        },
    }
}

fn apply_edit(path: &str, content: &str, search: &str, replace: &str) -> ToolOutput {
    let count = content.matches(search).count();
    if count == 0 {
        return ToolOutput {
            success: false,
            output: format!("Error: search text not found in {}", path),
        };
    }
    if count > 1 {
        return ToolOutput {
            success: false,
            output: format!(
                "Error: search text appears {} times in {}. Be more specific.",
                count, path
            ),
        };
    }
    let new_content = content.replacen(search, replace, 1);
    match std::fs::write(path, &new_content) {
        Ok(()) => {
            let output = format!(
                "Edited {} successfully ({} -> \"{}\")",
                path,
                truncate_text(search, 30),
                truncate_text(replace, 30)
            );
            ToolOutput { success: true, output }
        }
        Err(e) => ToolOutput {
            success: false,
            output: format!("Error writing {}: {}", path, e),
        },
    }
}

fn get_str(params: &serde_json::Value, key: &str) -> String {
    params.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_file_tool() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_edit.txt");
        std::fs::write(&test_file, "hello world").unwrap();

        let result = execute_edit_file(&serde_json::json!({
            "path": test_file.to_string_lossy(),
            "search": "world",
            "replace": "rust"
        }));
        assert!(result.success);

        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "hello rust");

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn edit_empty_search_rejected() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_edit2.txt");
        std::fs::write(&test_file, "hello").unwrap();

        let result = execute_edit_file(&serde_json::json!({
            "path": test_file.to_string_lossy(),
            "search": "",
            "replace": "world"
        }));
        assert!(!result.success);

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn edit_nonexistent_search_rejected() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_edit3.txt");
        std::fs::write(&test_file, "hello").unwrap();

        let result = execute_edit_file(&serde_json::json!({
            "path": test_file.to_string_lossy(),
            "search": "notfound",
            "replace": "world"
        }));
        assert!(!result.success);

        let _ = std::fs::remove_file(test_file);
    }
}
