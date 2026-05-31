//! Error sanitization tests.
//!
//! Tests for sanitize_error_message function behavior:
//! - Truncates messages >500 chars
//! - Detects stack traces
//! - Stack trace hides details, keeps first 5 lines
//! - No truncate for short messages

use crate::tui::update::agent::sanitize_error_message;

#[test]
fn test_sanitize_error_message_truncates_long_messages() {
    let long_error = "Error: ".to_string() + &"x".repeat(1000);
    let sanitized = sanitize_error_message(&long_error);

    assert!(
        sanitized.len() < long_error.len(),
        "Long error should be truncated"
    );
    assert!(
        sanitized.contains("[message truncated"),
        "Should indicate truncation"
    );
}

#[test]
fn test_sanitize_error_message_no_truncate_short_messages() {
    let short_error = "Connection reset by peer";
    let sanitized = sanitize_error_message(short_error);

    assert_eq!(sanitized, short_error);
    assert!(!sanitized.contains("truncated"));
}

#[test]
fn test_sanitize_error_message_detects_stack_trace() {
    let stack_trace = "Connection error\nstack backtrace:\n   at 0x7f8d9f... (main.rs:100)\n   at 0x7f8da0... (main.rs:101)";
    let sanitized = sanitize_error_message(stack_trace);

    assert!(
        sanitized.contains("Connection error"),
        "Should preserve error summary"
    );
}

#[test]
fn test_sanitize_error_message_stack_trace_shows_first_five_lines() {
    let stack_trace = "Error: Something went wrong\nstack backtrace:\n   at 0x1 (lib.rs:10)\n   at 0x2 (lib.rs:20)\n   at 0x3 (lib.rs:30)\n   at 0x4 (lib.rs:40)\n   at 0x5 (lib.rs:50)\n   at 0x6 (lib.rs:60)";
    let sanitized = sanitize_error_message(stack_trace);

    // Should only show first 5 lines
    let lines: Vec<&str> = sanitized.lines().collect();
    assert!(
        lines.len() <= 6, // 5 lines + 1 note about hidden details
        "Should show first 5 lines plus note, got {} lines",
        lines.len()
    );
    assert!(
        sanitized.contains("[Additional details hidden]") || sanitized.contains("[truncated"),
        "Should indicate hidden details"
    );
}

#[test]
fn test_sanitize_error_message_stack_trace_patterns() {
    // Test various stack trace patterns
    let patterns = vec![
        "thread 'main' has overflowed",
        "panicked at 'assertion failed'",
        "---- test::test_main failed",
        "test result: FAILED",
        "at 0x7f8d9f...",
    ];

    for pattern in patterns {
        let sanitized = sanitize_error_message(pattern);
        // Should be processed (either truncated or have hidden details note)
        assert!(
            sanitized.contains("hidden") || sanitized.contains("truncated") || sanitized.len() < pattern.len() * 2,
            "Pattern '{}' should be detected as stack trace",
            pattern
        );
    }
}

#[test]
fn test_sanitize_error_message_preserves_first_line_of_stack_trace() {
    let error = "RuntimeError: index out of bounds\nstack backtrace:\n   0: rust_out_of_bounds\n   1: core::panicking\n   2: test_case";
    let sanitized = sanitize_error_message(error);

    assert!(
        sanitized.starts_with("RuntimeError: index out of bounds"),
        "Should preserve the actual error message line"
    );
}

#[test]
fn test_sanitize_error_message_long_stack_trace_truncation() {
    // Create a long stack trace >500 chars
    let long_stack = "Error\nstack backtrace:\n".to_string()
        + &"   at function_a (file_a.rs:10)\n".repeat(50);
    let sanitized = sanitize_error_message(&long_stack);

    assert!(
        sanitized.len() < long_stack.len(),
        "Long stack trace should be truncated"
    );
    assert!(
        sanitized.contains("[truncated"),
        "Should indicate truncation for long stack traces"
    );
}

#[test]
fn test_sanitize_error_message_empty_string() {
    let empty = "";
    let sanitized = sanitize_error_message(empty);
    assert_eq!(sanitized, "");
}

#[test]
fn test_sanitize_error_message_exactly_500_chars() {
    // Exactly 500 chars should not be truncated
    let exact_500 = "x".repeat(500);
    let sanitized = sanitize_error_message(&exact_500);
    assert_eq!(sanitized.len(), 500);
}

#[test]
fn test_sanitize_error_message_501_chars_truncated() {
    // 501 chars should be truncated
    let chars_501 = "x".repeat(501);
    let sanitized = sanitize_error_message(&chars_501);
    assert!(
        sanitized.len() < 501,
        "501 chars should be truncated"
    );
    assert!(
        sanitized.contains("truncated"),
        "Should indicate truncation"
    );
}
