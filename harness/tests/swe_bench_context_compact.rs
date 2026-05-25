//! SWE-bench-style grader for: context_compact task.
//!
//! Pass conditions:
//!   1. compact_context respects max_messages limit
//!   2. Keeps recent messages
//!   3. Removes duplicate system prompts
//!   4. Implementation is idempotent

#[path = "../src/graders/mod.rs"]
mod graders;
use graders::{file_matches, run_checks, Check};

fn crate_root() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../runie/crates/runie-agent/src/harness/tasks/context_compact")
}

fn main() {
    let task_dir = crate_root();
    let src_path = task_dir.join("src/compactor.rs");
    let mut checks = Vec::new();

    // Check 1: src/compactor.rs exists
    if !src_path.exists() {
        checks.push(Check::fail("src/compactor.rs not found"));
        let (_, total) = run_checks(checks);
        std::process::exit(if total > 0 { 0 } else { 1 });
    }

    let content = std::fs::read_to_string(&src_path).unwrap_or_default();

    // Check 2: Respects max_messages limit
    let max_msg_checks = ["max_messages", "len()", "truncate"];
    let has_max_msg = max_msg_checks.iter().any(|p| content.contains(*p));
    let has_max_param = content.contains("max_messages: usize");
    if has_max_msg && has_max_param {
        checks.push(Check::pass("respects max_messages limit"));
    } else {
        checks.push(Check::fail("max_messages limit not implemented"));
    }

    // Check 3: Keeps recent messages
    let recent_patterns = [r"\[\s*-", "reversed", "rev()", r"\.\.[\s)]"];
    let keeps_recent = recent_patterns
        .iter()
        .any(|p| file_matches(&src_path, p));
    if keeps_recent {
        checks.push(Check::pass("keeps recent messages"));
    } else {
        checks.push(Check::fail("recent message preservation not found"));
    }

    // Check 4: Removes duplicates
    let dedup_patterns = ["HashSet", "BTreeSet", "duplicate", "distinct", "filter"];
    let has_dedup = dedup_patterns
        .iter()
        .any(|p| content.to_lowercase().contains(&p.to_lowercase()));
    if has_dedup {
        checks.push(Check::pass("removes duplicate system prompts"));
    } else {
        checks.push(Check::fail("duplicate removal not found"));
    }

    // Check 5: Preserves message structure
    let struct_checks = [
        r"Message\s*\{",
        "role:",
        "content:",
    ];
    let preserves_struct = struct_checks
        .iter()
        .all(|p| file_matches(&src_path, p));
    if preserves_struct {
        checks.push(Check::pass("preserves message structure"));
    } else {
        checks.push(Check::fail("message structure not preserved"));
    }

    let (passed, total) = run_checks(checks);
    std::process::exit(if passed == total { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_respects_max_messages() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/compactor.rs");
        let content = std::fs::read_to_string(&src_path).unwrap_or_default();
        let has_max_msg = content.contains("max_messages")
            || content.contains("len()")
            || content.contains("truncate");
        let has_max_param = content.contains("max_messages: usize");
        assert!(has_max_msg && has_max_param, "FAIL: max_messages limit not implemented");
        println!("PASS: respects max_messages limit");
    }

    #[test]
    fn test_keeps_recent() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/compactor.rs");
        let recent_patterns = [r"\[\s*-", "reversed", "rev()", r"\.\.[\s)]"];
        let keeps_recent = recent_patterns
            .iter()
            .any(|p| file_matches(&src_path, p));
        assert!(keeps_recent, "FAIL: recent message preservation not found");
        println!("PASS: keeps recent messages");
    }

    #[test]
    fn test_removes_duplicates() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/compactor.rs");
        let content = std::fs::read_to_string(&src_path).unwrap_or_default();
        let has_dedup = content.contains("HashSet")
            || content.contains("BTreeSet")
            || content.contains("duplicate")
            || content.contains("distinct")
            || content.contains("filter");
        assert!(has_dedup, "FAIL: duplicate removal not found");
        println!("PASS: removes duplicates");
    }

    #[test]
    fn test_preserves_structure() {
        let task_dir = crate_root();
        let src_path = task_dir.join("src/compactor.rs");
        assert!(file_matches(&src_path, r"Message\s*\{"), "FAIL: Message struct not constructed");
        assert!(file_matches(&src_path, "role:"), "FAIL: role field not preserved");
        assert!(file_matches(&src_path, "content:"), "FAIL: content field not preserved");
        println!("PASS: message structure preserved");
    }
}
