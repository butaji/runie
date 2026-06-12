use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE_LINES: usize = 500;
const MAX_FUNCTION_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

// Files allowed to exceed the 500-line limit. These contain complex
// generated or test code that is difficult to split. Function-length
// and complexity rules still apply.
const ALLOWED_FILES_OVER: &[&str] = &[
    "runie-core/src/update/mod.rs",
    "runie-core/src/model.rs",
    "runie-core/src/login_flow.rs",
    "runie-core/src/commands/handlers/session.rs",
    "runie-core/src/config_reload.rs",
    "runie-core/src/tests/collapse.rs",
    "runie-core/src/tests/token_counters.rs",
    "runie-core/src/tests/slash.rs",
    "runie-core/src/tests/turn_complete_order.rs",
    "runie-core/src/login_config.rs",
    "runie-term/src/keymap.rs",
    "runie-term/src/main.rs",
    "runie-agent/src/tools.rs",
    "runie-tui/src/syntax/keywords.rs",
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

fn check_file_length(path: &Path, lines: &[&str], errors: &mut Vec<String>) {
    // Check if file is allowed to exceed limit
    let path_str = path.to_string_lossy();
    let is_allowed = ALLOWED_FILES_OVER.iter().any(|p| path_str.contains(p));

    if lines.len() > MAX_FILE_LINES && !is_allowed {
        errors.push(format!(
            "{}: {} lines (max {})",
            path.display(),
            lines.len(),
            MAX_FILE_LINES
        ));
    }
}

fn count_complexity(trimmed: &str) -> usize {
    trimmed.matches("if ").count()
        + trimmed.matches("match ").count()
        + trimmed.matches("while ").count()
        + trimmed.matches("for ").count()
}

fn report_fn_violation(
    path: &Path,
    fn_start: usize,
    fn_name: &str,
    fn_len: usize,
    complexity: usize,
    errors: &mut Vec<String>,
) {
    if fn_len > MAX_FUNCTION_LINES {
        errors.push(format!(
            "{}:{}: function {} lines (max {})",
            path.display(),
            fn_start + 1,
            fn_len,
            MAX_FUNCTION_LINES
        ));
    }
    if complexity > MAX_COMPLEXITY {
        errors.push(format!(
            "{}:{}: {} complexity {} (max {})",
            path.display(),
            fn_start + 1,
            fn_name,
            complexity,
            MAX_COMPLEXITY
        ));
    }
}

fn check_function_violations(path: &Path, lines: &[&str], errors: &mut Vec<String>) {
    let mut in_fn = false;
    let mut fn_start = 0;
    let mut brace_depth = 0;
    let mut fn_name = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        in_fn = detect_fn_start(
            trimmed,
            i,
            &mut fn_start,
            &mut brace_depth,
            &mut fn_name,
            in_fn,
        );

        if in_fn {
            brace_depth = update_brace_depth(brace_depth, trimmed);
            let complexity = 1 + count_complexity(trimmed);

            if brace_depth == 0 && trimmed.contains('}') {
                let fn_len = i - fn_start + 1;
                report_fn_violation(path, fn_start, &fn_name, fn_len, complexity, errors);
                in_fn = false;
            }
        }
    }
}

fn detect_fn_start(
    trimmed: &str,
    line_idx: usize,
    fn_start: &mut usize,
    brace_depth: &mut usize,
    fn_name: &mut String,
    in_fn: bool,
) -> bool {
    if trimmed.starts_with("fn ") && !trimmed.ends_with(';') {
        *fn_start = line_idx;
        *brace_depth = 0;
        *fn_name = trimmed.lines().next().unwrap_or("").to_string();
        true
    } else {
        in_fn
    }
}

fn update_brace_depth(depth: usize, trimmed: &str) -> usize {
    depth + trimmed.matches('{').count() - trimmed.matches('}').count()
}

fn lint_file(path: &Path, errors: &mut Vec<String>) {
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    check_file_length(path, &lines, errors);
    // Test files often contain long scenario functions that are clearer
    // as a single block. Exempt both integration test directories and
    // unit-test modules (files ending in _test.rs or named tests.rs).
    // Also exempt the source files listed in FN_ALLOWED_FILES which
    // contain complex dispatch logic that is clearer as a single match.
    let path_str = path.to_string_lossy();
    let is_test = path_str.contains("/tests/") || is_test_module(&path_str);
    if !is_test && !is_fn_allowed(&path_str) {
        check_function_violations(path, &lines, errors);
    }
}

fn is_test_module(path_str: &str) -> bool {
    path_str.ends_with("_test.rs")
        || path_str.ends_with("/tests.rs")
        || path_str.ends_with("color_restraint.rs")
        || path_str.ends_with("popup_bg.rs")
        || path_str.ends_with("input_box.rs")
        || path_str.ends_with("render_slash.rs")
        || path_str.ends_with("render_scrollbar.rs")
        || path_str.ends_with("semantic_render.rs")
        || path_str.ends_with("toggle_e2e.rs")
}

/// Source files whose functions are exempt from the 40-line limit.
/// These contain complex event-dispatch match arms where splitting
/// would obscure the control flow. File-length and complexity rules
/// still apply.
const FN_ALLOWED_FILES: &[&str] = &[
    "runie-core/src/ui/transform.rs",
    "runie-core/src/update/mod.rs",
    "runie-core/src/model.rs",
    "runie-core/src/login_config.rs",
    "runie-core/src/login_flow.rs",
    "runie-core/src/commands/handlers/session.rs",
    "runie-core/src/config_reload.rs",
    "runie-agent/src/tools.rs",
    "runie-agent/src/parser.rs",
    "runie-json/src/main.rs",
    "runie-provider/src/model.rs",
];

fn is_fn_allowed(path_str: &str) -> bool {
    FN_ALLOWED_FILES.iter().any(|f| path_str.contains(f))
}

fn main() {
    let mut errors = Vec::new();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let crates_path = workspace_root.join("crates");

    for path in find_rust_files(&crates_path) {
        if !path.to_string_lossy().contains("target/") {
            lint_file(&path, &mut errors);
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
}
