//! Build script for runie-core.
//!
//! This script performs checks:
//! **AppState field access guardrail**: ensures internal AppState fields are accessed
//! through accessors, not directly.
//!
//! **Magic number guardrail**: prevents raw numeric literals (>= 1000) in production code.
//! Small numbers (0-9), underscore-separated, hex, and test code are exempt.
//!
//! **Orphan spawn guardrail**: ensures all `tokio::spawn` calls have their JoinHandle
//! captured (stored in a variable or passed to an owner). Fire-and-forget spawns are
//! flagged in production code. Exemptions:
//! - Test files (`#[cfg(test)]` or `tests/` directories)
//! - `#[allow(unused_mut)]` on the function containing the spawn
//! - Files with explicit exemptions (see `SPAWN_EXEMPTIONS`)
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
    // state.xxx patterns
    ("state.session.", "state.session()"),
    ("state.input.", "state.input()"),
    ("state.agent.", "state.agent_state()"),
    ("state.view.", "state.view()"),
    ("state.config.", "state.config()"),
    ("state.completion.", "state.completion()"),
    ("state.should_quit ", "state.should_quit_mut()"),
    ("state.open_dialog ", "state.open_dialog_mut()"),
    ("state.open_dialog.", "state.open_dialog_mut()"),
    ("state.dialog_back_stack.", "state.dialog_back_stack_mut()"),
    ("state.login_flow ", "state.login_flow_mut()"),
    ("state.transient_message ", "state.transient_message_mut()"),
    ("state.transient_until ", "state.transient_until_mut()"),
    ("state.transient_level ", "state.transient_level_mut()"),
    ("state.fff_file_results.", "state.fff_file_results()"),
    ("state.fff_debounce ", "state.fff_debounce_mut()"),
    ("state.perm_req ", "state.permission_request_opt()"),
    ("state.cwd_name ", "state.cwd_name_mut()"),
    ("state.git_info ", "state.git_info_mut()"),
    ("state.git_info.", "state.git_info_mut()"),
    ("state.skills ", "state.skills_mut()"),
    ("state.prompts ", "state.prompts_mut()"),
    ("state.trust_decisions ", "state.trust_decisions_mut()"),
    ("state.actor_handles ", "state.actor_handles_mut()"),
    ("state.registry ", "state.registry_mut()"),
    ("state.registry.", "state.registry_mut()"),
    ("state.turn_state.", "state.turn_state()"),
    // self.xxx patterns
    ("self.session.", "self.session()"),
    ("self.input.", "self.input()"),
    ("self.agent.", "self.agent_state()"),
    ("self.view.", "self.view()"),
    ("self.config.", "self.config()"),
    ("self.completion.", "self.completion()"),
    ("self.should_quit ", "self.should_quit_mut()"),
    ("self.open_dialog ", "self.open_dialog_mut()"),
    ("self.open_dialog.", "self.open_dialog_mut()"),
    ("self.dialog_back_stack.", "self.dialog_back_stack_mut()"),
    ("self.login_flow ", "self.login_flow_mut()"),
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
    ("self.actor_handles ", "self.actor_handles_mut()"),
    ("self.registry ", "self.registry_mut()"),
    ("self.registry.", "self.registry_mut()"),
    ("self.turn_state.", "self.turn_state()"),
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
        "commands/dsl/handlers/session/mod.rs",
        "retry.rs",
        "login_flow/validation.rs",
        "model/state/input.rs",
        // Test helper files that use direct turn_state access for test setup
        "update/input/tests.rs",
        "tests/flow.rs",
        "tests/queue.rs",
        "tests/vim_mode.rs",
        // TurnActor handlers access TurnActorState.turn_state (different from AppState.turn_state)
        "actors/turn/handlers.rs",
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
    let http_status_codes = ["401", "403", "429", "500", "502", "503", "504"];

    // Known JSON-RPC error codes.
    let json_rpc_codes = ["32700", "32600", "32601", "32602", "32603"];

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
            let before = if cap.start() > 0 {
                &line[..cap.start()]
            } else {
                ""
            };
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

// ── Orphan spawn guardrail ───────────────────────────────────────────────────────

/// Files exempt from the orphan spawn check.
///
/// These files may contain fire-and-forget spawns that are:
/// - Actor files where spawns are part of actor lifecycle
/// - Intentionally fire-and-forget (background services, abort signals)
/// - Properly managed through other means (channel-based, etc.)
/// - Test-only patterns
const SPAWN_EXEMPTIONS: &[&str] = &[
    // runie-core actor files - spawns are part of actor lifecycle and observed via actor shutdown
    "crates/runie-core/src/actors/config/handlers.rs",
    "crates/runie-core/src/actors/config/ractor_config.rs",
    "crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs",
    "crates/runie-core/src/actors/io/ractor_io.rs",
    "crates/runie-core/src/actors/leader/actor.rs",
    "crates/runie-core/src/actors/leader/test_helpers.rs",
    "crates/runie-core/src/actors/permission/ractor_permission.rs",
    "crates/runie-core/src/actors/session/ractor_session_actor.rs",
    "crates/runie-core/src/actors/session/session_handlers.rs",
    // runie-core other files with intentional fire-and-forget spawns
    "crates/runie-core/src/config/config_impl.rs", // Config watching - background service
    "crates/runie-core/src/session/store.rs",      // Session persistence - background service
    "crates/runie-core/src/tool/format.rs",        // Tool formatting - background service
    "crates/runie-core/src/update/system.rs", // Abort signal routing - fire-and-forget by design
    "crates/runie-core/src/shell.rs",         // Process execution - observed via channel
    "crates/runie-core/src/tool/cache.rs",    // Background TTL eviction - runs to process exit
    "crates/runie-core/src/bus.rs",           // Test bus implementation
    // runie-tui spawns - managed by application lifecycle and shutdown signals
    "crates/runie-tui/src/bootstrap.rs", // Bootstrap spawns - managed by app lifecycle
    "crates/runie-tui/src/ui_actor/mod.rs", // Event forwarder - fire-and-forget by design
    "crates/runie-tui/src/ui_actor/effects.rs", // Effect spawns - fire-and-forget via channels
    "crates/runie-tui/src/keymap.rs",    // Debug logging - fire-and-forget by design
    // runie-cli spawns - managed by server lifecycle
    "crates/runie-cli/src/server.rs", // Connection handler spawns - managed by listener
    // runie-agent spawns - managed by actor lifecycle
    "crates/runie-agent/src/subagent.rs", // Response accumulation - fire-and-forget via channel
    "crates/runie-agent/src/actor/mod.rs", // Agent turn spawn - managed by actor lifecycle
];

fn needs_spawn_lint(rel_path: &str) -> bool {
    // Skip test files (already covered by other lints) and exempted files.
    // Return true if we SHOULD lint (i.e., skip test files and exemptions).
    !is_test_file(rel_path) && !SPAWN_EXEMPTIONS.iter().any(|e| rel_path.ends_with(e))
}

/// Check for orphan tokio::spawn calls that don't capture the JoinHandle.
///
/// A spawn is "orphan" if:
/// - The JoinHandle is not assigned to a variable
/// - The JoinHandle is not passed as an argument to a function
/// - The JoinHandle is not stored in a struct field
///
/// Valid capture patterns:
/// - `let handle = tokio::spawn(...)` - OK (named capture)
/// - `let _handle = tokio::spawn(...)` - OK (underscore-prefixed capture)
/// - `spawn_unchecked(async { ... })` - OK if JoinHandle captured
/// - Comments like `// fire-and-forget` - OK
/// - `#[allow(unused_mut)]` on containing function - OK
///
/// INVALID (will be flagged):
/// - `let _ = tokio::spawn(...)` - explicit discard is NOT a valid capture
fn check_orphan_spawns(rel_path: &str, content: &str, errors: &mut Vec<String>) {
    /// Number of lines to look back when searching for `#[allow(...)]` above a spawn.
    const SPAWN_ALLOW_LOOKBACK: usize = 10;

    let lines: Vec<_> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip comments.
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // Check for tokio::spawn pattern.
        // We look for "spawn" followed by "(" to catch tokio::spawn, spawn, spawn_unchecked, etc.
        if !line.contains("spawn") || !line.contains('(') {
            continue;
        }

        // Check if this is a spawn call.
        // Pattern: identifier containing "spawn" followed by "("
        let spawn_patterns = [
            "tokio::spawn",
            "::tokio::spawn",
            "spawn_blocking",
            "spawn_unchecked",
        ];

        let is_spawn = spawn_patterns.iter().any(|p| line.contains(p));
        if !is_spawn {
            continue;
        }

        // Check if JoinHandle is captured.
        // Patterns that indicate capture:
        // 1. `let <name> = tokio::spawn(...)`
        // 2. `let _ = tokio::spawn(...)`
        // 3. Function argument: `foo(tokio::spawn(...))`
        // 4. Struct field: `field: tokio::spawn(...)`
        // 5. Array element: `[tokio::spawn(...)]`

        // Check for capture patterns.
        // NOTE: `let _ = tokio::spawn(...)` is NOT allowed - explicit discard still
        // represents an unobserved spawn that could leak on panic/crash.
        // Valid capture patterns:
        // - `let handle = tokio::spawn(...)` - named capture
        // - `let _handle = tokio::spawn(...)` - underscore-prefixed capture
        // - Spawn as function argument, struct field, array element, Some(), vec![]
        let has_let_spawn = (line.contains("let ") || line.contains("let_"))
            && (line.contains("= tokio::spawn")
                || line.contains("= ::tokio::spawn")
                || line.contains("= spawn_blocking")
                || line.contains("= spawn_unchecked"));
        let is_explicit_discard = line.contains("let _ = tokio::spawn")
            || line.contains("let _ = ::tokio::spawn")
            || line.contains("let _ = spawn_blocking")
            || line.contains("let _ = spawn_unchecked");
        let has_valid_capture = has_let_spawn && !is_explicit_discard;
        let has_other_capture = line.contains(", tokio::spawn")
            || line.contains("tokio::spawn,")
            || line.contains("field: tokio::spawn")
            || line.contains("field: spawn_blocking")
            || line.contains("Some(tokio::spawn")
            || line.contains("Some(spawn_blocking")
            || line.contains("vec![tokio::spawn")
            || line.contains("vec![spawn_blocking");
        let has_capture = has_valid_capture || has_other_capture;

        if has_capture {
            continue;
        }

        // Check for fire-and-forget comment.
        let fire_and_forget = trimmed.contains("fire-and-forget")
            || trimmed.contains("fire_forget")
            || trimmed.contains("ff");
        if fire_and_forget {
            continue;
        }

        // This appears to be an orphan spawn.
        // Check if the containing function has #[allow(unused_mut)].
        let has_allow_unused = lines[..i.min(SPAWN_ALLOW_LOOKBACK)]
            .iter()
            .rev()
            .take_while(|l| !l.contains("fn ") && !l.contains("pub fn "))
            .any(|l| l.contains("#[allow(unused") || l.contains("#[allow(unused_mut)"));

        if has_allow_unused {
            continue;
        }

        // Flag as violation.
        errors.push(format!(
            "{}:{}: orphan `tokio::spawn` — capture JoinHandle or document with `// fire-and-forget`",
            rel_path,
            i + 1
        ));
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
    if needs_spawn_lint(&rel_path) {
        let content = fs::read_to_string(path).unwrap();
        check_orphan_spawns(&rel_path, &content, errors);
    }
}

fn main() {
    // Bump MSRV if needed when adding new dependencies.
    println!("cargo:rustc-check=+" /* See MSRV in Cargo.toml */);

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());

    // Check AppState field access patterns and magic numbers.
    let mut errors = Vec::new();

    // Scan all crates in the workspace for violations.
    // Build script is in crates/runie-core/, so we need to find the workspace root.
    let workspace_root = manifest_dir
        .parent() // crates/runie-core
        .and_then(|p| p.parent()) // crates
        .and_then(|p| p.parent()) // workspace root
        .unwrap_or(&manifest_dir);

    // Find all Cargo.toml files to identify workspace crates.
    let crates_path = workspace_root.join("crates");
    for entry in fs::read_dir(&crates_path).unwrap_or_else(|_| fs::read_dir(".").unwrap()) {
        let entry = entry.unwrap();
        let crate_path = entry.path();
        if crate_path.is_dir() {
            let src_path = crate_path.join("src");
            if src_path.exists() {
                for path in find_rust_files(&src_path) {
                    lint_file(&path, workspace_root, &mut errors);
                }
            }
        }
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
        assert!(
            errors.is_empty(),
            "Should allow underscore-separated numbers"
        );
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

    // ── Orphan spawn lint tests ─────────────────────────────────────────────────

    #[test]
    fn test_spawn_lint_catches_orphan_spawn() {
        let content = r#"
            pub async fn foo() {
                tokio::spawn(async { });
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(!errors.is_empty(), "Should catch orphan tokio::spawn");
        assert!(errors[0].contains("orphan"));
    }

    #[test]
    fn test_spawn_lint_allows_captured_handle() {
        let content = r#"
            pub async fn foo() {
                let handle = tokio::spawn(async { });
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(errors.is_empty(), "Should allow JoinHandle capture");
    }

    #[test]
    fn test_spawn_lint_allows_underscore_prefixed_capture() {
        let content = r#"
            pub async fn foo() {
                let _handle = tokio::spawn(async { });
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(
            errors.is_empty(),
            "Should allow underscore-prefixed capture"
        );
    }

    #[test]
    fn test_spawn_lint_rejects_explicit_discard() {
        let content = r#"
            pub async fn foo() {
                let _ = tokio::spawn(async { });
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(
            !errors.is_empty(),
            "Explicit discard `let _ = tokio::spawn(...)` should be flagged"
        );
        assert!(errors[0].contains("orphan"));
    }

    #[test]
    fn test_spawn_lint_allows_fire_and_forget_comment() {
        let content = r#"
            pub async fn foo() {
                // fire-and-forget: background cleanup
                tokio::spawn(async { });
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(errors.is_empty(), "Should allow fire-and-forget comment");
    }

    #[test]
    fn test_spawn_lint_allows_spawn_blocking_capture() {
        let content = r#"
            pub fn foo() {
                let handle = tokio::task::spawn_blocking(|| { });
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(
            errors.is_empty(),
            "Should allow spawn_blocking with capture"
        );
    }

    #[test]
    fn test_spawn_lint_allows_some_spawn() {
        let content = r#"
            pub async fn foo() {
                let handle = Some(tokio::spawn(async { }));
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(errors.is_empty(), "Should allow Some(tokio::spawn(...))");
    }

    #[test]
    fn test_spawn_lint_allows_function_argument() {
        let content = r#"
            pub async fn foo() {
                register_handle(tokio::spawn(async { }));
            }
        "#;
        let mut errors = Vec::new();
        check_orphan_spawns("src/foo.rs", content, &mut errors);
        assert!(errors.is_empty(), "Should allow spawn as function argument");
    }

    #[test]
    fn test_spawn_lint_allows_test_files() {
        // Test files should be exempt from spawn lint.
        assert!(
            !needs_spawn_lint("tests/foo.rs"),
            "test files should be exempt"
        );
        assert!(
            !needs_spawn_lint("src/foo_tests.rs"),
            "_tests.rs files should be exempt"
        );
        assert!(
            !needs_spawn_lint("src/foo_test.rs"),
            "_test.rs files should be exempt"
        );
    }

    #[test]
    fn test_spawn_lint_allows_exempted_files() {
        // Exempted files should be skipped.
        assert!(
            !needs_spawn_lint("src/update/system.rs"),
            "system.rs is exempted"
        );
        assert!(!needs_spawn_lint("src/shell.rs"), "shell.rs is exempted");
        assert!(
            !needs_spawn_lint("src/tool/cache.rs"),
            "cache.rs is exempted"
        );
    }

    #[test]
    fn test_spawn_lint_requires_lint_on_regular_files() {
        // Non-exempt production files should be linted.
        assert!(
            needs_spawn_lint("src/foo.rs"),
            "regular files should be linted"
        );
        assert!(
            needs_spawn_lint("src/bar/mod.rs"),
            "regular mod files should be linted"
        );
    }
}
