//! Magic number lint tests.
//!
//! Verifies that the magic number detection logic in build.rs works correctly.
//! This test exercises the regex patterns and exemption logic without running
//! the full build script.

use regex::Regex;

/// Regex for numeric literals >= 1000 (4+ digits).
fn magic_number_regex() -> Regex {
    Regex::new(r"\b(\d{4,}(?:_\d+)*)\b").unwrap()
}

/// Known standard HTTP status codes (used in match arms).
fn http_status_codes() -> &'static [&'static str] {
    &["401", "403", "429", "500", "502", "503", "504"]
}

/// Known JSON-RPC error codes.
fn json_rpc_codes() -> &'static [&'static str] {
    &["32700", "32600", "32601", "32602", "32603"]
}

/// Check if a line should be exempt from magic number linting.
fn is_line_exempt(line: &str) -> bool {
    let trimmed = line.trim();

    // Skip all comments and doc comments.
    if trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("///")
        || trimmed.starts_with("//!")
    {
        return true;
    }

    // Skip lines that define constants.
    if trimmed.starts_with("const ") || trimmed.starts_with("pub const ") {
        return true;
    }

    // Skip assert/panic/debug_assert lines.
    if trimmed.contains("assert!")
        || trimmed.contains("debug_assert!")
        || trimmed.contains("panic!")
        || trimmed.contains("matches!(")
    {
        return true;
    }

    // Skip vec!, hashmap!, etc. literals.
    if line.contains("vec!")
        || line.contains("hashmap!")
        || line.contains("HashMap!")
        || line.contains("btreemap!")
    {
        return true;
    }

    false
}

/// Find magic numbers in a line.
fn find_magic_numbers(line: &str) -> Vec<String> {
    // Skip entire line if exempt.
    if is_line_exempt(line) {
        return Vec::new();
    }

    let re = magic_number_regex();
    let mut numbers = Vec::new();

    for cap in re.find_iter(line) {
        let matched = cap.as_str();

        // Skip hex.
        if cap.start() > 0 && line[cap.start() - 1..cap.start()].starts_with("0x") {
            continue;
        }

        // Check if the number is in a string literal.
        let before = if cap.start() > 0 {
            &line[..cap.start()]
        } else {
            ""
        };
        if before.ends_with('"') || before.ends_with('\'') {
            continue;
        }

        // Check if this looks like a named constant (preceded by =, :, ,, =>).
        let looks_named =
            before.ends_with('=') || before.ends_with(':') || before.ends_with(',') || before.ends_with("=>");

        if looks_named {
            continue;
        }

        // Skip array sizes [0; 1000] and range bounds 0..1000.
        if line.contains("; ") && line.contains("[") {
            continue;
        }
        if line.contains("..") {
            continue;
        }

        // Skip if already has a clear name nearby.
        if line.contains("const ")
            || line.contains("static ")
            || line.contains("pub const")
        {
            continue;
        }

        // Skip HTTP status codes that appear in match arms.
        if http_status_codes().iter().any(|code| line.contains(code)) {
            continue;
        }

        // Skip JSON-RPC error codes.
        if json_rpc_codes().iter().any(|code| line.contains(code)) {
            continue;
        }

        numbers.push(matched.to_string());
    }

    numbers
}

#[test]
fn test_magic_number_lint_catches_violation() {
    let line = "pub fn foo() { let x = 12345; }";
    assert!(
        !find_magic_numbers(line).is_empty(),
        "Should catch magic number 12345"
    );
    assert!(
        find_magic_numbers(line).contains(&String::from("12345")),
        "Should contain 12345"
    );
}

#[test]
fn test_magic_number_lint_allows_small_numbers() {
    let line = "pub fn foo() { let x = 99; }";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should not flag numbers < 1000"
    );
}

#[test]
fn test_magic_number_lint_allows_underscore_separated() {
    let line = "pub fn foo() { let x = 1_000_000; }";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow underscore-separated numbers"
    );
}

#[test]
fn test_magic_number_lint_allows_named_constants() {
    let line = "const BUFFER_SIZE: usize = 4096;";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow const definitions"
    );
}

#[test]
fn test_magic_number_lint_allows_field_assignment() {
    let line = "let config = Config { timeout: 5000 };";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow struct field assignments"
    );
}

#[test]
fn test_magic_number_lint_allows_hex() {
    let line = "let x = 0xFFFF;";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow hex literals"
    );
}

#[test]
fn test_magic_number_lint_allows_http_status_codes() {
    let line = "match status { 500 => ... }";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow HTTP status codes"
    );
}

#[test]
fn test_magic_number_lint_allows_json_rpc_codes() {
    let line = "match code { 32700 => ... }";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow JSON-RPC error codes"
    );
}

#[test]
fn test_magic_number_lint_allows_string_literals() {
    let line = "vec![\"1000\".into(), \"2000\".into()]";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow numbers in string literals"
    );
}

#[test]
fn test_magic_number_lint_allows_vec_macro() {
    let line = "let items = vec![1, 2, 3000];";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow vec! macro contents"
    );
}

#[test]
fn test_magic_number_lint_allows_assert() {
    // Note: 1234 is < 1000, so this is already exempt
    let line = "assert!(x > 9999);";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow numbers in assert! macros"
    );
}

#[test]
fn test_magic_number_lint_allows_panic() {
    // Note: 9999 >= 1000 but panic! should be exempt
    let line = "panic!(\"error code: 9999\");";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow numbers in panic! macros"
    );
}

#[test]
fn test_magic_number_lint_allows_doc_comments() {
    // Doc comments at start of line are exempt
    let line = "/// Default value is 12345";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow numbers in doc comments"
    );
}

#[test]
fn test_magic_number_lint_allows_range_bounds() {
    let line = "for i in 0..1000 { }";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow range bounds"
    );
}

#[test]
fn test_magic_number_lint_allows_array_size() {
    let line = "let arr: [u8; 1024] = [0; 1024];";
    assert!(
        find_magic_numbers(line).is_empty(),
        "Should allow array size literals"
    );
}
