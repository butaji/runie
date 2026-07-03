#![allow(clippy::all)]
use crate::file_refs::{
    extract_lines, find_files, is_image_file, parse_file_ref, read_file_ref,
    read_file_ref_with_range,
};

#[test]
fn find_files_finds_rust_sources() {
    let files = find_files("*.rs", ".", 10);
    assert!(!files.is_empty());
    assert!(files.iter().all(|f| f.ends_with(".rs")));
}

#[test]
fn find_files_respects_limit() {
    let files = find_files("*.rs", ".", 3);
    assert!(files.len() <= 3);
}

#[test]
fn find_files_empty_pattern() {
    let files = find_files("", ".", 10);
    assert!(!files.is_empty());
}

#[test]
fn find_files_substring_match() {
    let files = find_files("toml", ".", 10);
    assert!(!files.is_empty());
    assert!(
        files
            .iter()
            .any(|f| f.contains("toml") || f.contains("TOML")),
        "Should find files matching substring 'toml'. Got: {:?}",
        files
    );
}

#[test]
fn is_image_file_detects_png() {
    assert!(is_image_file("photo.png"));
    assert!(is_image_file("image.jpg"));
    assert!(is_image_file("pic.jpeg"));
    assert!(is_image_file("anim.gif"));
    assert!(is_image_file("icon.webp"));
}

#[test]
fn is_image_file_rejects_text() {
    assert!(!is_image_file("main.rs"));
    assert!(!is_image_file("README.md"));
}

#[test]
fn read_file_ref_reads_text() {
    let result = read_file_ref("Cargo.toml");
    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(!content.text.is_empty());
    assert!(!content.is_image);
}

// ── Line range parsing ───────────────────────────────────────────────────────────

#[test]
fn parse_file_ref_with_range() {
    let parsed = parse_file_ref("src/main.rs:10-50").unwrap();
    assert_eq!(parsed.path, "src/main.rs");
    assert_eq!(parsed.range, Some(10..=50));
    assert_eq!(parsed.original, "src/main.rs:10-50");
}

#[test]
fn parse_file_ref_without_range() {
    let parsed = parse_file_ref("src/main.rs").unwrap();
    assert_eq!(parsed.path, "src/main.rs");
    assert_eq!(parsed.range, None);
}

#[test]
fn parse_file_ref_range_single_digit() {
    let parsed = parse_file_ref("file.txt:1-5").unwrap();
    assert_eq!(parsed.path, "file.txt");
    assert_eq!(parsed.range, Some(1..=5));
}

#[test]
fn parse_file_ref_invalid_start_zero() {
    // Line 0 is not valid.
    let parsed = parse_file_ref("file.txt:0-10");
    assert!(parsed.is_none(), "0 is not a valid line number");
}

#[test]
fn parse_file_ref_invalid_start_greater_than_end() {
    let parsed = parse_file_ref("file.txt:50-10").unwrap();
    assert_eq!(parsed.range, None);
    assert_eq!(parsed.path, "file.txt"); // colon separator found; invalid range → plain path
}

#[test]
fn parse_file_ref_no_hyphen() {
    // No hyphen means no valid range.
    let parsed = parse_file_ref("file.txt:100").unwrap();
    assert_eq!(parsed.range, None);
}

#[test]
fn parse_file_ref_trailing_colon() {
    // Trailing colon is not a range.
    let parsed = parse_file_ref("file.txt:").unwrap();
    assert_eq!(parsed.range, None);
}

#[test]
fn parse_file_ref_multiple_hyphens() {
    // Multiple hyphens — not a valid range.
    let parsed = parse_file_ref("file.txt:10-20-30").unwrap();
    assert_eq!(parsed.range, None);
}

#[test]
fn extract_lines_returns_range() {
    let text = "line1\nline2\nline3\nline4\nline5";
    let result = extract_lines(text, 2..=4).unwrap();
    assert_eq!(result, "line2\nline3\nline4");
}

#[test]
fn extract_lines_single_line() {
    let text = "line1\nline2\nline3";
    let result = extract_lines(text, 2..=2).unwrap();
    assert_eq!(result, "line2");
}

#[test]
fn extract_lines_clamped_to_bounds() {
    let text = "line1\nline2\nline3";
    // Request beyond file end — clamped.
    let result = extract_lines(text, 2..=100).unwrap();
    assert_eq!(result, "line2\nline3");
}

#[test]
fn extract_lines_empty_file() {
    let result = extract_lines("", 1..=10);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn extract_lines_invalid_range() {
    let text = "line1\nline2\nline3";
    let result = extract_lines(text, 5..=2);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("start"));
}

#[test]
fn read_file_ref_with_range_extracts_lines() {
    let result = read_file_ref_with_range("Cargo.toml", Some(1..=5));
    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(!content.text.is_empty());
    // Content should have at most 5 lines.
    let line_count = content.text.lines().count();
    assert!(line_count <= 5, "expected ≤5 lines, got {}", line_count);
}
