//! SWE-bench-style grader for: error_recovery task.
//!
//! Pass conditions:
//!   1. fetch_data has a retry loop
//!   2. Has exponential backoff
//!   3. Maximum 3 retries
//!   4. Returns Result type

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_matches, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates/runie-agent/src/harness/tasks/error_recovery")
}

fn main() {
    let task_dir = crate_root();
    let src_path = task_dir.join("src/api.rs");
    let mut checks = Vec::new();

    // Check 1: src/api.rs exists
    if !src_path.exists() {
        checks.push(Check::fail("src/api.rs not found"));
        let (_, total) = run_checks(checks);
        std::process::exit(if total > 0 { 0 } else { 1 });
    }

    // Check 2: Has retry loop
    let retry_patterns = [
        r"for\s+\w+\s+in\s+\d+\.\.",
        r"let\s+mut\s+\w+\s*=\s*\d+.*while",
        r"for\s+\w+\s+in\s+0\.\.\3",
    ];
    let has_retry_loop = retry_patterns
        .iter()
        .any(|p| file_matches(&src_path, p));
    if has_retry_loop {
        checks.push(Check::pass("has retry loop"));
    } else {
        checks.push(Check::fail("retry loop not found"));
    }

    // Check 3: Has exponential backoff
    let backoff_patterns = [r"\*\s*2", r"duration.*\*\s*2", r"sleep.*\*", r"pow\("];
    let has_backoff = backoff_patterns
        .iter()
        .any(|p| file_matches(&src_path, p));
    if has_backoff {
        checks.push(Check::pass("has exponential backoff"));
    } else {
        checks.push(Check::fail("exponential backoff not found"));
    }

    // Check 4: Max retries is 3
    let limit_patterns = [r"<\s*3", r"!=\s*3", r"<=\s*3", r"\.\.\3"];
    let has_limit = limit_patterns
        .iter()
        .any(|p| file_matches(&src_path, p));
    if has_limit {
        checks.push(Check::pass("max retries is 3"));
    } else {
        checks.push(Check::fail("max retries limit not found"));
    }

    // Check 5: Returns Result
    if file_matches(&src_path, r"fn\s+fetch_data.*->\s*Result<") {
        checks.push(Check::pass("returns Result<T, E>"));
    } else {
        checks.push(Check::fail("fetch_data does not return Result"));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_retry_loop() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/api.rs");
        let retry_patterns = [
            r"for\s+\w+\s+in\s+\d+\.\.",
            r"let\s+mut\s+\w+\s*=\s*\d+.*while",
            r"for\s+\w+\s+in\s+0\.\.\3",
        ];
        let has_retry = retry_patterns
            .iter()
            .any(|p| file_matches(&src_path, p));
        assert!(has_retry, "FAIL: retry loop not found");
        println!("PASS: retry loop found");
    }

    #[test]
    fn test_has_backoff() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/api.rs");
        let backoff_patterns = [r"\*\s*2", r"duration.*\*\s*2", r"sleep.*\*", r"pow\("];
        let has_backoff = backoff_patterns
            .iter()
            .any(|p| file_matches(&src_path, p));
        assert!(has_backoff, "FAIL: exponential backoff not found");
        println!("PASS: exponential backoff found");
    }

    #[test]
    fn test_max_retries_3() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/api.rs");
        let limit_patterns = [r"<\s*3", r"!=\s*3", r"<=\s*3", r"\.\.\3"];
        let has_limit = limit_patterns
            .iter()
            .any(|p| file_matches(&src_path, p));
        assert!(has_limit, "FAIL: max retries limit not found");
        println!("PASS: max retries limit found");
    }

    #[test]
    fn test_returns_result() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/api.rs");
        assert!(
            file_matches(&src_path, r"fn\s+fetch_data.*->\s*Result<"),
            "FAIL: fetch_data does not return Result"
        );
        println!("PASS: fetch_data returns Result");
    }
}
