use crate::truncate::{truncate_head, truncate_tail, TruncationPolicy};

#[test]
fn policy_from_toml() {
    // Configurable from config.toml. This is the parse-side of the new feature.
    #[derive(serde::Deserialize)]
    struct Wrap {
        truncation: crate::truncate::TruncationConfig,
    }
    let toml = r#"
[truncation]
max_lines = 500
max_bytes = 10000
"#;
    let parsed: Wrap = toml::from_str(toml).unwrap();
    assert_eq!(parsed.truncation.max_lines, 500);
    assert_eq!(parsed.truncation.max_bytes, 10000);
}

#[test]
fn policy_from_toml_defaults() {
    // No [truncation] section — fields fall back to defaults.
    #[derive(serde::Deserialize)]
    struct Wrap {
        #[serde(default)]
        truncation: crate::truncate::TruncationConfig,
    }
    let parsed: Wrap = toml::from_str("").unwrap();
    assert_eq!(parsed.truncation.max_lines, 2000);
    assert_eq!(parsed.truncation.max_bytes, 50 * 1024);
}

#[test]
fn policy_from_section_honors_values() {
    // The term-crate wiring path: take the parsed TruncationSection
    // (max_lines, max_bytes) and build a TruncationPolicy.
    let p = crate::truncate::policy_from_section(500, 10_000);
    assert_eq!(p.max_lines, 500);
    assert_eq!(p.max_bytes, 10_000);
}

#[test]
fn policy_from_section_zero_means_default() {
    // 0 is the TOML default for missing fields; it should mean "use default".
    let p = crate::truncate::policy_from_section(0, 0);
    assert_eq!(p.max_lines, crate::truncate::DEFAULT_MAX_LINES);
    assert_eq!(p.max_bytes, crate::truncate::DEFAULT_MAX_BYTES);
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
