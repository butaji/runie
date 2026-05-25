//! Grader for empty_state task.
//!
//! Verifies that MessageList::render_ref handles empty state with:
//! 1. Empty check guard at start of render
//! 2. Greeting/guidance text
//! 3. Keyboard shortcut hints
//! 4. CTA to start conversation

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates")
}

fn main() {
    let root = crate_root();
    let msg_list_path = root.join("runie-tui/src/components/message_list/render.rs");
    let mut checks = Vec::new();

    if !msg_list_path.exists() {
        checks.push(Check::fail("message_list/render.rs not found"));
        let (_, total) = run_checks(checks);
        std::process::exit(if total > 0 { 0 } else { 1 });
    }

    let content = std::fs::read_to_string(&msg_list_path).unwrap_or_default();

    // Check 1: has_empty_check - is_empty() usage
    if file_contains(&msg_list_path, "is_empty()") {
        checks.push(Check::pass("has empty check guard"));
    } else {
        checks.push(Check::fail("is_empty() not found"));
    }

    // Check 2: has_greeting - welcome/start text
    let greeting_patterns = [
        "No messages",
        "Start typing",
        "Welcome",
        "hello",
        "Start a conversation",
        "empty_state",
        "Empty state",
        "No messages yet",
        "Type your first",
    ];
    let has_greeting = greeting_patterns
        .iter()
        .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
    if has_greeting {
        checks.push(Check::pass("has greeting/welcome text"));
    } else {
        checks.push(Check::fail("greeting text not found"));
    }

    // Check 3: has_shortcuts_hint - keyboard hints
    let hint_patterns = ["Enter", "shortcut", "hint", "^k", "^b", "scroll", "commands"];
    let has_hints = hint_patterns
        .iter()
        .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
    if has_hints {
        checks.push(Check::pass("has keyboard shortcut hints"));
    } else {
        checks.push(Check::fail("shortcut hints not found"));
    }

    // Check 4: has_cta - call to action
    let cta_patterns = [
        "Press Enter",
        "Start",
        "begin",
        "Type",
        "Send a message",
        "Send",
    ];
    let has_cta = cta_patterns
        .iter()
        .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
    if has_cta {
        checks.push(Check::pass("has call to action"));
    } else {
        checks.push(Check::fail("call to action not found"));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_empty_check() {
        let root = crate_root();
        let path = root.join("runie-tui/src/components/message_list/render.rs");
        assert!(
            file_contains(&path, "is_empty()"),
            "FAIL: is_empty() not found"
        );
        println!("PASS: is_empty() found");
    }

    #[test]
    fn test_has_greeting() {
        let root = crate_root();
        let path = root.join("runie-tui/src/components/message_list/render.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let greeting_patterns = [
            "No messages",
            "Start typing",
            "Welcome",
            "hello",
            "Start a conversation",
            "empty_state",
            "Empty state",
            "No messages yet",
            "Type your first",
        ];
        let has_greeting =
            greeting_patterns
                .iter()
                .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
        assert!(has_greeting, "FAIL: greeting text not found");
        println!("PASS: greeting text found");
    }

    #[test]
    fn test_has_shortcut_hints() {
        let root = crate_root();
        let path = root.join("runie-tui/src/components/message_list/render.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let hint_patterns = ["Enter", "shortcut", "hint", "^k", "^b", "scroll", "commands"];
        let has_hints =
            hint_patterns
                .iter()
                .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
        assert!(has_hints, "FAIL: shortcut hints not found");
        println!("PASS: shortcut hints found");
    }

    #[test]
    fn test_has_cta() {
        let root = crate_root();
        let path = root.join("runie-tui/src/components/message_list/render.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let cta_patterns = [
            "Press Enter",
            "Start",
            "begin",
            "Type",
            "Send a message",
            "Send",
        ];
        let has_cta = cta_patterns
            .iter()
            .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
        assert!(has_cta, "FAIL: call to action not found");
        println!("PASS: call to action found");
    }
}
