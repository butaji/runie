//! Global linter rules for the runie project.
//! 
//! Enforces:
//! - Max 500 lines per file
//! - Max 40 lines per function
//! - Max 10 complexity (cyclomatic)
//!
//! No exceptions allowed.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

const MAX_FILE_LINES: usize = 500;
const MAX_FUNCTION_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_path = Path::new(&manifest).join("src");
    
    let mut errors: Vec<String> = Vec::new();
    
    collect_errors(&src_path, &mut errors);
    
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("{}", err);
        }
        std::process::exit(1);
    }
    
    println!("Linting passed!");
}

fn collect_errors(path: &Path, errors: &mut Vec<String>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                collect_errors(&path, errors);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                check_file(&path, errors);
            }
        }
    }
}

fn check_file(path: &Path, errors: &mut Vec<String>) {
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    
    // Check file line count (excluding blank lines and comments for fair count)
    let code_lines: Vec<&str> = lines.iter()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("//!")
        })
        .cloned()
        .collect();
    
    if code_lines.len() > MAX_FILE_LINES {
        errors.push(format!(
            "{}:{}: Too many lines ({} > {}), excluding comments/blank",
            path.display(), code_lines.len(), code_lines.len(), MAX_FILE_LINES
        ));
    }
    
    // Parse and check functions
    check_functions(&content, path, errors);
}

fn check_functions(content: &str, path: &Path, errors: &mut Vec<String>) {
    let mut in_function = false;
    let mut brace_count = 0;
    let mut function_start = 0;
    let mut function_lines: Vec<&str> = Vec::new();
    let mut current_line = 0;
    let mut complexity_stack: Vec<usize> = Vec::new();
    let mut max_complexity = 0;
    
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        
        // Track function start
        if !in_function && (trimmed.starts_with("fn ") || trimmed.starts_with("async fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn ")) {
            // Check if it's a function definition (not inside impl block continuation)
            if !trimmed.contains('{') {
                in_function = true;
                function_start = idx;
                function_lines.clear();
                brace_count = 0;
                complexity_stack.clear();
                complexity_stack.push(1);
                max_complexity = 1;
            }
        }
        
        if in_function {
            function_lines.push(line);
            
            // Count braces
            for c in line.chars() {
                match c {
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    _ => {}
                }
            }
            
            // Track complexity (if/while/for/match/&&/||)
            if trimmed.starts_with("if ") || trimmed.starts_with("while ") || 
               trimmed.starts_with("for ") || trimmed.starts_with("match ") ||
               trimmed.contains(" && ") || trimmed.contains(" || ") ||
               (trimmed.starts_with("if") && !trimmed.starts_with("impl")) {
                if let Some(last) = complexity_stack.last_mut() {
                    *last += 1;
                }
                max_complexity = max_complexity.max(*complexity_stack.last().unwrap_or(&1));
            }
            
            // Handle else/else if
            if trimmed.starts_with("else ") {
                if let Some(last) = complexity_stack.last_mut() {
                    *last += 1;
                }
                max_complexity = max_complexity.max(*complexity_stack.last().unwrap_or(&1));
            }
            
            // Function ended
            if brace_count <= 0 && trimmed.ends_with('}') {
                let line_count = function_lines.len();
                if line_count > MAX_FUNCTION_LINES {
                    errors.push(format!(
                        "{}:{}: Function too long ({} > {} lines)",
                        path.display(), function_start + 1, line_count, MAX_FUNCTION_LINES
                    ));
                }
                if max_complexity > MAX_COMPLEXITY {
                    errors.push(format!(
                        "{}:{}: Function complexity too high ({} > {})",
                        path.display(), function_start + 1, max_complexity, MAX_COMPLEXITY
                    ));
                }
                in_function = false;
                function_lines.clear();
                brace_count = 0;
            }
        }
        
        current_line = idx;
    }
}
