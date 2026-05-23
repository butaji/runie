use async_trait::async_trait;
use runie_core::{Tool, ToolSchema, ToolOutput, ToolError};
use serde_json::json;
use crate::Workspace;
use walkdir::WalkDir;

pub struct SearchTool {
    workspace: Workspace,
}

impl SearchTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Search for files by name or content pattern in the workspace."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (glob for files, substring for content)"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["filename", "content"],
                        "description": "Search type"
                    }
                },
                "required": ["pattern", "type"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let pattern = args["pattern"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'pattern' argument".to_string()))?;
        let search_type = args["type"].as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'type' argument".to_string()))?;
        
        let mut results = Vec::new();
        
        for entry in WalkDir::new(&self.workspace.root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !self.workspace.contains(path) {
                continue;
            }
            
            match search_type {
                "filename" => {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.contains(pattern) {
                            results.push(path.strip_prefix(&self.workspace.root).unwrap_or(path).display().to_string());
                        }
                    }
                }
                "content" => {
                    if path.is_file() && path.metadata().map(|m| m.len() < 1_000_000).unwrap_or(false) {
                        if let Ok(content) = tokio::fs::read_to_string(path).await {
                            if content.contains(pattern) {
                                results.push(path.strip_prefix(&self.workspace.root).unwrap_or(path).display().to_string());
                            }
                        }
                    }
                }
                _ => return Err(ToolError::InvalidArguments(format!("Invalid search type: {}", search_type))),
            }
        }
        
        results.truncate(50); // Limit results
        
        Ok(ToolOutput {
            content: results.join("\n"),
            metadata: json!({"count": results.len(), "pattern": pattern}),
            terminate: false,
        })
    }
}
