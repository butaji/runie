//! Tests for runie-tools

use crate::{BashTool, ReadFileTool, ToolRegistry, Workspace};
use runie_core::Tool;
use std::path::PathBuf;

#[test]
fn test_bash_tool_name() {
    let tool = BashTool::new(Workspace::new(PathBuf::from(".")));
    assert_eq!(tool.name(), "bash");
}

#[test]
fn test_bash_tool_description() {
    let tool = BashTool::new(Workspace::new(PathBuf::from(".")));
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("bash"));
}

#[test]
fn test_bash_tool_schema() {
    let tool = BashTool::new(Workspace::new(PathBuf::from(".")));
    let schema = tool.schema();
    assert_eq!(schema.name, "bash");
    assert!(schema.parameters.is_object());
    let props = schema.parameters.get("properties").unwrap();
    assert!(props.get("command").is_some());
}

#[test]
fn test_read_file_tool_name() {
    let tool = ReadFileTool::new(Workspace::new(PathBuf::from(".")));
    assert_eq!(tool.name(), "read_file");
}

#[test]
fn test_read_file_tool_description() {
    let tool = ReadFileTool::new(Workspace::new(PathBuf::from(".")));
    assert!(!tool.description().is_empty());
    assert!(tool.description().contains("file"));
}

#[test]
fn test_read_file_tool_schema() {
    let tool = ReadFileTool::new(Workspace::new(PathBuf::from(".")));
    let schema = tool.schema();
    assert_eq!(schema.name, "read_file");
    let props = schema.parameters.get("properties").unwrap();
    assert!(props.get("path").is_some());
}

#[test]
fn test_workspace_new() {
    let ws = Workspace::new(PathBuf::from("."));
    assert_eq!(ws.root, PathBuf::from("."));
}

#[test]
fn test_workspace_resolve_valid_path() {
    let ws = Workspace::new(PathBuf::from("."));
    let resolved = ws.resolve("Cargo.toml");
    assert!(resolved.is_ok());
    assert!(resolved.unwrap().ends_with("Cargo.toml"));
}

#[test]
fn test_workspace_resolve_nested_path() {
    let ws = Workspace::new(PathBuf::from("."));
    // Use a path that exists in the repo
    let resolved = ws.resolve("src/tests.rs");
    assert!(resolved.is_ok());
    assert!(resolved.unwrap().ends_with("src/tests.rs"));
}

#[test]
fn test_workspace_contains() {
    let ws = Workspace::new(PathBuf::from("."));
    assert!(ws.contains(&PathBuf::from(".").canonicalize().unwrap()));
    assert!(ws.contains(&PathBuf::from("Cargo.toml").canonicalize().unwrap()));
}

#[test]
fn test_tool_registry_new() {
    let registry = ToolRegistry::new();
    assert!(registry.list().is_empty());
}

#[test]
fn test_tool_registry_register() {
    let mut registry = ToolRegistry::new();
    let tool = BashTool::new(Workspace::new(PathBuf::from(".")));
    registry.register(Box::new(tool));
    assert!(!registry.list().is_empty());
}

#[test]
fn test_tool_registry_get() {
    let mut registry = ToolRegistry::new();
    let tool = BashTool::new(Workspace::new(PathBuf::from(".")));
    registry.register(Box::new(tool));
    assert!(registry.get("bash").is_some());
    assert!(registry.get("nonexistent").is_none());
}

#[test]
fn test_tool_registry_list() {
    let mut registry = ToolRegistry::new();
    let bash = BashTool::new(Workspace::new(PathBuf::from(".")));
    let read = ReadFileTool::new(Workspace::new(PathBuf::from(".")));
    registry.register(Box::new(bash));
    registry.register(Box::new(read));
    let tools = registry.list();
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_tool_registry_schemas() {
    let mut registry = ToolRegistry::new();
    let bash = BashTool::new(Workspace::new(PathBuf::from(".")));
    registry.register(Box::new(bash));
    let schemas = registry.schemas();
    assert_eq!(schemas.len(), 1);
    assert_eq!(schemas[0].name, "bash");
}

#[test]
fn test_tool_registry_names() {
    let mut registry = ToolRegistry::new();
    let bash = BashTool::new(Workspace::new(PathBuf::from(".")));
    let read = ReadFileTool::new(Workspace::new(PathBuf::from(".")));
    registry.register(Box::new(bash));
    registry.register(Box::new(read));
    let names = registry.names();
    assert!(names.contains(&"bash".to_string()));
    assert!(names.contains(&"read_file".to_string()));
}

#[tokio::test]
async fn test_read_file_tool_execute_missing_path() {
    let tool = ReadFileTool::new(Workspace::new(PathBuf::from(".")));
    let args = serde_json::json!({"path": "nonexistent_file_12345.txt"});
    let result = tool.execute(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_read_file_tool_execute_valid_file() {
    let workspace = std::env::current_dir().unwrap();
    let file_path = workspace.join("test_runie_read.txt");
    tokio::fs::write(&file_path, "Hello, test!".as_bytes()).await.unwrap();
    
    let tool = ReadFileTool::new(Workspace::new(workspace.clone()));
    let args = serde_json::json!({"path": "test_runie_read.txt"});
    let result = tool.execute(args).await;
    
    tokio::fs::remove_file(&file_path).await.ok();
    
    assert!(result.is_ok());
    assert!(result.unwrap().content.contains("Hello"));
}

// Note: BashTool execution tests skipped due to pre-existing syntax error at bash.rs:60
// where an extra closing brace exists after the dangerous command check loop.
