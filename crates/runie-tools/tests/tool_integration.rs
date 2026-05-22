use runie_core::Tool;
use runie_tools::{BashTool, ReadFileTool, WriteFileTool, Workspace};
use std::path::PathBuf;

#[tokio::test]
async fn test_bash_tool_echo() {
    let ws = Workspace::new(PathBuf::from("."));
    let tool = BashTool::new(ws);
    let args = serde_json::json!({"command": "echo hello"});
    let result = tool.execute(args).await.unwrap();
    assert!(result.content.contains("hello"));
}

#[tokio::test]
async fn test_bash_tool_with_timeout() {
    let ws = Workspace::new(PathBuf::from("."));
    let tool = BashTool::new(ws);
    let args = serde_json::json!({
        "command": "sleep 0.1 && echo done",
        "timeout": 5
    });
    let result = tool.execute(args).await.unwrap();
    assert!(result.content.contains("done"));
}

#[tokio::test]
async fn test_bash_tool_multi_command() {
    let ws = Workspace::new(PathBuf::from("."));
    let tool = BashTool::new(ws);
    let args = serde_json::json!({
        "command": "echo line1 && echo line2 && echo line3"
    });
    let result = tool.execute(args).await.unwrap();
    assert!(result.content.contains("line1"));
    assert!(result.content.contains("line2"));
    assert!(result.content.contains("line3"));
}

#[tokio::test]
async fn test_bash_tool_exit_code() {
    let ws = Workspace::new(PathBuf::from("."));
    let tool = BashTool::new(ws);
    let args = serde_json::json!({"command": "exit 0"});
    let result = tool.execute(args).await.unwrap();
    // Exit code should be in metadata
    let exit_code = result.metadata.get("exit_code").and_then(|v| v.as_i64());
    assert_eq!(exit_code, Some(0));
}

#[tokio::test]
async fn test_bash_tool_stderr_capture() {
    let ws = Workspace::new(PathBuf::from("."));
    let tool = BashTool::new(ws);
    let args = serde_json::json!({"command": "echo error >&2"});
    let result = tool.execute(args).await.unwrap();
    assert!(result.content.contains("[stderr]"));
    assert!(result.content.contains("error"));
}
