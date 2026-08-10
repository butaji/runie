use super::{BashTool, EditFileTool, GlobTool, GrepTool, ReadFileTool, WriteFileTool};
use crate::types::{AgentTool, ToolResultContent};
#[tokio::test]
async fn read_returns_requested_line_range() {
    let path = std::env::temp_dir().join(format!("runie-read-{}.txt", std::process::id()));
    tokio::fs::write(&path, "one\ntwo\nthree\n").await.unwrap();
    let result = ReadFileTool
        .execute(
            "read-1",
            serde_json::json!({"path": path, "line_offset": 2, "n_lines": 1}),
            None,
            None,
        )
        .await
        .unwrap();
    let ToolResultContent::Text { text } = &result.content[0] else {
        panic!("expected text")
    };
    assert_eq!(text, "two\n[output truncated]");
    assert_eq!(result.details["line_offset"], 2);
    assert_eq!(result.details["line_count"], 1);
    tokio::fs::remove_file(path).await.unwrap();
}

#[tokio::test]
async fn read_rejects_missing_path() {
    let error = ReadFileTool
        .execute("read-1", serde_json::json!({}), None, None)
        .await
        .unwrap_err();
    assert!(error.contains("requires"));
}

#[tokio::test]
async fn write_and_edit_preserve_exact_match_safety() {
    let path = std::path::PathBuf::from(format!("runie-edit-{}.txt", std::process::id()));
    WriteFileTool
        .execute(
            "w",
            serde_json::json!({"path": path, "content": "old\nold"}),
            None,
            None,
        )
        .await
        .unwrap();
    let error = EditFileTool
        .execute(
            "e",
            serde_json::json!({"path": path, "old_string": "old", "new_string": "new"}),
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(error.contains("matched 2"));
    EditFileTool.execute("e", serde_json::json!({"path": path, "old_string": "old", "new_string": "new", "replace_all": true}), None, None).await.unwrap();
    assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "new\nnew");
    tokio::fs::remove_file(path).await.unwrap();
}

#[tokio::test]
async fn grep_and_glob_return_deterministic_matches() {
    let root = std::env::temp_dir().join(format!("runie-search-{}", std::process::id()));
    tokio::fs::create_dir_all(&root).await.unwrap();
    tokio::fs::write(root.join("a.rs"), "needle\n")
        .await
        .unwrap();
    tokio::fs::write(root.join("b.txt"), "other\n")
        .await
        .unwrap();
    let grep = GrepTool
        .execute(
            "g",
            serde_json::json!({"path": root, "pattern": "needle"}),
            None,
            None,
        )
        .await
        .unwrap();
    let ToolResultContent::Text { text } = &grep.content[0] else {
        panic!("expected text")
    };
    assert!(text.ends_with(":1:needle"));
    assert_eq!(grep.details["match_count"], 1);
    let glob = GlobTool
        .execute(
            "f",
            serde_json::json!({"path": root, "pattern": "*.rs"}),
            None,
            None,
        )
        .await
        .unwrap();
    let ToolResultContent::Text { text } = &glob.content[0] else {
        panic!("expected text")
    };
    assert!(text.ends_with("a.rs"));
    assert_eq!(glob.details["match_count"], 1);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn bash_returns_output_and_surfaces_failures() {
    let updates = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let updates_seen = updates.clone();
    let result = BashTool
        .execute(
            "b",
            serde_json::json!({"command": "printf one; printf two >&2"}),
            None,
            Some(Box::new(move |value| {
                updates_seen.lock().unwrap().push(value)
            })),
        )
        .await
        .unwrap();
    let ToolResultContent::Text { text } = &result.content[0] else {
        panic!("expected text")
    };
    assert_eq!(text, "onetwo");
    assert_eq!(result.details["stdout"], "one");
    assert_eq!(result.details["stderr"], "two");
    assert_eq!(result.details["exit_code"], 0);
    assert!(updates
        .lock()
        .unwrap()
        .iter()
        .any(|value| value["complete"] == false));
    assert!(updates
        .lock()
        .unwrap()
        .iter()
        .any(|value| value["complete"] == true));
    let error = BashTool
        .execute("b", serde_json::json!({"command": "exit 3"}), None, None)
        .await
        .unwrap_err();
    assert!(error.contains("exited"));
}
