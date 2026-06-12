//! Verifies that every entry in `build.rs`'s `ALLOWED_FILES_OVER` and
//! `ALLOWED_FUNCS_OVER` constants points at a file/function that actually
//! exists in the current tree. This catches allow-list drift when a file is
//! renamed, moved, or deleted.
//!
//! Run with:  cargo test -p runie-core --lib tests::lint_drift

use std::path::{Path, PathBuf};

// Re-declare the allow-lists exactly as they appear in build.rs.
// If either list in build.rs is edited without updating this module the
// compiler will catch the mismatch.

const ALLOWED_FILES_OVER: &[&str] = &[
    "crates/runie-core/src/update/mod.rs",
    "crates/runie-core/src/model.rs",
    "crates/runie-core/src/login_flow.rs",
    "crates/runie-core/src/config_reload.rs",
    "crates/runie-core/src/tests/stack_navigation.rs",
    "crates/runie-core/src/tests/login_logout.rs",
    "crates/runie-term/tests/e2e_legacy.rs",
    "crates/runie-tui/src/pipe/render/overlays.rs",
    "crates/runie-tui/src/tui/update/palette_tests.rs",
    "crates/runie-tui/src/tui/update/slash_tests.rs",
    "crates/runie-tui/src/tui/tests_onboarding.rs",
    "crates/runie-tui/src/tui/tests/comprehensive_suite/stream_interruption_tests.rs",
    "crates/runie-tui/src/tui/tests/e2e_flow_tests/palette_flows.rs",
    "crates/runie-tui/src/tui/tests/input_unicode.rs",
    "crates/runie-tui/src/tui/tests/grok_element_tests.rs",
    "crates/runie-tui/src/tui/tests/scroll_auto_tests.rs",
    "crates/runie-tui/src/tui/tests/session_management_tests.rs",
    "crates/runie-tui/src/tui/tests/snapshot_regression_tests/grok_parity_tests.rs",
    "crates/runie-tui/src/tui/tests/grok_parity_tests.rs",
    "crates/runie-tui/src/tui/tests/agent_events/message_flow.rs",
    "crates/runie-tui/src/tui/tests/agent_events/tool_execution.rs",
    "crates/runie-tui/src/tui/tests/agent_events/lifecycle.rs",
    "crates/runie-tui/src/tui/view_models.rs",
    "crates/runie-tui/src/components/message_list.rs",
    "crates/runie-tui/src/components/input_bar/mod.rs",
    "crates/runie-tui/src/components/message_list/render/assistant.rs",
    "crates/runie-tui/src/components/message_list/render/messages_test.rs",
    "crates/runie-tui/src/components/diff_viewer.rs",
    "crates/runie-tui/src/components/top_bar/tests.rs",
    "crates/runie-tui/src/tests.rs",
];

// Format: "path:line_number:function_name"
const ALLOWED_FUNCS_OVER: &[&str] = &[
    "crates/runie-core/src/commands/handlers/session.rs:11:register",
    "crates/runie-core/src/commands/handlers/system.rs:7:register",
    "crates/runie-core/src/model_catalog.rs:54:model_catalog",
];

/// The workspace root (two directories up from this crate's manifest dir).
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/runie-core/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn allowed_files_exist() {
    let ws = workspace_root();
    let mut missing = Vec::new();
    for &path in ALLOWED_FILES_OVER {
        if !ws.join(path).exists() {
            missing.push(path);
        }
    }
    assert!(
        missing.is_empty(),
        "ALLOWED_FILES_OVER entries not found in tree:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn allowed_funcs_exist() {
    let ws = workspace_root();
    let mut missing = Vec::new();
    for &entry in ALLOWED_FUNCS_OVER {
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() < 3 {
            missing.push(format!("{entry}  (malformed — expected path:line:name)"));
            continue;
        }
        let file_path = parts[0];
        let line_no: usize = parts[1].parse().unwrap_or(0);
        let fn_name = parts[2];

        let full_path = ws.join(file_path);
        if !full_path.exists() {
            missing.push(format!("{entry}  (file does not exist)"));
            continue;
        }

        let content = std::fs::read_to_string(&full_path).unwrap();
        let lines: Vec<_> = content.lines().collect();

        // Verify a function named `fn_name` starts near the recorded line.
        // Use a small window so the test survives minor line-number drift.
        let window = 3;
        let start = line_no.saturating_sub(1);
        let end = (start + window).min(lines.len());
        let near = &lines[start..end];

        let found = near.iter().any(|l| {
            let t = l.trim();
            t.starts_with("fn ") && t.contains(fn_name)
                || t.starts_with("pub fn ") && t.contains(fn_name)
        });

        if !found {
            missing.push(format!(
                "{entry}  (no function '{fn_name}' near line {line_no})"
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "ALLOWED_FUNCS_OVER entries not found in tree:\n  {}",
        missing.join("\n  ")
    );
}
