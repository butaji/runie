//! SWE-bench-style grader for: param_struct task.
//!
//! Pass conditions:
//!   1. ServiceConfig struct is defined (pub struct ServiceConfig)
//!   2. Struct has fields: host, port, timeout_ms, max_connections, debug_mode
//!   3. Struct derives Debug and Clone
//!   4. init_service takes exactly one parameter (config: ServiceConfig)
//!   5. init_service returns Result<T, E>

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_contains, file_matches, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates/runie-agent/src/harness/tasks/param_struct")
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

    // Check 2: ServiceConfig struct defined
    if file_contains(&src_path, "pub struct ServiceConfig") {
        checks.push(Check::pass("has ServiceConfig struct"));
    } else {
        checks.push(Check::fail("ServiceConfig struct not found"));
    }

    // Check 3: Struct has required fields
    let required_fields = ["host", "port", "timeout_ms", "max_connections", "debug_mode"];
    let content_no_derive = content
        .split("pub struct ServiceConfig")
        .nth(1)
        .map(|s| s.split(|c| c == '#' || c == '\n').next().unwrap_or(s))
        .unwrap_or("");
    let missing_fields: Vec<&str> = required_fields
        .iter()
        .filter(|f| !content_no_derive.contains(*f))
        .cloned()
        .collect();
    if missing_fields.is_empty() {
        checks.push(Check::pass("struct has all 5 required fields"));
    } else {
        checks.push(Check::fail(format!(
            "missing fields: {}",
            missing_fields.join(", ")
        )));
    }

    // Check 4: Struct derives Debug and Clone
    let has_debug_clone = file_matches(&src_path, r"#\[derive\([^)]*Debug[^)]*\)")
        && file_matches(&src_path, r"#\[derive\([^)]*Clone[^)]*\)");
    if has_debug_clone {
        checks.push(Check::pass("struct derives Debug and Clone"));
    } else {
        checks.push(Check::fail("struct missing Debug or Clone derive"));
    }

    // Check 5: init_service takes single ServiceConfig param
    if file_matches(&src_path, r"pub fn init_service\s*\(\s*config:\s*ServiceConfig\s*\)") {
        checks.push(Check::pass("init_service takes single ServiceConfig param"));
    } else {
        checks.push(Check::fail("init_service signature incorrect"));
    }

    // Check 6: init_service returns Result
    if file_matches(&src_path, r"pub fn init_service[^)]+\)\s*->\s*Result<") {
        checks.push(Check::pass("init_service returns Result"));
    } else {
        checks.push(Check::fail("init_service does not return Result"));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_config_struct() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        assert!(
            file_contains(&src_path, "pub struct ServiceConfig"),
            "FAIL: ServiceConfig struct not found"
        );
        println!("PASS: ServiceConfig struct found");
    }

    #[test]
    fn test_required_fields() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        let content = std::fs::read_to_string(&src_path).unwrap_or_default();
        let required_fields = ["host", "port", "timeout_ms", "max_connections", "debug_mode"];
        let content_no_derive = content
            .split("pub struct ServiceConfig")
            .nth(1)
            .map(|s| s.split(|c| c == '#' || c == '\n').next().unwrap_or(s))
            .unwrap_or("");
        for field in required_fields {
            assert!(
                content_no_derive.contains(field),
                "FAIL: missing field {}",
                field
            );
        }
        println!("PASS: all required fields found");
    }

    #[test]
    fn test_derives_debug_clone() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        assert!(
            file_matches(&src_path, r"#\[derive\([^)]*Debug[^)]*\)"),
            "FAIL: missing Debug derive"
        );
        assert!(
            file_matches(&src_path, r"#\[derive\([^)]*Clone[^)]*\)"),
            "FAIL: missing Clone derive"
        );
        println!("PASS: Debug and Clone derive found");
    }

    #[test]
    fn test_init_service_signature() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        assert!(
            file_matches(&src_path, r"pub fn init_service\s*\(\s*config:\s*ServiceConfig\s*\)"),
            "FAIL: init_service signature incorrect"
        );
        println!("PASS: init_service takes single ServiceConfig param");
    }

    #[test]
    fn test_init_service_returns_result() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/lib.rs");
        assert!(
            file_matches(&src_path, r"pub fn init_service[^)]+\)\s*->\s*Result<"),
            "FAIL: init_service does not return Result"
        );
        println!("PASS: init_service returns Result");
    }
}
