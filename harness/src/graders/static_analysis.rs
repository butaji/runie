#![allow(dead_code)]

//! Static analysis utilities for Rust source code validation.
//!
//! Provides pattern matching and source file analysis for grader tests.

use std::path::Path;
use regex::Regex;

/// Result of a single check.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: Option<String>,
}

impl Check {
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            detail: None,
        }
    }

    pub fn pass_with_detail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            detail: Some(detail.into()),
        }
    }

    pub fn fail(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            detail: None,
        }
    }

    pub fn fail_with_detail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            detail: Some(detail.into()),
        }
    }

    pub fn info(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            detail: None,
        }
    }

    pub fn print_result(&self) {
        let status = if self.passed { "PASS" } else { "FAIL" };
        if let Some(ref detail) = self.detail {
            println!("{}: {} ({})", status, self.name, detail);
        } else {
            println!("{}: {}", status, self.name);
        }
    }
}

/// Check if a file contains a pattern.
pub fn file_contains<P: AsRef<Path>>(path: P, pattern: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|content| content.contains(pattern))
        .unwrap_or(false)
}

/// Check if a file matches a regex pattern.
pub fn file_matches<P: AsRef<Path>>(path: P, pattern: &str) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return false,
    };
    regex.is_match(&content)
}

/// Extract captures from a regex pattern in a file.
pub fn file_captures<P: AsRef<Path>>(path: P, pattern: &str) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let regex = Regex::new(pattern).ok()?;
    let caps = regex.captures(&content)?;
    Some(
        caps.iter()
            .skip(1) // Skip full match
            .filter_map(|m| m.map(|m| m.as_str().to_string()))
            .collect(),
    )
}

/// Search for a pattern in a directory recursively.
pub fn dir_contains(dir: &Path, pattern: &str, include: Option<&str>) -> bool {
    let pattern_lower = pattern.to_lowercase();
    walkdir(dir, &mut |path: &Path| {
        if let Some(ext) = include {
            if let Some(path_ext) = path.extension() {
                if path_ext != ext {
                    return true; // skip
                }
            } else {
                return true; // skip
            }
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            content.to_lowercase().contains(&pattern_lower)
        } else {
            false
        }
    })
}

/// Search for a regex pattern in a directory recursively.
pub fn dir_matches(dir: &Path, pattern: &str, include: Option<&str>) -> bool {
    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return false,
    };
    walkdir(dir, &mut |path: &Path| {
        if let Some(ext) = include {
            if let Some(path_ext) = path.extension() {
                if path_ext != ext {
                    return true; // skip
                }
            } else {
                return true; // skip
            }
        }
        if let Ok(content) = std::fs::read_to_string(path) {
            regex.is_match(&content)
        } else {
            false
        }
    })
}

/// Walk directory recursively, calling predicate on each file.
/// Uses explicit stack to avoid recursion limit issues.
fn walkdir(dir: &Path, predicate: &mut impl FnMut(&Path) -> bool) -> bool {
    if !dir.is_dir() {
        return false;
    }

    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        if !current.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                // Skip target and .git directories
                if path.file_name().map(|n| n == "target" || n == ".git").unwrap_or(false) {
                    continue;
                }
                stack.push(path);
            } else {
                if predicate(&path) {
                    return true;
                }
            }
        }
    }
    false
}

/// Run all checks and print results.
pub fn run_checks(checks: Vec<Check>) -> (usize, usize) {
    let total = checks.len();
    let passed = checks.iter().filter(|c| c.passed).count();

    for check in &checks {
        check.print_result();
    }

    println!("\n{}/{} checks passed", passed, total);

    if passed == total {
        println!("RESULT: pass");
    } else {
        println!("RESULT: fail ({} passed)", passed);
    }

    (passed, total)
}
