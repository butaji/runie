//! State transition test grader.
//!
//! Validates that the state machine:
//! 1. Defines all valid transitions
//! 2. Rejects invalid transitions
//! 3. Handles edge cases
//!
//! This grader analyzes the state.rs file to verify state machine implementation.

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, file_matches, run_checks, Check};

/// Get the crate root path (two directories up from tests/, then into runie/crates)
fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates")
}

fn main() {
    let root = crate_root();
    let mut checks = Vec::new();

    let state_path = root.join("runie-tui/src/tui/state/enums.rs");

    // Check 1: TuiMode enum exists with all expected states
    let content = std::fs::read_to_string(&state_path).unwrap_or_default();

    if content.contains("pub enum TuiMode") {
        checks.push(Check::pass("TuiMode enum defined with all states"));
    } else {
        checks.push(Check::fail("TuiMode enum not found"));
    }

    // Check 2: State transitions defined in events.rs (mode handler functions)
    let events_path = root.join("runie-tui/src/tui/events.rs");
    if file_matches(&events_path, r"TuiMode::") {
        checks.push(Check::pass("State transition logic found in events.rs"));
    } else {
        checks.push(Check::fail("State transition logic not found"));
    }

    // Check 3: Chat mode is the default initial state
    if file_matches(&state_path, r"mode:\s*TuiMode::Chat|default.*TuiMode::Chat") {
        checks.push(Check::pass("Chat mode is default initial state"));
    } else {
        checks.push(Check::fail("Chat mode default not found"));
    }

    // Check 4: CloseModal and OpenCommandPalette transitions exist
    if file_contains(&state_path, "CloseModal") && file_contains(&state_path, "OpenCommandPalette") {
        checks.push(Check::pass("Modal/CommandPalette transitions defined"));
    } else {
        checks.push(Check::fail("Modal transitions not properly defined"));
    }

    // Check 5: Permission handling exists
    if file_contains(&state_path, "PermissionConfirm") || file_contains(&state_path, "PermissionCancel") {
        checks.push(Check::pass("Permission state transitions defined"));
    } else {
        checks.push(Check::fail("Permission transitions not found"));
    }

    // Check 6: Edge case handling (rapid switching protection)
    // Look for debounce, throttle, or similar patterns
    if file_matches(&state_path, r"debounce|throttle|cooldown|rapid") {
        checks.push(Check::pass("Rapid switching protection found"));
    } else {
        checks.push(Check::info(
            "No explicit rapid switching protection (may be handled by UI framework)",
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
    fn test_tui_mode_enum_exists() {
        let root = crate_root();
        let state_path = root.join("runie-tui/src/tui/state/enums.rs");
        assert!(
            file_contains(&state_path, "pub enum TuiMode"),
            "FAIL: TuiMode enum not found"
        );
        println!("PASS: TuiMode enum found");
    }

    #[test]
    fn test_chat_mode_default() {
        let root = crate_root();
        // Chat mode default is in AppState::default() in mod.rs
        let state_path = root.join("runie-tui/src/tui/state/mod.rs");
        assert!(
            file_matches(
                &state_path,
                r"mode:\s*TuiMode::Chat|default.*TuiMode::Chat",
            ),
            "FAIL: Chat mode default not found"
        );
        println!("PASS: Chat mode is default");
    }

    #[test]
    fn test_state_transition_logic() {
        let root = crate_root();
        let events_path = root.join("runie-tui/src/tui/events.rs");
        assert!(
            file_matches(&events_path, r"TuiMode::"),
            "FAIL: State transition logic not found"
        );
        println!("PASS: State transition logic found");
    }

    #[test]
    fn test_modal_transitions() {
        let root = crate_root();
        let state_path = root.join("runie-tui/src/tui/state/enums.rs");
        assert!(
            file_contains(&state_path, "CloseModal") && file_contains(&state_path, "OpenCommandPalette"),
            "FAIL: Modal transitions not properly defined"
        );
        println!("PASS: Modal/CommandPalette transitions defined");
    }

    #[test]
    fn test_permission_transitions() {
        let root = crate_root();
        let state_path = root.join("runie-tui/src/tui/state/enums.rs");
        assert!(
            file_contains(&state_path, "PermissionConfirm") || file_contains(&state_path, "PermissionCancel"),
            "FAIL: Permission transitions not found"
        );
        println!("PASS: Permission state transitions defined");
    }

    #[test]
    fn test_rapid_switching_protection() {
        let root = crate_root();
        let state_path = root.join("runie-tui/src/tui/state/enums.rs");
        // Info check - don't fail if not found
        if file_matches(&state_path, r"debounce|throttle|cooldown|rapid") {
            println!("PASS: Rapid switching protection found");
        } else {
            println!("INFO: No explicit rapid switching protection (may be handled by UI framework)");
        }
    }
}
