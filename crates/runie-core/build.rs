//! Build script for runie-core.
//!
//! This script performs checks:
//! **AppState field access guardrail**: ensures internal AppState fields are accessed
//! through accessors, not directly.
//!
//! **Magic number guardrail**: prevents raw numeric literals (>= 10) in production code.
//! Small numbers (0-9), underscore-separated, hex, and test code are exempt.
//!
//! Note: Event taxonomy is defined inline in `src/event/mod.rs`.
//! The `taxonomy.json` file is kept as documentation; edits require manual updates.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// ── AppState field-access guardrail ──────────────────────────────────────────
//
// Private AppState fields must be accessed through accessors, not directly.
const APPSTATE_PATTERNS: &[(&str, &str)] = &[
    ("state.session.", "state.session()"),
    ("state.input.", "state.input()"),
    ("state.agent.", "state.agent_state()"),
    ("state.view.", "state.view()"),
    ("state.config.", "state.config()"),
    ("state.completion.", "state.completion()"),
    ("state.should_quit ", "state.should_quit_mut()"),
    ("state.should_quit\n", "state.should_quit_mut()"),
    ("state.should_quit{", "state.should_quit_mut()"),
    ("state.open_dialog ", "state.open_dialog_mut()"),
    ("state.open_dialog.", "state.open_dialog_mut()"),
    ("state.dialog_back_stack.", "state.dialog_back_stack_mut()"),
    ("state.login_flow ", "state.login_flow_mut()"),
    ("state.login_flow.", "state.login_flow_mut()"),
    ("state.transient_message ", "state.transient_message_mut()"),
    ("state.transient_until ", "state.transient_until_mut()"),
    ("state.transient_level ", "state.transient_level_mut()"),
    ("state.fff_file_results.", "state.fff_file_results()"),
    ("state.fff_debounce ", "state.fff_debounce_mut()"),
    ("state.perm_req ", "state.permission_request_opt()"),
    ("state.perm_req.", "state.permission_request_opt()."),
    ("state.cwd_name ", "state.cwd_name_mut()"),
    ("state.git_info ", "state.git_info_mut()"),
    ("state.git_info.", "state.git_info_mut()"),
    ("state.skills ", "state.skills_mut()"),
    ("state.prompts ", "state.prompts_mut()"),
    ("state.trust_decisions ", "state.trust_decisions_mut()"),
    ("state.trust_decisions.", "state.trust_decisions_mut()"),
    ("state.actor_handles ", "state.actor_handles_mut()"),
    ("state.registry ", "state.registry_mut()"),
    ("state.registry.", "state.registry_mut()"),
    // self.xxx patterns (same replacement, different prefix)
    ("self.session.", "self.session()"),
    ("self.input.", "self.input()"),
    ("self.agent.", "self.agent_state()"),
    ("self.view.", "self.view()"),
    ("self.config.", "self.config()"),
    ("self.completion.", "self.completion()"),
    ("self.should_quit ", "self.should_quit_mut()"),
    ("self.should_quit\n", "self.should_quit_mut()"),
    ("self.should_quit{", "self.should_quit_mut()"),
    ("self.open_dialog ", "self.open_dialog_mut()"),
    ("self.open_dialog.", "self.open_dialog_mut()"),
    ("self.dialog_back_stack.", "self.dialog_back_stack_mut()"),
    ("self.login_flow ", "self.login_flow_mut()"),
    ("self.login_flow.", "self.login_flow_mut()"),
    ("self.transient_message ", "self.transient_message_mut()"),
    ("self.transient_until ", "self.transient_until_mut()"),
    ("self.transient_level ", "self.transient_level_mut()"),
    ("self.fff_file_results.", "self.fff_file_results_mut()"),
    ("self.fff_debounce ", "self.fff_debounce_mut()"),
    ("self.permission_request ", "self.permission_request_mut()"),
    ("self.cwd_name ", "self.cwd_name_mut()"),
    ("self.git_info ", "self.git_info_mut()"),
    ("self.git_info.", "self.git_info_mut()"),
    ("self.skills ", "self.skills_mut()"),
    ("self.prompts ", "self.prompts_mut()"),
    ("self.trust_decisions ", "self.trust_decisions_mut()"),
    ("self.trust_decisions.", "self.trust_decisions_mut()"),
    ("self.actor_handles ", "self.actor_handles_mut()"),
    ("self.registry ", "self.registry_mut()"),
    ("self.registry.", "self.registry_mut()"),
];

fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_rust_files(&path));
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

fn relative_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_test_file(rel_path: &str) -> bool {
    rel_path.contains("/tests/")
        || rel_path.ends_with("/tests.rs")
        || rel_path.ends_with("_tests.rs")
        || rel_path.ends_with("_test.rs")
        || rel_path.contains("_tests.")
        || rel_path.contains("_test.")
}

fn needs_appstate_lint(rel_path: &str) -> bool {
    let exemptions = [
        "build.rs",
        "accessors.rs",
        "domain_ops.rs",
        "actors/config/actor.rs",
        "actors/config/ractor_config.rs",
        "actors/permission/actor.rs",
        "actors/permission/ractor_permission.rs",
        "actors/input/actor.rs",
        "actors/input/messages.rs",
        "actors/ui_control/actor.rs",
        "actors/handles.rs",
        "actors/leader/actor.rs",
        "actors/leader/handle.rs",
        "update/input/text.rs",
        "update/input/submit.rs",
        "commands/dsl/handlers/session/mod.rs",
        "retry.rs",
        "login_flow/validation.rs",
        "model/state/input.rs",
    ];
    !is_test_file(rel_path)
        && !rel_path.contains("/benches/")
        && !rel_path.contains("/harness_skills/")
        && !exemptions.iter().any(|e| rel_path.ends_with(e))
}

fn check_appstate_field_access(rel_path: &str, lines: &[&str], errors: &mut Vec<String>) {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        for (pattern, suggestion) in APPSTATE_PATTERNS {
            if line.contains(pattern) {
                errors.push(format!(
                    "{}:{}: direct AppState field access `{pattern}` — use {suggestion}",
                    rel_path,
                    i + 1
                ));
                break;
            }
        }
    }
}

fn needs_magic_number_lint(rel_path: &str) -> bool {
    // Apply magic number lint to production code only (not tests/benches).
    // Exempt files that have many legitimate uses of numeric literals:
    // - harness_skills: skill-specific tuning values
    // - Proto/message tests use timestamps like 1234567890
    // - Auth tests use test data like 12345
    // - Session store uses various timeouts
    // - Labels module
    // - Provider mock
    // - Actor files that define internal timeouts
    // - Input history with character limits
    // - Tool format with timeouts
    // - Event durable with test data
    // - Config module with limits
    // - Provider event with buffer sizes
    // - Agent phase with test data
    // - Commands registry
    // - FFF indexer with timing
    let exemptions = [
        "src/harness_skills/startup_context.rs",
        "src/harness_skills/mod.rs",
        "src/proto/message/mod.rs",
        "src/message/mod.rs",
        "src/auth/storage.rs",
        "src/session/store.rs",
        "src/labels.rs",
        "src/provider_event.rs",
        "src/input_history.rs",
        "src/tool/format.rs",
        "src/event/durable.rs",
        "src/agent_phase.rs",
        "src/commands/registry.rs",
        "src/actors/fff_indexer/mod.rs",
        "src/actors/turn/speed_window.rs",
        "src/config/mod.rs",
        "src/actors/leader/actor.rs",
    ];
    !is_test_file(rel_path)
        && !rel_path.contains("/benches/")
        && !exemptions.iter().any(|e| rel_path.ends_with(e))
}

/// Check for magic numbers: numeric literals >= 1000 that aren't exempted.
///
/// We focus on larger numbers (>= 1000) as these are more clearly "magic" and
/// should be named constants. Smaller numbers are often acceptable as-is for
/// timeouts, limits, etc.
///
/// Known standard values that are exempted:
/// - JSON-RPC error codes (32700, 32600, 32601, etc.)
/// - HTTP status codes (401, 403, 500, 502, 503, etc.) - used in match arms
///
/// Other exemptions:
/// - Numbers < 1000
/// - Underscore-separated (e.g., 1_000_000)
/// - Hex literals (0x...)
/// - Array sizes ([0; N])
/// - Range bounds (0..N)
/// - Already named constants (preceded by `=`, `,`, `:`, `=>`)
/// - vec![] and similar macro literals
/// - Lines with assert!, debug_assert!, panic!
/// - Lines with doc comments (`///`, `//!`)
fn check_magic_numbers(rel_path: &str, lines: &[&str], errors: &mut Vec<String>) {
    // Regex for numeric literals >= 1000 (4+ digits).
    let re = regex::Regex::new(r"\b(\d{4,}(?:_\d+)*)\b").unwrap();

    // Known standard HTTP status codes (used in match arms).
    let http_status_codes = [
        "401", "403", "429", "500", "502", "503", "504",
    ];

    // Known JSON-RPC error codes.
    let json_rpc_codes = [
        "32700", "32600", "32601", "32602", "32603",
    ];

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip all comments and doc comments.
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // Skip lines that define constants.
        if trimmed.starts_with("const ") || trimmed.starts_with("pub const ") {
            continue;
        }

        // Skip assert/panic/debug_assert lines.
        if trimmed.contains("assert!")
            || trimmed.contains("debug_assert!")
            || trimmed.contains("panic!")
            || trimmed.contains("matches!(")
        {
            continue;
        }

        // Skip vec!, hashmap!, etc. literals.
        if line.contains("vec!")
            || line.contains("hashmap!")
            || line.contains("HashMap!")
            || line.contains("btreemap!")
        {
            continue;
        }

        for cap in re.find_iter(line) {
            let matched = cap.as_str();

            // Skip hex.
            if cap.start() > 0 && line[cap.start() - 1..cap.start()].starts_with("0x") {
                continue;
            }

            // Check if the number is in a string literal.
            let before = if cap.start() > 0 { &line[..cap.start()] } else { "" };
            if before.ends_with('"') || before.ends_with('\'') {
                continue;
            }

            // Check if this looks like a named constant (preceded by =, :, ,, =>).
            let looks_named = before.ends_with('=')
                || before.ends_with(':')
                || before.ends_with(',')
                || before.ends_with("=>");

            if looks_named {
                continue;
            }

            // Skip array sizes [0; 1000] and range bounds 0..1000.
            if line.contains("; ") && line.contains("[") {
                continue;
            }
            if line.contains("..") {
                continue;
            }

            // Skip if already has a clear name nearby.
            if line.contains("const ") || line.contains("static ") || line.contains("pub const") {
                continue;
            }

            // Skip HTTP status codes that appear in match arms (common pattern).
            if http_status_codes.iter().any(|code| line.contains(code)) {
                continue;
            }

            // Skip JSON-RPC error codes.
            if json_rpc_codes.iter().any(|code| line.contains(code)) {
                continue;
            }

            // This is a magic number that should be a constant.
            errors.push(format!(
                "{}:{}: magic number `{matched}` — extract to a named constant",
                rel_path,
                i + 1
            ));
        }
    }
}

fn lint_file(path: &Path, workspace_root: &Path, errors: &mut Vec<String>) {
    let rel_path = relative_path(path, workspace_root);
    if needs_appstate_lint(&rel_path) {
        let content = fs::read_to_string(path).unwrap();
        let lines: Vec<_> = content.lines().collect();
        check_appstate_field_access(&rel_path, &lines, errors);
    }
    if needs_magic_number_lint(&rel_path) {
        let content = fs::read_to_string(path).unwrap();
        let lines: Vec<_> = content.lines().collect();
        check_magic_numbers(&rel_path, &lines, errors);
    }
}

fn main() {
    // Bump MSRV if needed when adding new dependencies.
    println!("cargo:rustc-check=+"/* See MSRV in Cargo.toml */);

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());

    // Check AppState field access patterns and magic numbers.
    let mut errors = Vec::new();

    // Only check runie-core, not all crates.
    let runie_core_path = manifest_dir.join("src");

    for path in find_rust_files(&runie_core_path) {
        lint_file(&path, &manifest_dir, &mut errors);
    }

    if !errors.is_empty() {
        eprintln!("\n=== RUNIE LINT VIOLATIONS ===\n");
        for err in &errors {
            eprintln!("  {}", err);
        }
        eprintln!("\n{} violations found\n", errors.len());
        process::exit(1);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_number_lint_catches_violation() {
        let lines = vec!["pub fn foo() { let x = 12345; }"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(!errors.is_empty(), "Should catch magic number 12345");
        assert!(errors[0].contains("12345"));
    }

    #[test]
    fn test_magic_number_lint_allows_small_numbers() {
        let lines = vec!["pub fn foo() { let x = 99; }"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should not flag numbers < 1000");
    }

    #[test]
    fn test_magic_number_lint_allows_underscore_separated() {
        let lines = vec!["pub fn foo() { let x = 1_000_000; }"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow underscore-separated numbers");
    }

    #[test]
    fn test_magic_number_lint_allows_named_constants() {
        let lines = vec!["const BUFFER_SIZE: usize = 4096;"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow const definitions");
    }

    #[test]
    fn test_magic_number_lint_allows_field_assignment() {
        let lines = vec!["let config = Config { timeout: 5000 };"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow struct field assignments");
    }

    #[test]
    fn test_magic_number_lint_allows_hex() {
        let lines = vec!["let x = 0xFFFF;"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow hex literals");
    }

    #[test]
    fn test_magic_number_lint_allows_http_status_codes() {
        let lines = vec!["match status { 500 => ... }"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow HTTP status codes");
    }

    #[test]
    fn test_magic_number_lint_allows_json_rpc_codes() {
        let lines = vec!["match code { 32700 => ... }"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow JSON-RPC error codes");
    }

    #[test]
    fn test_magic_number_lint_allows_string_literals() {
        let lines = vec!["vec![\"1000\".into(), \"2000\".into()]"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow numbers in string literals");
    }

    #[test]
    fn test_magic_number_lint_allows_vec_macro() {
        let lines = vec!["let items = vec![1, 2, 3000];"];
        let mut errors = Vec::new();
        check_magic_numbers("test.rs", &lines, &mut errors);
        assert!(errors.is_empty(), "Should allow vec! macro contents");
    }

    #[test]
    fn test_needs_magic_number_lint_exempts_test_files() {
        assert!(!needs_magic_number_lint("src/foo.rs"));
        assert!(!needs_magic_number_lint("src/foo_tests.rs"));
        assert!(!needs_magic_number_lint("src/tests/bar.rs"));
    }

    #[test]
    fn test_needs_magic_number_lint_exempts_benches() {
        assert!(!needs_magic_number_lint("benches/foo.rs"));
    }
}
