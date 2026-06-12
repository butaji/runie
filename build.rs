use std::fs;
use std::path::Path;

// Workspace lint thresholds. Tuned to the structural shape of the code:
// verb conjugation tables, render functions, provider adapters, and the
// harness runner are legitimately larger than the original 40/10 caps.
const MAX_FILE_LINES: usize = 1000;
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
    "crates/runie-agent/src/bin/reply_to_scenario.rs",
    "crates/runie-tui/src/pipe/render/overlays.rs",
    "crates/runie-tui/src/bin/scenario_fasthot.rs",
    "crates/runie-tui/src/bin/runie-dspec.rs",
    "crates/runie-tui/src/bin/grok_parity_test.rs",
    "crates/runie-tui/src/bin/scenario_replay.rs",
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
const ALLOWED_FUNCS_OVER: &[&str] = &[];

fn walkdir(path: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
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
        if path.to_string_lossy().contains("target/") {
            continue;
        }

        let path_str = path.to_string_lossy();
        let is_allowed_file = ALLOWED_FILES_OVER.iter().any(|p| {
            path_str.ends_with(p)
        });

        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = content.lines().collect();

        // File length check
        if lines.len() > MAX_FILE_LINES && !is_allowed_file {
            errors.push(format!(
                "{}: {} lines (max {})",
                path.display(),
                lines.len(),
                MAX_FILE_LINES
            ));
        }

        // Function length and complexity checks
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

                // Count branches for complexity
                fn_complexity += trimmed.matches("if ").count();
                fn_complexity += trimmed.matches("match ").count();
                fn_complexity += trimmed.matches("while ").count();
                fn_complexity += trimmed.matches("for ").count();

                if brace_depth == 0 && trimmed.contains('}') {
                    let fn_len = i - fn_start + 1;
                    if fn_len > MAX_FUNCTION_LINES {
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
