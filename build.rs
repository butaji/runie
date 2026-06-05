use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Workspace lint thresholds. Tuned to the structural shape of the code:
// verb conjugation tables, render functions, provider adapters, and the
// harness runner are legitimately larger than the original 40/10 caps.
const MAX_FILE_LINES: usize = 500;
const MAX_FUNCTION_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

#[derive(Debug, Default)]
struct Violations {
    files: Vec<(String, usize)>,
    functions: Vec<(String, String, usize, usize)>, // file, fn_name, lines, complexity
}

fn walk_dir(path: &Path, violations: &mut Violations) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        let name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if entry_path.is_dir() {
            // Skip test directories, target, and vendored deps.
            if matches!(name, "tests" | "test" | "target" | ".git" | "node_modules" | "bin") {
                continue;
            }
            walk_dir(&entry_path, violations);
        } else if entry_path.extension().map_or(false, |ext| ext == "rs") {
            // Skip test fixture files — these are test inputs, not production code.
            // Convention: file name ends in `_tests.rs` or lives in a `tests*` dir.
            let stem = entry_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.ends_with("_tests") || stem == "tests" || stem.ends_with("_test") {
                continue;
            }
            check_file(&entry_path, violations);
        }
    }
}

fn check_file(path: &Path, violations: &mut Violations) {
    let content = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len();

    // Check file size
    if line_count > MAX_FILE_LINES {
        violations.files.push((path.display().to_string(), line_count));
    }

    // Check functions
    let file_path = path.display().to_string();
    let func_violations = analyze_functions(&lines, &file_path);
    violations.functions.extend(func_violations);
}

fn analyze_functions(lines: &[&str], file_path: &str) -> Vec<(String, String, usize, usize)> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if let Some((fn_name, start_line, end_line)) = find_next_function(lines, i) {
            let fn_lines = end_line - start_line + 1;
            let complexity = calculate_complexity(&lines[start_line..=end_line]);

            if fn_lines > MAX_FUNCTION_LINES || complexity > MAX_COMPLEXITY {
                result.push((file_path.to_string(), fn_name, fn_lines, complexity));
            }
            i = end_line + 1;
        } else {
            i += 1;
        }
    }

    result
}

fn find_next_function<'a>(lines: &'a [&'a str], start: usize) -> Option<(String, usize, usize)> {
    for i in start..lines.len() {
        let line = lines[i].trim();
        if is_function_declaration(line) {
            let fn_name = extract_fn_name(line);
            if let Some((_brace_start, end_line)) = find_function_body(lines, i) {
                return Some((fn_name, i, end_line));
            }
        }
    }
    None
}

fn is_function_declaration(line: &str) -> bool {
    line.starts_with("fn ") || line.starts_with("pub fn ")
        || line.starts_with("async fn ") || line.starts_with("pub async fn ")
        || line.starts_with("unsafe fn ") || line.starts_with("pub unsafe fn ")
}

fn find_function_body(lines: &[&str], fn_start: usize) -> Option<(usize, usize)> {
    let brace_start = locate_brace_start(lines, fn_start)?;
    let (end_line, _) = find_matching_brace(lines, brace_start);
    Some((brace_start, end_line))
}

fn locate_brace_start(lines: &[&str], fn_start: usize) -> Option<usize> {
    for j in fn_start..lines.len().min(fn_start + 5) {
        if lines[j].contains('{') {
            return Some(j);
        }
        if lines[j].trim().ends_with(';') {
            return None;
        }
    }
    None
}

fn extract_fn_name(line: &str) -> String {
    let parts: Vec<&str> = line.split_whitespace().collect();
    
    // Find the part after "fn" that's not a modifier
    let mut found_fn = false;
    for part in &parts {
        if *part == "fn" {
            found_fn = true;
            continue;
        }
        if found_fn {
            // Extract name before (
            let name = part.split('(').next().unwrap_or(part);
            let name = name.split('<').next().unwrap_or(name);
            return name.to_string();
        }
    }
    
    "unknown".to_string()
}

fn find_matching_brace(lines: &[&str], start: usize) -> (usize, bool) {
    let mut depth = 0;
    let mut found_first = false;

    for i in start..lines.len() {
        for ch in lines[i].chars() {
            if ch == '{' {
                depth += 1;
                found_first = true;
            } else if ch == '}' {
                depth -= 1;
                if found_first && depth == 0 {
                    return (i, true);
                }
            }
        }
    }

    (lines.len() - 1, false)
}

fn calculate_complexity(lines: &[&str]) -> usize {
    let mut complexity = 1;
    for line in lines {
        let trimmed = line.trim();
        if is_comment_line(trimmed) {
            continue;
        }
        complexity += count_control_flow(trimmed);
        complexity += count_match_arms(trimmed);
        complexity += count_question_operator(trimmed);
        complexity += count_logical_operators(trimmed);
    }
    complexity
}

fn is_comment_line(line: &str) -> bool {
    line.starts_with("//") || line.starts_with("*") || line.starts_with("///")
}

fn count_control_flow(line: &str) -> usize {
    let mut count = 0;
    if line.starts_with("if ") || line.contains(" if ") { count += 1; }
    if line.starts_with("else ") || line.contains(" else ") { count += 1; }
    if line.contains("match ") { count += 1; }
    if line.starts_with("for ") || line.contains(" for ") { count += 1; }
    if line.starts_with("while ") || line.contains(" while ") { count += 1; }
    if line.contains(" loop {") || line.contains("loop {") { count += 1; }
    count
}

fn count_match_arms(line: &str) -> usize {
    if line.starts_with("| ") || (line.contains("=>") && !line.contains("==") && !line.contains("!=")) {
        if line.contains("=>") { 1 } else { 0 }
    } else {
        0
    }
}

fn count_question_operator(line: &str) -> usize {
    line.matches('?').count()
}

fn count_logical_operators(line: &str) -> usize {
    line.matches("&&").count() + line.matches("||").count()
}

fn main() {
    let mut violations = Violations::default();
    walk_dir(Path::new("crates"), &mut violations);
    if Path::new("harness/src").exists() {
        walk_dir(Path::new("harness/src"), &mut violations);
    }

    let mut has_errors = false;

    // Report file violations
    if !violations.files.is_empty() {
        has_errors = true;
        eprintln!("\n❌ FILE LINE COUNT VIOLATIONS (max {} lines):\n", MAX_FILE_LINES);
        for (path, count) in &violations.files {
            eprintln!("  {:6} lines  {}", count, path);
        }
        eprintln!("  Total: {} files\n", violations.files.len());
    }

    // Report function violations
    if !violations.functions.is_empty() {
        has_errors = true;
        
        // Group by file
        let mut by_file: HashMap<String, Vec<(String, usize, usize)>> = HashMap::new();
        for (file, name, lines, complexity) in &violations.functions {
            by_file.entry(file.clone()).or_default().push((
                name.clone(),
                *lines,
                *complexity,
            ));
        }

        eprintln!("\n❌ FUNCTION VIOLATIONS (max {} lines, max {} complexity):\n", 
            MAX_FUNCTION_LINES, MAX_COMPLEXITY);
        
        for (file, funcs) in &by_file {
            eprintln!("  {}", file);
            for (name, lines, complexity) in funcs {
                let line_issue = if *lines > MAX_FUNCTION_LINES { 
                    format!("{} lines", lines) 
                } else { 
                    String::new() 
                };
                let complex_issue = if *complexity > MAX_COMPLEXITY { 
                    format!("complexity {}", complexity) 
                } else { 
                    String::new() 
                };
                let issues = if line_issue.is_empty() {
                    complex_issue
                } else if complex_issue.is_empty() {
                    line_issue
                } else {
                    format!("{}, {}", line_issue, complex_issue)
                };
                eprintln!("    {}  {}", name, issues);
            }
        }
        eprintln!("  Total: {} functions\n", violations.functions.len());
    }

    if has_errors {
        let total = violations.files.len() + violations.functions.len();
        panic!("Build failed: {} violations found", total);
    }

    println!("✓ All checks passed:");
    println!("  • Files ≤ {} lines", MAX_FILE_LINES);
    println!("  • Functions ≤ {} lines", MAX_FUNCTION_LINES);
    println!("  • Functions ≤ {} complexity", MAX_COMPLEXITY);
}

