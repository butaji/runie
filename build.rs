//! Global build-time linter for the runie project.
//! 
//! Rules:
//! - Max 500 lines per file (excluding blank lines and comments)
//! - Max 40 lines per function
//! - Max 10 complexity (cyclomatic)
//!
//! This runs during `cargo build` and will fail if any rules are violated.

use std::env;
use std::fs;
use std::path::Path;

const MAX_FILE_LINES: usize = 500;
const MAX_FUNC_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_path = Path::new(&manifest).join("src");
    
    let mut errors: Vec<String> = Vec::new();
    collect_errors(&src_path, &mut errors);
    
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("LINT ERROR: {}", err);
        }
        eprintln!("\nLint failed with {} error(s)", errors.len());
        std::process::exit(1);
    }
    
    println!("cargo:warning=Lint passed!");
}

fn collect_errors(path: &Path, errors: &mut Vec<String>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            if p.is_dir() {
                collect_errors(&p, errors);
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                check_file(&p, errors);
            }
        }
    }
}

fn check_file(path: &Path, errors: &mut Vec<String>) {
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    
    // Count code lines (exclude blank and comment-only lines)
    let code_lines: Vec<&str> = lines.iter()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() && 
            !trimmed.starts_with("//") && 
            !trimmed.starts_with("//!") &&
            !trimmed.starts_with("/*") &&
            !trimmed.ends_with("*/")
        })
        .cloned()
        .collect();
    
    if code_lines.len() > MAX_FILE_LINES {
        errors.push(format!(
            "{}:{}: file has too many lines ({} > {})",
            path.display(),
            code_lines.len(),
            code_lines.len(),
            MAX_FILE_LINES
        ));
    }
    
    // Check functions
    check_functions(&content, path, errors);
}

fn check_functions(content: &str, path: &Path, errors: &mut Vec<String>) {
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut line_starts: Vec<usize> = vec![0];
    for line in content.lines() {
        line_starts.push(line_starts.last().unwrap() + line.len() + 1);
    }
    
    while i < len {
        // Skip strings and comments
        if chars[i] == '/' && i + 1 < len {
            if chars[i + 1] == '/' {
                while i < len && chars[i] != '\n' { i += 1; }
                continue;
            }
            if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') { i += 1; }
                i += 2;
                continue;
            }
        }
        if chars[i] == '"' {
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' { i += 1; }
                i += 1;
            }
            i += 1;
            continue;
        }
        
        // Check for function start
        if is_fn_start(&chars, i) {
            let (func_end, func_lines, complexity) = parse_function(&chars, i, &line_starts);
            
            if func_lines > MAX_FUNC_LINES {
                let line_num = find_line_num(i, &line_starts);
                errors.push(format!(
                    "{}:{}: function has {} lines (max {})",
                    path.display(),
                    line_num,
                    func_lines,
                    MAX_FUNC_LINES
                ));
            }
            
            if complexity > MAX_COMPLEXITY {
                let line_num = find_line_num(i, &line_starts);
                errors.push(format!(
                    "{}:{}: complexity {} (max {})",
                    path.display(),
                    line_num,
                    complexity,
                    MAX_COMPLEXITY
                ));
            }
            
            i = func_end;
        } else {
            i += 1;
        }
    }
}

fn is_fn_start(chars: &[char], i: usize) -> bool {
    let keywords = ["fn ", "async fn ", "pub fn ", "pub async fn "];
    for kw in &keywords {
        let mut match_ = true;
        for (j, c) in kw.chars().enumerate() {
            if i + j >= chars.len() || chars[i + j] != c {
                match_ = false;
                break;
            }
        }
        if match_ { return true; }
    }
    false
}

fn parse_function(chars: &[char], start: usize, _line_starts: &[usize]) -> (usize, usize, usize) {
    let mut depth = 0;
    let mut complexity = 1;
    let mut line_count = 1;
    let mut i = start;
    
    // Count function name line
    while i < chars.len() && chars[i] != '\n' { i += 1; }
    while i < chars.len() && chars[i] == '\n' { 
        i += 1; 
        line_count += 1;
    }
    
    // Find body
    while i < chars.len() {
        // Skip strings
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' { i += 1; }
                i += 1;
            }
            i += 1;
            continue;
        }
        
        // Skip comments
        if chars[i] == '/' && i + 1 < chars.len() {
            if chars[i + 1] == '/' {
                while i < chars.len() && chars[i] != '\n' { i += 1; }
                continue;
            }
            if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') { i += 1; }
                i += 2;
                continue;
            }
        }
        
        // Track newlines
        if chars[i] == '\n' {
            line_count += 1;
        }
        
        // Count complexity
        let remaining = &chars[i..];
        if remaining.starts_with(&['i','f',' ']) || 
           remaining.starts_with(&['e','l','s','e',' ','i','f',' ']) ||
           remaining.starts_with(&['w','h','i','l','e',' ']) ||
           remaining.starts_with(&['f','o','r',' ']) ||
           remaining.starts_with(&['m','a','t','c','h',' ']) {
            complexity += 1;
        }
        
        // Track braces
        if chars[i] == '{' {
            depth += 1;
        } else if chars[i] == '}' {
            depth -= 1;
            if depth == 0 {
                return (i + 1, line_count, complexity);
            }
        }
        
        i += 1;
    }
    
    (i, line_count, complexity)
}

fn find_line_num(pos: usize, line_starts: &[usize]) -> usize {
    for (i, &start) in line_starts.iter().enumerate() {
        if start > pos {
            return i;
        }
    }
    line_starts.len()
}
