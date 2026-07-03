use crate::truncate::{truncate_head, truncate_tail, TruncationPolicy};

#[test]
fn truncation_policy_from_config() {
    // TruncationPolicy builds from a TruncationSection with custom values.
    let section = runie_core::config::TruncationSection {
        max_lines: 500,
        max_bytes: 10_000,
    };
    let p = crate::truncate::policy_from_section(&section);
    assert_eq!(p.max_lines, 500);
    assert_eq!(p.max_bytes, 10_000);
}

#[test]
fn truncation_defaults_match() {
    // Defaults still 2000 lines / 50*1024 bytes.
    let section = runie_core::config::TruncationSection::default();
    assert_eq!(section.max_lines, 2000);
    assert_eq!(section.max_bytes, 50 * 1024);

    let p = crate::truncate::policy_from_section(&section);
    assert_eq!(p.max_lines, 2000);
    assert_eq!(p.max_bytes, 50 * 1024);
}

fn policy(lines: usize, bytes: usize) -> TruncationPolicy {
    TruncationPolicy {
        max_lines: lines,
        max_bytes: bytes,
    }
}

#[test]
fn head_keeps_beginning() {
    let content = "line1\nline2\nline3\nline4\nline5";
    let out = truncate_head(content, &policy(3, 1000));
    assert!(out.was_truncated);
    assert_eq!(out.content, "line1\nline2\nline3");
    assert_eq!(out.output_lines, 3);
}

#[test]
fn tail_keeps_end() {
    let content = "line1\nline2\nline3\nline4\nline5";
    let out = truncate_tail(content, &policy(3, 1000));
    assert!(out.was_truncated);
    assert_eq!(out.content, "line3\nline4\nline5");
    assert_eq!(out.output_lines, 3);
}

#[test]
fn no_truncation_when_under_limits() {
    let content = "short";
    let out = truncate_head(content, &policy(100, 10000));
    assert!(!out.was_truncated);
    assert_eq!(out.content, "short");
}

#[test]
fn head_respects_byte_limit() {
    let content = "aaaaaaaaaa\nbbbbbbbbbb\ncccccccccc";
    let out = truncate_head(content, &policy(100, 15));
    assert!(out.was_truncated);
    assert_eq!(out.output_lines, 1);
    assert!(out.output_bytes <= 15);
}

#[test]
fn tail_respects_byte_limit() {
    let content = "aaaaaaaaaa\nbbbbbbbbbb\ncccccccccc";
    let out = truncate_tail(content, &policy(100, 15));
    assert!(out.was_truncated);
    assert_eq!(out.output_lines, 1);
    assert!(out.output_bytes <= 15);
}

#[test]
fn no_partial_lines_head() {
    let content = "aaaaaaaaaa\nbbbbbbbbbb";
    let out = truncate_head(content, &policy(100, 12));
    assert!(out.was_truncated);
    assert_eq!(out.content, "aaaaaaaaaa");
}

#[test]
fn no_partial_lines_tail() {
    let content = "aaaaaaaaaa\nbbbbbbbbbb";
    let out = truncate_tail(content, &policy(100, 12));
    assert!(out.was_truncated);
    assert_eq!(out.content, "bbbbbbbbbb");
}

#[test]
fn empty_string_noop() {
    let out = truncate_head("", &policy(10, 100));
    assert!(!out.was_truncated);
    assert_eq!(out.content, "");
}

#[test]
fn truncation_tracks_totals() {
    let content = "a\nb\nc\nd\ne";
    let out = truncate_head(content, &policy(2, 100));
    assert_eq!(out.total_lines, 5);
    assert_eq!(out.total_bytes, content.len());
    assert_eq!(out.output_lines, 2);
}
