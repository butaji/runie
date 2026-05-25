//! SWE-bench-style grader for: readme_maker task.
//!
//! Pass conditions:
//!   1. README.md exists and has a title (# ...)
//!   2. Has an Installation section (## Installation or similar)
//!   3. Has a Usage section with code block (`'`'`)
//!   4. Has a License section
//!   5. Under 30 lines total

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_matches, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates/runie-agent/src/harness/tasks/readme_maker")
}

fn main() {
    let task_dir = crate_root();
    let readme_path = task_dir.join("README.md");
    let mut checks = Vec::new();

    // Check 1: README.md exists
    if readme_path.exists() {
        checks.push(Check::pass("README.md exists"));
    } else {
        checks.push(Check::fail("README.md not found"));
        let (_, total) = run_checks(checks);
        std::process::exit(if total > 0 { 0 } else { 1 });
    }

    let content = std::fs::read_to_string(&readme_path).unwrap_or_default();

    // Check 2: Has title (# heading)
    if file_matches(&readme_path, r"^#\s+\w") {
        checks.push(Check::pass("has title (# heading)"));
    } else {
        checks.push(Check::fail("missing title (# heading)"));
    }

    // Check 3: Has Installation section
    let has_install = content.to_lowercase().contains("installation")
        || content.to_lowercase().contains("install");
    if has_install {
        checks.push(Check::pass("has installation section"));
    } else {
        checks.push(Check::fail("missing installation section"));
    }

    // Check 4: Has code block
    if content.contains("```") {
        checks.push(Check::pass("has usage code block"));
    } else {
        checks.push(Check::fail("missing code block"));
    }

    // Check 5: Has License section
    if content.to_lowercase().contains("license") {
        checks.push(Check::pass("has license section"));
    } else {
        checks.push(Check::fail("missing license section"));
    }

    // Check 6: Under 30 non-empty lines
    let non_empty_lines: usize = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    if non_empty_lines <= 30 {
        checks.push(Check::pass_with_detail(
            "under 30 non-empty lines",
            format!("({} lines)", non_empty_lines),
        ));
    } else {
        checks.push(Check::fail_with_detail(
            "under 30 non-empty lines",
            format!("({} lines)", non_empty_lines),
        ));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_readme_exists() {
        let task_dir = crate_root();
        let readme_path = task_dir.join("README.md");
        assert!(readme_path.exists(), "FAIL: README.md not found");
        println!("PASS: README.md exists");
    }

    #[test]
    fn test_has_title() {
        let task_dir = crate_root();
        let readme_path = task_dir.join("README.md");
        assert!(
            file_matches(&readme_path, r"^#\s+\w"),
            "FAIL: missing title (# heading)"
        );
        println!("PASS: has title (# heading)");
    }

    #[test]
    fn test_has_installation() {
        let task_dir = crate_root();
        let readme_path = task_dir.join("README.md");
        let content = std::fs::read_to_string(&readme_path).unwrap_or_default();
        let has_install = content.to_lowercase().contains("installation")
            || content.to_lowercase().contains("install");
        assert!(has_install, "FAIL: missing installation section");
        println!("PASS: has installation section");
    }

    #[test]
    fn test_has_code_block() {
        let task_dir = crate_root();
        let readme_path = task_dir.join("README.md");
        let content = std::fs::read_to_string(&readme_path).unwrap_or_default();
        assert!(content.contains("```"), "FAIL: missing code block");
        println!("PASS: has code block");
    }

    #[test]
    fn test_has_license() {
        let task_dir = crate_root();
        let readme_path = task_dir.join("README.md");
        let content = std::fs::read_to_string(&readme_path).unwrap_or_default();
        assert!(
            content.to_lowercase().contains("license"),
            "FAIL: missing license section"
        );
        println!("PASS: has license section");
    }

    #[test]
    fn test_under_30_lines() {
        let task_dir = crate_root();
        let readme_path = task_dir.join("README.md");
        let content = std::fs::read_to_string(&readme_path).unwrap_or_default();
        let non_empty_lines: usize = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        assert!(non_empty_lines <= 30, "FAIL: exceeds 30 non-empty lines");
        println!("PASS: under 30 non-empty lines ({})", non_empty_lines);
    }
}
