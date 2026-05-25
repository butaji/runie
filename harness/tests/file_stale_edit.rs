//! Grader for file_stale_edit task.
//!
//! Verifies that edit_file tool detects when a file has been modified
//! between read and write operations (TOCTOU race condition).

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates")
}

fn main() {
    let root = crate_root();
    let edit_file_path = root.join("runie-tools/src/edit_file.rs");
    let mut checks = Vec::new();

    if !edit_file_path.exists() {
        checks.push(Check::fail("edit_file.rs not found"));
        let (_, total) = run_checks(checks);
        std::process::exit(if total > 0 { 0 } else { 1 });
    }

    let content = std::fs::read_to_string(&edit_file_path).unwrap_or_default();

    // Check 1: mtime tracking
    let mtime_patterns = [
        "modified()",
        "mtime",
        "last_modified",
        "file_time",
        "metadata().modified",
    ];
    let has_mtime = mtime_patterns.iter().any(|p| content.contains(*p));
    if has_mtime {
        checks.push(Check::pass("has mtime tracking"));
    } else {
        checks.push(Check::fail("mtime tracking not found"));
    }

    // Check 2: File change detection
    let detect_patterns = [
        "original_mtime",
        "saved_mtime",
        "current_mtime",
        "previous_mtime",
    ];
    let has_detection = detect_patterns.iter().any(|p| content.contains(*p));
    if has_detection {
        checks.push(Check::pass("detects file changes"));
    } else {
        checks.push(Check::fail("file change detection not found"));
    }

    // Check 3: Meaningful error message
    let error_patterns = [
        "stale",
        "modified since",
        "concurrent modification",
        "file changed",
        "TOCTOU",
    ];
    let has_error = error_patterns
        .iter()
        .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
    if has_error {
        checks.push(Check::pass("returns meaningful error"));
    } else {
        checks.push(Check::fail("meaningful error not found"));
    }

    // Check 4: Does not silently overwrite
    // Look for conditional check before write
    let has_guard = content.contains("if")
        && (content.contains("mtime") || content.contains("modified"));
    if has_guard {
        checks.push(Check::pass("does not silently overwrite"));
    } else {
        checks.push(Check::fail("no guard before write found"));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_mtime_tracking() {
        let root = crate_root();
        let path = root.join("runie-tools/src/edit_file.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mtime_patterns = [
            "modified()",
            "mtime",
            "last_modified",
            "file_time",
            "metadata().modified",
        ];
        let has_mtime = mtime_patterns.iter().any(|p| content.contains(*p));
        assert!(has_mtime, "FAIL: mtime tracking not found");
        println!("PASS: mtime tracking found");
    }

    #[test]
    fn test_detects_file_change() {
        let root = crate_root();
        let path = root.join("runie-tools/src/edit_file.rs");
        let detect_patterns = [
            "original_mtime",
            "saved_mtime",
            "current_mtime",
            "previous_mtime",
        ];
        let has_detection = detect_patterns.iter().any(|p| file_contains(&path, *p));
        assert!(has_detection, "FAIL: file change detection not found");
        println!("PASS: file change detection found");
    }

    #[test]
    fn test_meaningful_error() {
        let root = crate_root();
        let path = root.join("runie-tools/src/edit_file.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let error_patterns = [
            "stale",
            "modified since",
            "concurrent modification",
            "file changed",
            "TOCTOU",
        ];
        let has_error = error_patterns
            .iter()
            .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
        assert!(has_error, "FAIL: meaningful error not found");
        println!("PASS: meaningful error found");
    }

    #[test]
    fn test_no_silent_overwrite() {
        let root = crate_root();
        let path = root.join("runie-tools/src/edit_file.rs");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let has_guard = content.contains("if")
            && (content.contains("mtime") || content.contains("modified"));
        assert!(has_guard, "FAIL: no guard before write found");
        println!("PASS: does not silently overwrite");
    }
}
