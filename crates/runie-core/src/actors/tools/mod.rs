//! Tool implementations

mod bash;
mod edit;
mod find;
mod grep;
mod read;
mod write;

pub use bash::{check_bash_safety, execute_bash, run_bash_with_timeout};
pub use edit::execute_edit_file;
pub use find::execute_find;
pub use grep::execute_grep;
pub use read::execute_read_file;
pub use write::execute_write_file;

use crate::actors::ToolOutput;

/// Tool invocation message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInvocation {
    pub id: String,
    pub name: String,
    pub params: serde_json::Value,
}

/// Execute a tool by name with parameters
pub fn execute_tool(invocation: &ToolInvocation) -> ToolOutput {
    match invocation.name.as_str() {
        "read_file" => execute_read_file(&invocation.params),
        "list_dir" => execute_list_dir(&invocation.params),
        "write_file" => execute_write_file(&invocation.params),
        "edit_file" => execute_edit_file(&invocation.params),
        "bash" => execute_bash(&invocation.params),
        "grep" => execute_grep(&invocation.params),
        "find" => execute_find(&invocation.params),
        _ => ToolOutput {
            success: false,
            output: format!("Unknown tool: {}", invocation.name),
        },
    }
}

fn execute_list_dir(params: &serde_json::Value) -> ToolOutput {
    let path = get_str(params, "path");
    match std::fs::read_dir(&path) {
        Ok(entries) => {
            let mut lines: Vec<String> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let typ = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    "dir"
                } else {
                    "file"
                };
                lines.push(format!("{} ({})", name, typ));
            }
            let output = if lines.is_empty() {
                "(empty directory)".to_string()
            } else {
                lines.join("\n")
            };
            ToolOutput { success: true, output }
        }
        Err(e) => ToolOutput {
            success: false,
            output: format!("Error listing {}: {}", path, e),
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
    fn tool_invocation_serialization() {
        let inv = ToolInvocation {
            id: "test-1".to_string(),
            name: "bash".to_string(),
            params: serde_json::json!({"command": "echo hello"}),
        };
        let json = serde_json::to_string(&inv).unwrap();
        let parsed: ToolInvocation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "bash");
    }

    #[test]
    fn bash_tool_echo() {
        let result = execute_tool(&ToolInvocation {
            id: "test".to_string(),
            name: "bash".to_string(),
            params: serde_json::json!({"command": "echo hello"}),
        });
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[test]
    fn bash_tool_blocked() {
        let result = execute_tool(&ToolInvocation {
            id: "test".to_string(),
            name: "bash".to_string(),
            params: serde_json::json!({"command": "rm -rf /"}),
        });
        assert!(!result.success);
        assert!(result.output.contains("Blocked"));
    }

    #[test]
    fn unknown_tool() {
        let result = execute_tool(&ToolInvocation {
            id: "test".to_string(),
            name: "unknown_tool".to_string(),
            params: serde_json::json!({}),
        });
        assert!(!result.success);
        assert!(result.output.contains("Unknown tool"));
    }
}
