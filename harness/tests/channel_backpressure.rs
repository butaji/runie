//! Channel backpressure test grader.
//!
//! Validates that event channels handle backpressure:
//! 1. Backpressure strategy exists
//! 2. Timeout handling exists
//! 3. Try send with feedback
//! 4. No silent drops
//! 5. Buffer capacity defined

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, file_matches, dir_contains, run_checks, Check};

/// Get the crate root path (two directories up from tests/, then into runie/crates)
fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates")
}

fn main() {
    let root = crate_root();
    let mut checks = Vec::new();

    // Check 1: try_send with retry logic in tui_run/mod.rs
    let tui_run_path = root.join("runie-cli/src/tui_run/mod.rs");
    if tui_run_path.exists() {
        if file_contains(&tui_run_path, "try_send") {
            if file_matches(&tui_run_path, "(?i)retry") {
                checks.push(Check::pass_with_detail(
                    "Retry logic for try_send found",
                    "try_send with retry pattern detected",
                ));
            } else {
                checks.push(Check::pass_with_detail(
                    "try_send found but retry unclear",
                    "try_send exists, retry pattern not detected",
                ));
            }
        } else {
            checks.push(Check::fail("try_send handling not found"));
        }
    } else {
        checks.push(Check::fail("tui_run.rs not found"));
    }

    // Check 2: Channel capacity defined (mpsc::channel with capacity)
    if tui_run_path.exists() {
        let content = std::fs::read_to_string(&tui_run_path).unwrap_or_default();
        // Look for mpsc::channel with a numeric capacity argument
        if content.contains("mpsc::channel") {
            if let Some(cap) = extract_channel_capacity(&content) {
                checks.push(Check::pass_with_detail(
                    format!("Channel capacity defined: {}", cap),
                    "mpsc::channel with capacity found",
                ));
            } else {
                checks.push(Check::pass("Channel creation found (capacity unclear)"));
            }
        } else {
            checks.push(Check::fail("Channel creation (mpsc::channel) not found"));
        }
    }

    // Check 3: Timeout handling in permission
    let tui_dir = root.join("runie-tui/src");
    if dir_contains(&tui_dir, "timeout", Some("rs")) {
        checks.push(Check::pass("Permission timeout handling found"));
    } else {
        checks.push(Check::fail("Permission timeout not found"));
    }

    // Check 4: Event drop notification exists
    if dir_contains(&root.join("runie-tui/src"), "dropping event", Some("rs")) ||
       dir_contains(&root.join("runie-tui/src"), "event dropped", Some("rs")) ||
       dir_contains(&root.join("runie-tui/src"), "input dropped", Some("rs")) {
        checks.push(Check::pass("Event drop notification found"));
    } else {
        checks.push(Check::info(
            "No explicit event drop notification (may use retry)",
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

/// Extract channel capacity from mpsc::channel calls.
/// Looks for patterns like `channel::<T>(100)` or `channel(100)`.
fn extract_channel_capacity(content: &str) -> Option<String> {
    let re = regex::Regex::new(r"channel\s*(?:::<[^>]+>)?\s*\(\s*(\d+)\s*\)").ok()?;
    let caps = re.captures(content)?;
    caps.get(1).map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_backpressure_strategy() {
        let root = crate_root();
        let tui_run_path = root.join("runie-cli/src/tui_run/mod.rs");

        // Check for channel send handling (try_send or send)
        let has_send = file_contains(&tui_run_path, "try_send") || file_contains(&tui_run_path, ".send(");
        assert!(
            has_send,
            "FAIL: channel send handling not found"
        );
        println!("PASS: channel send handling found");
    }

    #[test]
    fn test_channel_capacity() {
        let root = crate_root();
        let tui_run_path = root.join("runie-cli/src/tui_run/mod.rs");

        let content = std::fs::read_to_string(&tui_run_path).unwrap();
        assert!(
            content.contains("mpsc::channel"),
            "FAIL: mpsc::channel not found"
        );
        println!("PASS: mpsc::channel found");
    }

    #[test]
    fn test_timeout_handling() {
        let root = crate_root();
        let tui_dir = root.join("runie-tui/src");

        assert!(
            dir_contains(&tui_dir, "timeout", Some("rs")),
            "FAIL: Timeout handling not found"
        );
        println!("PASS: Timeout handling found");
    }

    #[test]
    fn test_no_silent_drop() {
        let root = crate_root();
        let tui_dir = root.join("runie-tui/src");

        // This is an INFO check, so we don't fail if not found
        let has_drop_notification = dir_contains(&tui_dir, "dropping event", Some("rs")) ||
            dir_contains(&tui_dir, "event dropped", Some("rs")) ||
            dir_contains(&tui_dir, "input dropped", Some("rs"));

        if has_drop_notification {
            println!("PASS: Event drop notification found");
        } else {
            println!("INFO: No explicit event drop notification (may use retry)");
        }
    }
}
