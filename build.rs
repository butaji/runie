use std::fs;
use std::path::Path;

// Workspace lint thresholds. Tuned to the structural shape of the code:
// verb conjugation tables, render functions, provider adapters, and the
// harness runner are legitimately larger than the original 40/10 caps.
const MAX_FILE_LINES: usize = 500;
const MAX_FUNCTION_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

const ALLOWED_FILES_OVER: &[&str] = &[
    "crates/runie-core/src/update/mod.rs",
    "crates/runie-core/src/model.rs",
];
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

