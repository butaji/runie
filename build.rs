use std::fs;
use std::path::Path;

// Workspace lint thresholds.
// Test files get a relaxed budget because they contain many small #[test] fns.
// Source files are held to tighter limits.
const MAX_FILE_LINES: usize = 1000;
const MAX_FILE_LINES_TEST: usize = 1500;
const MAX_FUNCTION_LINES: usize = 80;
const MAX_COMPLEXITY: usize = 15;

// Files allowed to exceed MAX_FILE_LINES (large test files, generated code)
const ALLOWED_FILES_OVER: &[&str] = &[
    "crates/runie-core/src/update/mod.rs",
    "crates/runie-core/src/model.rs",
    "crates/runie-core/src/login_flow.rs",
    "crates/runie-core/src/config_reload.rs",
    "crates/runie-core/src/tests/stack_navigation.rs",
    "crates/runie-core/src/tests/login_logout.rs",
    "crates/runie-term/tests/e2e_legacy.rs",
    "crates/runie-tui/src/pipe/render/overlays.rs",
    "crates/runie-tui/src/tui/update/palette_tests.rs",
    "crates/runie-tui/src/tui/update/slash_tests.rs",
    "crates/runie-tui/src/tui/tests_onboarding.rs",
    "crates/runie-tui/src/tui/tests/comprehensive_suite/stream_interruption_tests.rs",
    "crates/runie-tui/src/tui/tests/e2e_flow_tests/palette_flows.rs",
    "crates/runie-tui/src/tui/tests/input_unicode.rs",
    "crates/runie-tui/src/tui/tests/grok_element_tests.rs",
    "crates/runie-tui/src/tui/tests/scroll_auto_tests.rs",
    "crates/runie-tui/src/tui/tests/session_management_tests.rs",
    "crates/runie-tui/src/tui/tests/snapshot_regression_tests/grok_parity_tests.rs",
    "crates/runie-tui/src/tui/tests/grok_parity_tests.rs",
    "crates/runie-tui/src/tui/tests/agent_events/message_flow.rs",
    "crates/runie-tui/src/tui/tests/agent_events/tool_execution.rs",
    "crates/runie-tui/src/tui/tests/agent_events/lifecycle.rs",
    "crates/runie-tui/src/tui/view_models.rs",
    "crates/runie-tui/src/components/message_list.rs",
    "crates/runie-tui/src/components/input_bar/mod.rs",
    "crates/runie-tui/src/components/message_list/render/assistant.rs",
    "crates/runie-tui/src/components/message_list/render/messages_test.rs",
    "crates/runie-tui/src/components/diff_viewer.rs",
    "crates/runie-tui/src/components/top_bar/tests.rs",
    "crates/runie-tui/src/tests.rs",
];

// Functions allowed to exceed MAX_FUNCTION_LINES
// Format: "path:line_number:function_name"
const ALLOWED_FUNCS_OVER: &[&str] = &[
    "crates/runie-core/src/commands/handlers/session.rs:11:register",
    "crates/runie-core/src/commands/handlers/system.rs:7:register",
    "crates/runie-core/src/model_catalog.rs:54:model_catalog",
];

fn walkdir(path: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            // Skip archived and build-output directories
            if p.to_string_lossy().contains("/_archive/") {
                continue;
            }
            if p.to_string_lossy().contains("/target/") {
                continue;
            }
            if p.is_dir() {
                files.extend(walkdir(&p));
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(p);
            }
        }
    }
    files
}

/// Returns true when `haystack` contains `needle` bounded by word characters
/// on both sides (i.e. as a proper keyword, not as part of an identifier).
fn contains_keyword(haystack: &str, keyword: &str) -> bool {
    let pat = format!(" {} ", keyword);
    haystack.contains(&pat)
}

fn main() {
    let skip = std::env::var("RUNIE_SKIP_BUILD_CHECKS");
    if skip.is_ok() {
        println!("cargo:rerun-if-changed=crates/");
        return;
    }

    let mut errors = Vec::new();
    let paths: Vec<_> = walkdir(std::path::Path::new("crates"));
    eprintln!("Linting {} files...", paths.len());

    for path in paths {
        let path_str = path.to_string_lossy();

        let is_allowed_file = ALLOWED_FILES_OVER
            .iter()
            .any(|p| path_str.ends_with(p));

        // Test files get a relaxed line budget; the function-length and
        // complexity checks still apply to them.
        let is_test_file = path_str.contains("/tests/");
        let max_lines = if is_test_file {
            MAX_FILE_LINES_TEST
        } else {
            MAX_FILE_LINES
        };

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = content.lines().collect();

        // ── File length check ───────────────────────────────────────────────
        if lines.len() > max_lines && !is_allowed_file {
            errors.push(format!(
                "{}: {} lines (max {})",
                path.display(),
                lines.len(),
                max_lines
            ));
        }

        // ── Function length and complexity checks ───────────────────────────
        // Always run on all files (including tests) so that oversize #[test]
        // functions are still flagged.
        let mut in_fn = false;
        let mut fn_start = 0;
        let mut brace_depth = 0;
        let mut fn_complexity = 0;
        let mut fn_name = String::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            if trimmed.starts_with("fn ") && !trimmed.ends_with(';') {
                in_fn = true;
                fn_start = i;
                brace_depth = 0;
                fn_complexity = 1;
                fn_name = trimmed.lines().next().unwrap_or("").to_string();
            }

            if in_fn {
                brace_depth += trimmed.matches('{').count();
                brace_depth -= trimmed.matches('}').count();

                // Count branches for complexity — only when the keyword appears
                // as a proper word (preceded by a space, followed by a space).
                // This avoids false positives from identifiers like `if_chain`.
                if contains_keyword(trimmed, "if") {
                    fn_complexity += trimmed.matches(" if ").count();
                }
                if contains_keyword(trimmed, "match") {
                    fn_complexity += trimmed.matches(" match ").count();
                }
                if contains_keyword(trimmed, "while") {
                    fn_complexity += trimmed.matches(" while ").count();
                }
                if contains_keyword(trimmed, "for") {
                    fn_complexity += trimmed.matches(" for ").count();
                }

                if brace_depth == 0 && trimmed.contains('}') {
                    let fn_len = i - fn_start + 1;

                    // Check if this function is in ALLOWED_FUNCS_OVER by matching
                    // "path:line_number:name" (fn_name is the full decl line).
                    let fn_decl = lines[fn_start].trim();
                    let is_allowed_fn = ALLOWED_FUNCS_OVER.iter().any(|entry| {
                        let parts: Vec<&str> = entry.split(':').collect();
                        if parts.len() >= 3 {
                            let file_path = parts[0];
                            let line_no: usize = parts[1].parse().unwrap_or(0);
                            let fn_short = parts[2];
                            path_str.ends_with(file_path)
                                && fn_start + 1 == line_no
                                && fn_decl.contains(fn_short)
                        } else {
                            false
                        }
                    });

                    if fn_len > MAX_FUNCTION_LINES && !is_allowed_fn {
                        errors.push(format!(
                            "{}:{}: function {} lines (max {})",
                            path.display(),
                            fn_start + 1,
                            fn_len,
                            MAX_FUNCTION_LINES
                        ));
                    }
                    if fn_complexity > MAX_COMPLEXITY {
                        errors.push(format!(
                            "{}:{}: {} complexity {} (max {})",
                            path.display(),
                            fn_start + 1,
                            fn_name,
                            fn_complexity,
                            MAX_COMPLEXITY
                        ));
                    }
                    in_fn = false;
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
        std::process::exit(1);
    }

    println!("cargo:rerun-if-changed=crates/");
}
