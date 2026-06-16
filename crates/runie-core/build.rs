use std::fs;
use std::path::{Path, PathBuf};

// Strict lint thresholds — no exceptions
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

fn relative_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn check_file_length(rel_path: &str, lines: usize, errors: &mut Vec<String>) {
    if lines > MAX_FILE_LINES {
        errors.push(format!(
            "{}: {} lines (max {})",
            rel_path,
            lines,
            MAX_FILE_LINES
        ));
    }
}

fn count_complexity(trimmed: &str) -> usize {
    trimmed.matches("if ").count()
        + trimmed.matches("else if").count()
        + trimmed.matches("match ").count()
        + trimmed.matches("while ").count()
        + trimmed.matches("for ").count()
        + trimmed.matches("&&").count()
        + trimmed.matches("||").count()
        + trimmed.matches('?').count()
}

fn is_function_start(trimmed: &str) -> Option<String> {
    if trimmed.ends_with(';') {
        return None;
    }
    let mut tokens = trimmed.split_whitespace().peekable();
    loop {
        match tokens.peek().copied() {
            Some("pub") | Some("pub(crate)") | Some("pub(super)") | Some("crate") => {
                tokens.next();
            }
            Some("async") | Some("const") | Some("unsafe") | Some("static") => {
                tokens.next();
            }
            Some("fn") => {
                tokens.next();
                return tokens.next().filter(|name| !name.starts_with('(')).map(String::from);
            }
            _ => return None,
        }
    }
}

fn report_fn_violation(
    rel_path: &str,
    fn_start: usize,
    fn_name: &str,
    fn_len: usize,
    complexity: usize,
    errors: &mut Vec<String>,
) {
    if fn_len > MAX_FUNCTION_LINES {
        errors.push(format!(
            "{}:{}: function {} lines (max {})",
            rel_path,
            fn_start + 1,
            fn_len,
            MAX_FUNCTION_LINES
        ));
    }
    if complexity > MAX_COMPLEXITY {
        errors.push(format!(
            "{}:{}: {} complexity {} (max {})",
            rel_path,
            fn_start + 1,
            fn_name,
            complexity,
            MAX_COMPLEXITY
        ));
    }
}

struct FnTracker {
    in_fn: bool,
    in_fn_body: bool,
    fn_start: usize,
    brace_depth: usize,
    fn_complexity: usize,
    fn_name: String,
}

impl Default for FnTracker {
    fn default() -> Self {
        Self {
            in_fn: false,
            in_fn_body: false,
            fn_start: 0,
            brace_depth: 0,
            fn_complexity: 0,
            fn_name: String::new(),
        }
    }
}

impl FnTracker {
    fn start(&mut self, i: usize, fn_name: String) {
        self.in_fn = true;
        self.in_fn_body = false;
        self.fn_start = i;
        self.fn_complexity = 1;
        self.fn_name = fn_name;
    }

    fn update(&mut self, trimmed: &str) {
        let opens = trimmed.matches('{').count();
        let closes = trimmed.matches('}').count();
        self.brace_depth = self.brace_depth.saturating_add(opens);
        self.brace_depth = self.brace_depth.saturating_sub(closes);
        if opens > 0 {
            self.in_fn_body = true;
        }
    }

    fn ended(&self, trimmed: &str) -> bool {
        self.in_fn_body && self.brace_depth == 0 && trimmed.contains('}')
    }

    fn report_and_reset(&mut self, rel_path: &str, i: usize, errors: &mut Vec<String>) {
        let fn_len = i - self.fn_start + 1;
        report_fn_violation(
            rel_path,
            self.fn_start,
            &self.fn_name,
            fn_len,
            self.fn_complexity,
            errors,
        );
        self.in_fn = false;
        self.in_fn_body = false;
        self.fn_complexity = 0;
        self.fn_name.clear();
    }
}

fn check_function_violations(rel_path: &str, lines: &[&str], errors: &mut Vec<String>) {
    let mut tracker = FnTracker::default();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if !tracker.in_fn {
            if let Some(fn_name) = is_function_start(trimmed) {
                tracker.start(i, fn_name);
            }
        }

        if tracker.in_fn {
            tracker.update(trimmed);
            tracker.fn_complexity += count_complexity(trimmed);

            if tracker.ended(trimmed) {
                tracker.report_and_reset(rel_path, i, errors);
            }
        }
    }
}

fn lint_file(path: &Path, workspace_root: &Path, errors: &mut Vec<String>) {
    let rel_path = relative_path(path, workspace_root);
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    check_file_length(&rel_path, lines.len(), errors);
    check_function_violations(&rel_path, &lines, errors);
}

fn main() {
    if std::env::var("RUNIE_SKIP_BUILD_CHECKS").is_ok() {
        return;
    }

    let mut errors = Vec::new();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let crates_path = workspace_root.join("crates");

    for path in find_rust_files(&crates_path) {
        if !path.to_string_lossy().contains("target/") {
            lint_file(&path, workspace_root, &mut errors);
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
