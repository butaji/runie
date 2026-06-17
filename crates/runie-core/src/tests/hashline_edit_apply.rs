//! Tests for HashlineEditSkill applying edits.

use std::io::Write;

use runie_core::harness_skills::{
    HashlineEdit, HashlineEditConfig, HashlineEditSkill, HarnessSkill, ToolCallCtx, ToolCallPhase,
    ToolCallResult,
};

fn make_skill() -> HashlineEditSkill {
    HashlineEditSkill::new(HashlineEditConfig::default())
}

fn edit_ctx(path: &str, edits: Vec<HashlineEdit>) -> ToolCallCtx {
    ToolCallCtx {
        tool_name: "edit_file".into(),
        tool_input: serde_json::json!({
            "path": path,
            "edits": edits,
        }),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    }
}

#[test]
fn hashline_edit_applies_valid_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::File::create(&path)
        .unwrap()
        .write_all(b"line one\nline two\nline three\n")
        .unwrap();

    let edits = vec![HashlineEdit {
        line: 2,
        hash: HashlineEditSkill::compute_hash("line two", 6),
        content: "line two edited".into(),
    }];

    let result = make_skill().on_tool_call(&edit_ctx(path.to_str().unwrap(), edits));

    if let ToolCallResult::SkipWithOutput(output) = result {
        assert!(output.contains("line two edited"));
        assert!(output.contains("-line two"));
        assert!(output.contains("+line two edited"));
    } else {
        panic!("expected SkipWithOutput, got {:?}", result);
    }

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("line two edited"));
    assert!(!content.contains("line two\n"));
}

#[test]
fn hashline_edit_skips_legacy_path() {
    let result = make_skill().on_tool_call(&ToolCallCtx {
        tool_name: "edit_file".into(),
        tool_input: serde_json::json!({
            "path": "src/main.rs",
            "search": "old",
            "replace": "new",
        }),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    });

    assert!(matches!(result, ToolCallResult::Continue));
}

#[test]
fn hashline_edit_emits_tool_result_event() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::File::create(&path)
        .unwrap()
        .write_all(b"alpha\nbeta\n")
        .unwrap();

    let edits = vec![HashlineEdit {
        line: 1,
        hash: HashlineEditSkill::compute_hash("alpha", 6),
        content: "ALPHA".into(),
    }];

    let result = make_skill().on_tool_call(&edit_ctx(path.to_str().unwrap(), edits));

    match result {
        ToolCallResult::SkipWithOutput(output) => {
            assert!(output.starts_with("Applied hashline edits. Diff:"));
        }
        other => panic!("expected SkipWithOutput, got {:?}", other),
    }
}
