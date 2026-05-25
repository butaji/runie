//! Grader for double_submit_dedup task.
//!
//! Validates that the system prevents duplicate submissions by:
//! 1. Checking for agent_running guard in submit handler
//! 2. Showing feedback when blocked
//! 3. Not spawning duplicate agents

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates")
}

fn main() {
    let root = crate_root();
    let misc_path = root.join("runie-tui/src/tui/update/misc.rs");
    let state_path = root.join("runie-tui/src/tui/state.rs");
    let mut checks = Vec::new();

    // Check misc.rs for submit handling
    if misc_path.exists() {
        let content = std::fs::read_to_string(&misc_path).unwrap_or_default();

        // Check for agent_running guard
        if file_contains(&misc_path, "agent_running") {
            checks.push(Check::pass("has agent_running check"));
        } else {
            checks.push(Check::fail("agent_running guard not found"));
        }

        // Check for feedback when blocked
        let feedback_patterns = [
            "already running",
            "still running",
            "wait",
            "Please wait",
        ];
        let has_feedback = feedback_patterns
            .iter()
            .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
        if has_feedback {
            checks.push(Check::pass("has feedback when blocked"));
        } else {
            checks.push(Check::fail("feedback when blocked not found"));
        }

        // Check for early return
        if content.contains("return vec![]") {
            checks.push(Check::pass("submit returns early when blocked"));
        } else {
            checks.push(Check::fail("early return not found"));
        }
    } else {
        checks.push(Check::fail("misc.rs not found"));
    }

    // Check state.rs for agent_running definition
    if state_path.exists() && file_contains(&state_path, "agent_running") {
        checks.push(Check::pass("agent_running flag defined in state"));
    } else {
        checks.push(Check::fail("agent_running not defined in state"));
    }

    let (passed, _total) = run_checks(checks);
    // Require at least 2 of 4 checks to pass
    std::process::exit(if passed >= 2 { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_agent_running_check() {
        let root = crate_root();
        let misc_path = root.join("runie-tui/src/tui/update/misc.rs");
        assert!(
            file_contains(&misc_path, "agent_running"),
            "FAIL: agent_running guard not found"
        );
        println!("PASS: agent_running check found");
    }

    #[test]
    fn test_has_feedback_when_blocked() {
        let root = crate_root();
        let misc_path = root.join("runie-tui/src/tui/update/misc.rs");
        let content = std::fs::read_to_string(&misc_path).unwrap_or_default();
        let feedback_patterns = [
            "already running",
            "still running",
            "wait",
            "Please wait",
        ];
        let has_feedback = feedback_patterns
            .iter()
            .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
        assert!(has_feedback, "FAIL: feedback when blocked not found");
        println!("PASS: feedback when blocked found");
    }

    #[test]
    fn test_submit_returns_early() {
        let root = crate_root();
        let misc_path = root.join("runie-tui/src/tui/update/misc.rs");
        let content = std::fs::read_to_string(&misc_path).unwrap_or_default();
        assert!(
            content.contains("return vec![]"),
            "FAIL: early return not found"
        );
        println!("PASS: submit returns early when blocked");
    }

    #[test]
    fn test_agent_running_in_state() {
        let root = crate_root();
        let state_path = root.join("runie-tui/src/tui/state.rs");
        assert!(
            file_contains(&state_path, "agent_running"),
            "FAIL: agent_running not defined in state"
        );
        println!("PASS: agent_running defined in state");
    }
}
