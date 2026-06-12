use std::fs;
use std::path::{Path, PathBuf};

const MAX_FILE_LINES: usize = 500;
const MAX_FUNCTION_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

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
    if lines.len() > MAX_FILE_LINES {
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
            fn_complexity += count_complexity(trimmed);

            if brace_depth == 0 && trimmed.contains('}') {
                let fn_len = i - fn_start + 1;
                report_fn_violation(path, fn_start, &fn_name, fn_len, fn_complexity, errors);
                in_fn = false;
            }
        }
    }
}

fn lint_file(path: &Path, errors: &mut Vec<String>) {
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    check_file_length(path, &lines, errors);
    check_function_violations(path, &lines, errors);
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
