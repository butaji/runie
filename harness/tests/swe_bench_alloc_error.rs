//! SWE-bench-style grader for: alloc_error task.
//!
//! Pass conditions:
//!   1. function signature returns Result<T, E>
//!   2. No panic!() calls in the function body
//!   3. Error message includes the offending size value

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_matches, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates/runie-agent/src/harness/tasks/alloc_error")
}

fn main() {
    let task_dir = crate_root();
    let src_path = task_dir.join("src/lib.rs");
    let mut checks = Vec::new();

    // Check 1: src/lib.rs exists
    if !src_path.exists() {
        checks.push(Check::fail("src/lib.rs not found"));
        let (_, total) = run_checks(checks);
        std::process::exit(if total > 0 { 0 } else { 1 });
    }

    let content = std::fs::read_to_string(&src_path).unwrap_or_default();

    // Check 2: allocate_buffer function returns Result
    // Look for function signature with Result return type
    let has_result = file_matches(&src_path, r"pub fn allocate_buffer[^)]+\)\s*->\s*Result<")
        || file_matches(&src_path, r"fn allocate_buffer[^)]+\)\s*->\s*Result<");
    if has_result {
        checks.push(Check::pass("function returns Result"));
    } else {
        checks.push(Check::fail("allocate_buffer does not return Result"));
    }

    // Check 3: No panic!() in function body
    // Extract function body and check for panic!
    let has_panic = content.contains("panic!");
    if !has_panic {
        checks.push(Check::pass("no panic!() calls in function body"));
    } else {
        // More nuanced check - ensure panic is not in allocate_buffer
        // Extract allocate_buffer function
        if let Some(fn_start) = content.find("fn allocate_buffer") {
            let fn_chunk = &content[fn_start..];
            let fn_end = fn_chunk[4..]
                .find("pub fn ")
                .map(|i| i + 4)
                .or_else(|| fn_chunk.find("fn "))
                .unwrap_or(fn_chunk.len());
            let fn_body = &fn_chunk[..fn_end];
            if !fn_body.contains("panic!") {
                checks.push(Check::pass("no panic!() calls in function body"));
            } else {
                checks.push(Check::fail("panic!() found in function body"));
            }
        } else {
            checks.push(Check::fail("allocate_buffer function not found"));
        }
    }

    // Check 4: Error message includes size info
    // Look for Err(...) with size mention
    let has_size_error = content.contains("size")
        || content.contains("usize")
        || content.contains("{size}");
    if has_size_error {
        checks.push(Check::pass("error messages include size info"));
    } else {
        checks.push(Check::fail("error message does not include size info"));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_result() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        let has_result = file_matches(&src_path, r"pub fn allocate_buffer[^)]+\)\s*->\s*Result<")
            || file_matches(&src_path, r"fn allocate_buffer[^)]+\)\s*->\s*Result<");
        assert!(has_result, "FAIL: allocate_buffer does not return Result");
        println!("PASS: allocate_buffer returns Result");
    }

    #[test]
    fn test_no_panic_in_body() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        let content = std::fs::read_to_string(&src_path).unwrap_or_default();

        // Extract allocate_buffer function
        if let Some(fn_start) = content.find("fn allocate_buffer") {
            let fn_chunk = &content[fn_start..];
            let fn_end = fn_chunk[4..]
                .find("pub fn ")
                .map(|i| i + 4)
                .or_else(|| fn_chunk.find("fn "))
                .unwrap_or(fn_chunk.len());
            let fn_body = &fn_chunk[..fn_end];
            assert!(
                !fn_body.contains("panic!"),
                "FAIL: panic!() found in function body"
            );
        }
        println!("PASS: no panic!() in function body");
    }

    #[test]
    fn test_error_includes_size() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        let content = std::fs::read_to_string(&src_path).unwrap_or_default();
        let has_size_error =
            content.contains("size") || content.contains("usize") || content.contains("{size}");
        assert!(
            has_size_error,
            "FAIL: error message does not include size info"
        );
        println!("PASS: error includes size info");
    }
}
