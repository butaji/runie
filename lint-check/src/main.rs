//! Project linter.
//!
//! Enforces source and dependency-boundary rules across the workspace:
//!
//! 1. **Magic numbers >= 1000** must be replaced with named constants.
//! 2. **Orphan `tokio::spawn`** calls must be owned by an actor (handle stored in
//!    `JoinSet` or actor mailbox). For now we just flag each `tokio::spawn` site
//!    so the implementer adds a justifying comment with an owner.
//! 3. **Layer violations** are rejected: `runie-core` cannot depend on TUI or
//!    terminal crates, and `runie-tui-model` cannot depend on terminal crates.
//!
//! Run with: `cargo run -p lint-check`.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use walkdir::WalkDir;

const SCAN_ROOT: &str = "crates";
const TEST_DIR_MARKER: &str = "/tests/";

fn main() -> ExitCode {
    let findings = scan_project();
    if findings.is_empty() {
        println!("lint-check: clean");
        ExitCode::SUCCESS
    } else {
        report_findings(&findings);
        ExitCode::FAILURE
    }
}

fn scan_project() -> Vec<String> {
    let mut findings = scan_dependency_boundaries();
    findings.extend(
        WalkDir::new(SCAN_ROOT)
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|entry| scan_entry(entry.path()))
            .flatten(),
    );
    findings
}

fn scan_dependency_boundaries() -> Vec<String> {
    [
        (
            "crates/runie-core/Cargo.toml",
            &["runie-tui", "ratatui", "crossterm", "vt100"][..],
        ),
        (
            "crates/runie-tui-model/Cargo.toml",
            &["ratatui", "crossterm", "vt100"][..],
        ),
    ]
    .into_iter()
    .flat_map(|(manifest, forbidden)| {
        let Ok(contents) = fs::read_to_string(manifest) else {
            return vec![format!("{manifest}: manifest is missing")];
        };
        forbidden
            .iter()
            .filter(|dependency| dependency_declared(&contents, dependency))
            .map(|dependency| {
                format!("{manifest}: forbidden dependency `{dependency}` violates layer boundary")
            })
            .collect()
    })
    .collect()
}

fn dependency_declared(manifest: &str, dependency: &str) -> bool {
    manifest.lines().any(|line| {
        let line = line.split('#').next().unwrap_or_default().trim();
        line.strip_prefix(dependency)
            .is_some_and(|rest| rest.starts_with(' ') || rest.starts_with('='))
    })
}

fn scan_entry(path: &Path) -> Option<Vec<String>> {
    let is_rust = path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs");
    let path_str = path.to_string_lossy().to_string();
    if !is_rust || !path_str.contains("/src/") || path_str.contains(TEST_DIR_MARKER) {
        return None;
    }
    let src = fs::read_to_string(path).ok()?;
    Some(scan_source(&path_str, &src))
}

fn scan_source(path: &str, src: &str) -> Vec<String> {
    src.lines()
        .enumerate()
        .flat_map(|(idx, line)| scan_line(path, src, idx, line))
        .collect()
}

fn scan_line(path: &str, src: &str, idx: usize, line: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let trimmed = line.trim_start();
    if (path.contains("/crates/runie-tui/src/") || path.starts_with("crates/runie-tui/src/"))
        && !path.ends_with("/yaml_runner.rs")
        && !path.contains("/src/bin/")
        && (trimmed.contains("std::fs::") || trimmed.contains("std::process::Command"))
    {
        findings.push(format!(
            "{path}:{}: blocking filesystem/process API in runtime TUI; use an owned async boundary",
            idx + 1
        ));
    }
    if let Some(lit) = extract_numeric_literal(trimmed) {
        if !is_exempt(trimmed, &lit)
            && !(path.ends_with("/appearance.rs") && trimmed.contains("#"))
            && !is_const_decl(trimmed)
            && lit.replace('_', "").parse::<u64>().is_ok_and(|n| n >= 1000)
        {
            findings.push(format!("{path}:{}: magic number >= 1000: `{lit}`", idx + 1));
        }
    }
    if trimmed.contains("tokio::spawn(") && !trimmed.starts_with("//") {
        let window_start = idx.saturating_sub(3);
        let owned = src
            .lines()
            .skip(window_start)
            .take(idx - window_start + 1)
            .any(|l| l.contains("// OWNER"));
        if !owned && !line_contains_joinset_above(src, idx) {
            findings.push(format!("{path}:{}: `tokio::spawn` must be owned by an actor (store handle in JoinSet or add `// OWNER: <actor>` comment)", idx + 1));
        }
    }
    findings
}

fn report_findings(findings: &[String]) {
    eprintln!("lint-check found {} issue(s):", findings.len());
    for finding in findings {
        eprintln!("  {finding}");
    }
}

fn extract_numeric_literal(line: &str) -> Option<String> {
    let after_colon = line.split(':').next_back().unwrap_or(line).trim();
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

#[cfg(test)]
mod tests {
    use super::dependency_declared;

    #[test]
    fn dependency_boundary_parser_ignores_comments_and_similar_names() {
        let manifest = "# ratatui = \"0.29\"\nratatui-extra = \"1\"\n";
        assert!(!dependency_declared(manifest, "ratatui"));
    }

    #[test]
    fn dependency_boundary_parser_accepts_table_and_version_forms() {
        assert!(dependency_declared("ratatui = \"0.29\"", "ratatui"));
        assert!(dependency_declared(
            "ratatui = { version = \"0.29\" }",
            "ratatui"
        ));
    }
}
