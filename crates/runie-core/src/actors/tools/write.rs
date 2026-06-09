//! Write file tool implementation

use std::path::Path;
use super::ToolOutput;

/// Execute write_file tool
pub fn execute_write_file(params: &serde_json::Value) -> ToolOutput {
    let path = get_str(params, "path");
    let content = get_str(params, "content");

    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolOutput {
                    success: false,
                    output: format!("Error creating parent directories: {}", e),
                };
            }
        }
    }

    match std::fs::write(&path, &content) {
        Ok(()) => ToolOutput {
            success: true,
            output: format!("Wrote {} bytes to {}", content.len(), path),
        },
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
    fn write_file_tool() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("runie_test_write.txt");

        let result = execute_write_file(&serde_json::json!({
            "path": test_file.to_string_lossy(),
            "content": "hello world"
        }));
        assert!(result.success);

        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "hello world");

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn write_creates_parent_dirs() {
        let temp_dir = std::env::temp_dir().join("runie_test_nested");
        let test_file = temp_dir.join("subdir/file.txt");

        let result = execute_write_file(&serde_json::json!({
            "path": test_file.to_string_lossy(),
            "content": "nested content"
        }));
        assert!(result.success);

        let content = std::fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "nested content");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
