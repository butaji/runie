//! Tools — Built-in coding tools inspired by pi
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  ToolRegistry                                                    │
//! │  - tool_definitions: HashMap<name, ToolDef>                     │
//! │  - execute(name, input) -> ToolOutput                          │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Tool (trait)                                                    │
//! │  - name: &str                                                   │
//! │  - input_schema: Schema                                         │
//! │  - execute(input) -> Output                                     │
//! │  - render(call, result) -> String                              │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Built-in Tools                                                  │
//! │  - ReadTool: Read file contents                                │
//! │  - BashTool: Execute shell commands                             │
//! │  - EditTool: Apply edits to files (diff-based)                   │
//! │  - WriteTool: Write entire file contents                        │
//! │  - GrepTool: Search file contents                               │
//! │  - FindTool: Find files by pattern                              │
//! │  - LsTool: List directory contents                              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool input (JSON-serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    #[serde(flatten)]
    pub args: HashMap<String, serde_json::Value>,
}

impl ToolInput {
    pub fn new() -> Self {
        Self { args: HashMap::new() }
    }

    pub fn with_arg(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.args.insert(key.into(), v);
        }
        self
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.args.get(key).cloned()
    }

    pub fn get_str(&self, key: &str) -> Option<String> {
        self.args.get(key)
            .and_then(|v| v.as_str().map(String::from))
    }

    pub fn get_opt_str(&self, key: &str) -> Option<Option<String>> {
        self.args.get(key).map(|v| v.as_str().map(String::from))
    }

    pub fn get_usize(&self, key: &str) -> Option<usize> {
        self.args.get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    }
}

impl Default for ToolInput {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ToolOutput {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
            truncated: None,
            details: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(msg.to_string()),
            truncated: None,
            details: None,
        }
    }

    pub fn truncated(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            error: None,
            truncated: Some(true),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Tool definition (schema + metadata)
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub read_only: bool,
}

impl ToolDef {
    pub fn new(name: impl Into<String>, description: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: schema,
            read_only: false,
        }
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Tool trait — implement this to create a tool
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name
    fn name(&self) -> &str;

    /// Tool description for LLM
    fn description(&self) -> &str;

    /// JSON Schema for input
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the tool
    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput;

    /// Render a tool call (for TUI display)
    fn render_call(&self, input: &ToolInput) -> String {
        let args: Vec<String> = input.args.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        format!("{}({})", self.name(), args.join(", "))
    }

    /// Render a tool result (for TUI display)
    fn render_result(&self, result: &ToolOutput) -> String {
        if result.success {
            if result.content.len() > 1000 {
                format!("{}...\n[truncated]", &result.content[..1000])
            } else {
                result.content.clone()
            }
        } else {
            format!("Error: {}", result.error.as_deref().unwrap_or("Unknown error"))
        }
    }

    /// Register this tool with a registry
    fn register(&self, _registry: &ToolRegistry) {
        // Registration handled by registry
    }

    /// Clone as boxed trait object
    fn clone_box(&self) -> Box<dyn Tool>;
}

impl Clone for Box<dyn Tool> {
    fn clone(&self) -> Box<dyn Tool> {
        self.clone_box()
    }
}

/// Tool registry
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, name: String, tool: Box<dyn Tool>) {
        self.tools.write().await.insert(name, tool);
    }

    pub async fn get(&self, name: &str) -> Option<Box<dyn Tool>> {
        self.tools.read().await.get(name).map(|t| t.clone_box())
    }

    pub async fn list(&self) -> Vec<ToolDef> {
        let tools = self.tools.read().await;
        tools.values()
            .map(|t| ToolDef::new(t.name(), t.description(), t.input_schema()))
            .collect()
    }

    pub async fn execute(&self, name: &str, input: ToolInput, cwd: &PathBuf) -> Result<ToolOutput, String> {
        let tools = self.tools.read().await;
        let tool = tools
            .get(name)
            .ok_or_else(|| format!("Tool not found: {}", name))?;
        
        Ok(tool.execute(input, cwd).await)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Read Tool
// ============================================================================

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read (relative or absolute)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (1-indexed)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read"
                }
            },
            "required": ["path"]
        })
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str { "read" }
    fn description(&self) -> &str { "Read the contents of a file" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let path = match input.get_str("path") {
            Some(p) => p,
            None => return ToolOutput::error("Missing required argument: path"),
        };

        let resolved = if PathBuf::from(&path).is_absolute() {
            PathBuf::from(&path)
        } else {
            cwd.join(&path)
        };

        match tokio::fs::read_to_string(&resolved).await {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total_lines = lines.len();

                let offset = input.get_usize("offset").unwrap_or(1).saturating_sub(1);
                let limit = input.get_usize("limit").unwrap_or(usize::MAX);

                let slice: String = lines.into_iter()
                    .skip(offset)
                    .take(limit)
                    .enumerate()
                    .map(|(i, l)| format!("{:4}  {}", offset + i + 1, l))
                    .collect::<Vec<_>>()
                    .join("\n");

                let truncated = total_lines > offset + limit;
                let mut result = ToolOutput::success(slice);
                result.truncated = Some(truncated);
                result.details = Some(serde_json::json!({
                    "total_lines": total_lines,
                    "offset": offset + 1,
                    "limit": limit,
                    "truncated": truncated
                }));
                result
            }
            Err(e) => ToolOutput::error(&format!("Failed to read {}: {}", path, e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

// ============================================================================
// Bash Tool
// ============================================================================

pub struct BashTool {
    max_output_chars: usize,
}

impl BashTool {
    pub fn new() -> Self {
        Self { max_output_chars: 100_000 }
    }

    pub fn with_max_output(mut self, max: usize) -> Self {
        self.max_output_chars = max;
        self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 60)"
                }
            },
            "required": ["command"]
        })
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }
    fn description(&self) -> &str { "Execute a shell command" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let command = match input.get_str("command") {
            Some(c) => c,
            None => return ToolOutput::error("Missing required argument: command"),
        };

        let timeout_secs = input.get_usize("timeout").unwrap_or(60);

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(cwd)
            .output()
            .await;

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let truncated = stdout.len() > self.max_output_chars;
                let final_stdout = if truncated {
                    format!("{}\n\n[Output truncated at {} chars]", 
                        &stdout[..self.max_output_chars], 
                        self.max_output_chars)
                } else {
                    stdout.to_string()
                };

                let mut result = if output.status.success() {
                    ToolOutput::success(&final_stdout)
                } else {
                    ToolOutput::success(format!("{}\n\n[Exit code: {}]", final_stdout, 
                        output.status.code().unwrap_or(-1)))
                };
                result.truncated = Some(truncated);
                
                if !stderr.is_empty() {
                    result.content.push_str("\n\n[Stderr]:\n");
                    result.content.push_str(&stderr);
                }

                result
            }
            Err(e) => ToolOutput::error(&format!("Failed to execute: {}", e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

// ============================================================================
// Write Tool
// ============================================================================

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to write to (relative or absolute)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            },
            "required": ["path", "content"]
        })
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str { "write" }
    fn description(&self) -> &str { "Write content to a file (overwrites existing)" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let path = match input.get_str("path") {
            Some(p) => p,
            None => return ToolOutput::error("Missing required argument: path"),
        };

        let content = match input.get_str("content") {
            Some(c) => c,
            None => return ToolOutput::error("Missing required argument: content"),
        };

        let resolved = if PathBuf::from(&path).is_absolute() {
            PathBuf::from(&path)
        } else {
            cwd.join(&path)
        };

        // Create parent directories
        if let Some(parent) = resolved.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return ToolOutput::error(&format!("Failed to create directories: {}", e));
            }
        }

        match tokio::fs::write(&resolved, &content).await {
            Ok(_) => ToolOutput::success(format!("Written {} bytes to {}", content.len(), path)),
            Err(e) => ToolOutput::error(&format!("Failed to write {}: {}", path, e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

// ============================================================================
// Edit Tool
// ============================================================================

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "Text to replace (exact match)"
                },
                "new_text": {
                    "type": "string",
                    "description": "Replacement text"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str { "edit" }
    fn description(&self) -> &str { "Replace exact text in a file" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let path = match input.get_str("path") {
            Some(p) => p,
            None => return ToolOutput::error("Missing required argument: path"),
        };

        let old_text = match input.get_str("old_text") {
            Some(t) => t,
            None => return ToolOutput::error("Missing required argument: old_text"),
        };

        let new_text = match input.get_str("new_text") {
            Some(t) => t,
            None => return ToolOutput::error("Missing required argument: new_text"),
        };

        let resolved = if PathBuf::from(&path).is_absolute() {
            PathBuf::from(&path)
        } else {
            cwd.join(&path)
        };

        let content = match tokio::fs::read_to_string(&resolved).await {
            Ok(c) => c,
            Err(e) => return ToolOutput::error(&format!("Failed to read {}: {}", path, e)),
        };

        if !content.contains(&old_text) {
            return ToolOutput::error(&format!(
                "Text not found in {}. Make sure the exact text exists in the file.", 
                path
            ));
        }

        let new_content = content.replace(&old_text, &new_text);

        if new_content == content {
            return ToolOutput::error("Edit resulted in no change");
        }

        match tokio::fs::write(&resolved, &new_content).await {
            Ok(_) => {
                let lines_added = new_text.lines().count();
                let lines_removed = old_text.lines().count();
                ToolOutput::success(format!(
                    "Edited {}: {} lines removed, {} lines added", 
                    path, lines_removed, lines_added
                ))
            }
            Err(e) => ToolOutput::error(&format!("Failed to write {}: {}", path, e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

// ============================================================================
// Grep Tool
// ============================================================================

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Path to search in (file or directory)"
                },
                "context": {
                    "type": "integer",
                    "description": "Number of context lines (default: 0)"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Case sensitive search (default: true)"
                }
            },
            "required": ["pattern", "path"]
        })
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }
    fn description(&self) -> &str { "Search for pattern in files" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let pattern = match input.get_str("pattern") {
            Some(p) => p,
            None => return ToolOutput::error("Missing required argument: pattern"),
        };

        let path = match input.get_str("path") {
            Some(p) => p,
            None => return ToolOutput::error("Missing required argument: path"),
        };

        let context = input.get_usize("context").unwrap_or(0);
        let case_insensitive = input.get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let resolved = if PathBuf::from(&path).is_absolute() {
            PathBuf::from(&path)
        } else {
            cwd.join(&path)
        };

        let mut cmd = tokio::process::Command::new("grep");
        cmd.arg("-n");

        if case_insensitive {
            cmd.arg("-i");
        }

        if context > 0 {
            cmd.arg("-C").arg(context.to_string());
        }

        cmd.arg(&pattern).arg(resolved.to_str().unwrap_or(""));

        match cmd.output().await {
            Ok(output) => {
                if output.status.success() {
                    let result = String::from_utf8_lossy(&output.stdout);
                    ToolOutput::success(result.to_string())
                } else if output.status.code() == Some(1) {
                    // No matches
                    ToolOutput::success("No matches found".to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    ToolOutput::error(&format!("Grep failed: {}", stderr))
                }
            }
            Err(e) => ToolOutput::error(&format!("Failed to execute grep: {}", e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

// ============================================================================
// Find Tool
// ============================================================================

pub struct FindTool;

impl FindTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern to match (e.g., *.rs)"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: current)"
                },
                "type": {
                    "type": "string",
                    "description": "File type: 'f' for files, 'd' for directories"
                }
            },
            "required": ["pattern"]
        })
    }
}

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &str { "find" }
    fn description(&self) -> &str { "Find files matching a pattern" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let pattern = match input.get_str("pattern") {
            Some(p) => p,
            None => return ToolOutput::error("Missing required argument: pattern"),
        };

        let path = input.get_str("path")
            .map(|p| {
                if PathBuf::from(&p).is_absolute() {
                    PathBuf::from(&p)
                } else {
                    cwd.join(&p)
                }
            })
            .unwrap_or_else(|| cwd.clone());

        let file_type = input.get_str("type");

        let mut cmd = tokio::process::Command::new("find");
        cmd.arg(path.to_str().unwrap_or("."));

        if let Some(ref ft) = file_type {
            cmd.arg("-type").arg(ft);
        }

        cmd.arg("-name").arg(&pattern);

        match cmd.output().await {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                let files: Vec<&str> = result.lines().filter(|l| !l.is_empty()).collect();
                
                if files.is_empty() {
                    ToolOutput::success("No files found".to_string())
                } else {
                    ToolOutput::success(format!("Found {} files:\n{}", 
                        files.len(), 
                        files.join("\n")
                    ))
                }
            }
            Err(e) => ToolOutput::error(&format!("Failed to execute find: {}", e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

// ============================================================================
// Ls Tool
// ============================================================================

pub struct LsTool;

impl LsTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to list (default: current)"
                },
                "all": {
                    "type": "boolean",
                    "description": "Show hidden files (default: false)"
                },
                "long": {
                    "type": "boolean",
                    "description": "Long format with details (default: false)"
                }
            },
            "required": []
        })
    }
}

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str { "ls" }
    fn description(&self) -> &str { "List directory contents" }
    fn input_schema(&self) -> serde_json::Value { Self::input_schema() }

    async fn execute(&self, input: ToolInput, cwd: &PathBuf) -> ToolOutput {
        let path = input.get_str("path")
            .map(|p| {
                if PathBuf::from(&p).is_absolute() {
                    PathBuf::from(&p)
                } else {
                    cwd.join(&p)
                }
            })
            .unwrap_or_else(|| cwd.clone());

        let show_all = input.get("all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let long_format = input.get("long")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut cmd = tokio::process::Command::new("ls");
        
        if long_format {
            cmd.arg(if long_format && show_all { "-la" } else if long_format { "-l" } else { "-la" });
        } else if show_all {
            cmd.arg("-a");
        }

        cmd.arg(path.to_str().unwrap_or("."));

        match cmd.output().await {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                ToolOutput::success(result.to_string())
            }
            Err(e) => ToolOutput::error(&format!("Failed to execute ls: {}", e)),
        }
    }

    fn clone_box(&self) -> Box<dyn Tool> {
        Box::new(Self::new())
    }
}

/// Register all built-in tools
pub fn register_builtin_tools(registry: &ToolRegistry) {
    // Note: In async context, would need to register differently
    // For now, tools are registered via Tool::register() method
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_tool() {
        let tool = ReadTool::new();
        let cwd = PathBuf::from("/tmp");
        
        // Create a test file
        tokio::fs::write("/tmp/test_read.txt", "line 1\nline 2\nline 3").await.unwrap();
        
        let input = ToolInput::new()
            .with_arg("path", "/tmp/test_read.txt");
        
        let result = tool.execute(input, &cwd).await;
        assert!(result.success);
        assert!(result.content.contains("line 1"));
        
        tokio::fs::remove_file("/tmp/test_read.txt").await.ok();
    }

    #[tokio::test]
    async fn test_bash_tool() {
        let tool = BashTool::new();
        let cwd = PathBuf::from("/tmp");
        
        let input = ToolInput::new()
            .with_arg("command", "echo 'hello world'");
        
        let result = tool.execute(input, &cwd).await;
        assert!(result.success);
        assert!(result.content.contains("hello world"));
    }

    #[tokio::test]
    async fn test_write_tool() {
        let tool = WriteTool::new();
        let cwd = PathBuf::from("/tmp");
        
        let input = ToolInput::new()
            .with_arg("path", "/tmp/test_write.txt")
            .with_arg("content", "test content");
        
        let result = tool.execute(input, &cwd).await;
        assert!(result.success);
        
        let content = tokio::fs::read_to_string("/tmp/test_write.txt").await.unwrap();
        assert_eq!(content, "test content");
        
        tokio::fs::remove_file("/tmp/test_write.txt").await.ok();
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();
        
        registry.register("read".to_string(), Box::new(ReadTool::new())).await;
        
        let tools = registry.list().await;
        assert!(tools.iter().any(|t| t.name == "read"));
    }
}
