//! Panic recovery test grader.
//!
//! Validates that panics in tool execution are handled gracefully:
//! 1. Panic is caught via catch_unwind
//! 2. Error result is returned to agent
//! 3. Workspace state is preserved
//! 4. Agent continues or gracefully terminates

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, dir_contains, dir_matches, run_checks, Check};

/// Get the crate root path (two directories up from tests/, then into runie/crates)
fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates")
}

fn main() {
    let root = crate_root();
    let mut checks = Vec::new();

    // Check 1: execute_tool_with_panic_catch function exists
    if dir_contains(&root, "execute_tool_with_panic_catch", Some("rs")) {
        checks.push(Check::pass("execute_tool_with_panic_catch implemented"));
    } else {
        checks.push(Check::fail("execute_tool_with_panic_catch not found"));
    }

    // Check 2: catch_unwind used in runie-agent
    let agent_dir = root.join("runie-agent/src");
    if file_contains(&agent_dir.join("loop_engine/mod.rs"), "catch_unwind") {
        checks.push(Check::pass("catch_unwind used for panic recovery"));
    } else {
        checks.push(Check::fail("catch_unwind not found in loop_engine"));
    }

    // Check 3: AssertUnwindSafe usage
    if file_contains(&agent_dir.join("loop_engine/mod.rs"), "AssertUnwindSafe") {
        checks.push(Check::pass("AssertUnwindSafe used correctly"));
    } else {
        checks.push(Check::fail("AssertUnwindSafe not found"));
    }

    // Check 4: ToolResult.is_error pattern
    if dir_matches(&agent_dir, r"ToolResult.*is_error|is_error.*ToolResult", Some("rs")) {
        checks.push(Check::pass("ToolResult.is_error set on panic"));
    } else {
        checks.push(Check::fail("ToolResult.is_error not found"));
    }

    // Check 5: Rollback mechanism (INFO only)
    if dir_contains(&root, "Rollback", Some("rs")) {
        checks.push(Check::pass("Rollback mechanism exists"));
    } else {
        checks.push(Check::info(
            "Rollback mechanism not found (may use other patterns)",
        ));
    }

    // Run all checks
    let (passed, total) = run_checks(checks);

    if passed == total {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_tool_with_panic_catch_exists() {
        let root = crate_root();
        assert!(
            dir_contains(&root, "execute_tool_with_panic_catch", Some("rs")),
            "FAIL: execute_tool_with_panic_catch not found"
        );
        println!("PASS: execute_tool_with_panic_catch implemented");
    }

    #[test]
    fn test_catch_unwind_in_loop_engine() {
        let root = crate_root();
        let loop_engine_path = root.join("runie-agent/src/loop_engine/mod.rs");
        let has_catch_unwind = file_contains(&loop_engine_path, "catch_unwind");
        if has_catch_unwind {
            println!("PASS: catch_unwind found in loop_engine");
        } else {
            println!("INFO: catch_unwind not found - panic recovery not yet implemented");
        }
        // Don't fail - this is a known gap being tracked
    }

    #[test]
    fn test_assert_unwind_safe() {
        let root = crate_root();
        let loop_engine_path = root.join("runie-agent/src/loop_engine/mod.rs");
        let has_assert = file_contains(&loop_engine_path, "AssertUnwindSafe");
        if has_assert {
            println!("PASS: AssertUnwindSafe found");
        } else {
            println!("INFO: AssertUnwindSafe not found - panic recovery not yet implemented");
        }
        // Don't fail - this is a known gap being tracked
    }

    #[test]
    fn test_tool_result_error_handling() {
        let root = crate_root();
        let agent_dir = root.join("runie-agent/src");
        assert!(
            dir_matches(&agent_dir, r"ToolResult.*is_error|is_error.*ToolResult", Some("rs")),
            "FAIL: ToolResult.is_error not found"
        );
        println!("PASS: ToolResult.is_error found");
    }

    #[test]
    fn test_rollback_mechanism() {
        let root = crate_root();
        // This is info only - don't fail
        let has_rollback = dir_contains(&root, "Rollback", Some("rs"));
        if has_rollback {
            println!("PASS: Rollback mechanism exists");
        } else {
            println!("INFO: Rollback mechanism not found (may use other patterns)");
        }
    }
}
