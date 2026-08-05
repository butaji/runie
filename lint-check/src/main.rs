//! Project linter.
//!
//! Enforces two rules across `crates/runie-core/src/**.rs`:
//!
//! 1. **Magic numbers >= 1000** must be replaced with named constants.
//! 2. **Orphan `tokio::spawn`** calls must be owned by an actor (handle stored in
//!    `JoinSet` or actor mailbox). For now we just flag each `tokio::spawn` site
//!    so the implementer adds a justifying comment with an owner.
//!
//! Run with: `cargo run -p lint-check`.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use walkdir::WalkDir;

const SCAN_ROOT: &str = "crates";
const TEST_DIR_MARKER: &str = "/tests/";

fn main() -> ExitCode {
    let mut findings: Vec<String> = Vec::new();

    for entry in WalkDir::new(SCAN_ROOT).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        // Only scan production src/, exclude tests/.
        if !path_str.contains("/src/") {
            continue;
        }
        if path_str.contains(TEST_DIR_MARKER) {
            continue;
        }

        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };

        for (idx, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();

            // Magic number >= 1000 in production code (excluding comments, underscore-separated,
            // and `const X: usize = N` / `const X: u32 = N` named-constant declarations).
            if let Some(lit) = extract_numeric_literal(trimmed) {
                if !is_exempt(trimmed, &lit) && !is_const_decl(trimmed) {
                    if let Ok(n) = lit.replace('_', "").parse::<u64>() {
                        if n >= 1000 {
                            findings.push(format!(
                                "{}:{}: magic number >= 1000: `{}`",
                                path_str,
                                idx + 1,
                                lit
                            ));
                        }
                    }
                }
            }

            // Orphan tokio::spawn check.
            if trimmed.contains("tokio::spawn(") && !trimmed.starts_with("//") {
                // Require either a `JoinSet` nearby OR a justifying `// OWNER:` comment.
                let window_start = idx.saturating_sub(3);
                let has_owner_marker = src
                    .lines()
                    .skip(window_start)
                    .take(idx - window_start + 1)
                    .any(|l| l.contains("// OWNER"));
                let in_joinset = line_contains_joinset_above(&src, idx);
                if !has_owner_marker && !in_joinset {
                    findings.push(format!(
                        "{}:{}: `tokio::spawn` must be owned by an actor \
                         (store handle in JoinSet or add `// OWNER: <actor>` comment)",
                        path_str,
                        idx + 1
                    ));
                }
            }
        }
    }

    if findings.is_empty() {
        println!("lint-check: clean");
        ExitCode::SUCCESS
    } else {
        eprintln!("lint-check found {} issue(s):", findings.len());
        for f in findings {
            eprintln!("  {f}");
        }
        ExitCode::FAILURE
    }
}

fn extract_numeric_literal(line: &str) -> Option<String> {
    let after_colon = line.split(':').last().unwrap_or(line).trim();
    let bytes = after_colon.as_bytes();
    let mut i = 0;
    // Skip leading non-digit chars but stop early if we find a digit start.
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'_') {
        i += 1;
    }
    Some(after_colon[start..i].to_string())
}

fn is_exempt(line: &str, lit: &str) -> bool {
    // Underscore-separated literals (e.g. 1_024).
    if lit.contains('_') {
        return true;
    }
    // Hex.
    if line.contains("0x") || line.contains("0X") {
        return true;
    }
    false
}

/// `const FOO: usize = 1024;` style declarations are intentional.
fn is_const_decl(line: &str) -> bool {
    line.starts_with("const ") || line.contains("pub const ") || line.contains("const CAP_")
}

fn line_contains_joinset_above(src: &str, idx: usize) -> bool {
    let window_start = idx.saturating_sub(8);
    src.lines()
        .skip(window_start)
        .take(idx - window_start + 1)
        .any(|l| l.contains("JoinSet") || l.contains(".spawn_owned(") || l.contains("JoinHandle"))
}

#[allow(dead_code)]
fn _path_marker(_p: &Path) {}
