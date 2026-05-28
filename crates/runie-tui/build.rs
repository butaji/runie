use std::fs;
use std::path::Path;

const MAX_FILE_LINES: usize = 1200;
const MAX_FUNC_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

fn main() {
    println!("cargo::rerun-if-changed=src/");
    check_source_files("src");
}

fn check_source_files(dir: &str) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            check_source_files(&path.to_string_lossy());
        } else if path.extension().is_some_and(|e| e == "rs") {
            check_file(&path);
        }
    }
}

fn check_file(path: &Path) {
    let content = fs::read_to_string(path).expect("Failed to read file");
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() > MAX_FILE_LINES {
        panic!(
            "\nFILE TOO LONG: {} has {} lines (max {})\n",
            path.display(), lines.len(), MAX_FILE_LINES
        );
    }

    check_functions(path, &lines);
}

fn check_functions(path: &Path, lines: &[&str]) {
    let mut in_function = false;
    let mut func_start = 0;
    let mut brace_depth = 0;
    let mut func_name = String::new();

    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();

        if !in_function {
            if is_function_start(stripped) {
                func_name = extract_func_name(stripped);
                if !func_name.is_empty() {
                    in_function = true;
                    func_start = i;
                    brace_depth = stripped.matches('{').count() as i32 - stripped.matches('}').count() as i32;
                    if brace_depth <= 0 {
                        in_function = false; // Single-line or trait sig
                    }
                }
            }
            continue;
        }

        let code_part = stripped.split("//").next().unwrap_or("");
        for ch in code_part.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        let func_lines = count_non_empty_lines(&lines[func_start..=i]);
                        let complexity = calculate_complexity(&lines[func_start..=i]);

                        if func_lines > MAX_FUNC_LINES {
                            panic!(
                                "\nFUNCTION TOO LONG: `{}` in {} has {} lines (max {} at line {})\n",
                                func_name, path.display(), func_lines, MAX_FUNC_LINES, func_start + 1
                            );
                        }
                        if complexity > MAX_COMPLEXITY {
                            panic!(
                                "\nCOMPLEXITY TOO HIGH: `{}` in {} has complexity {} (max {} at line {})\n",
                                func_name, path.display(), complexity, MAX_COMPLEXITY, func_start + 1
                            );
                        }
                        in_function = false;
                        break;
                    }
                }
                _ => {}
            }
        }
    }
}

fn is_function_start(line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 { return false; }
    
    // Check for fn keyword, optionally after pub/async/const/unsafe
    for (i, part) in parts.iter().enumerate() {
        if *part == "fn" && i > 0 {
            let prev = parts[i - 1];
            // Valid before fn: pub, async, const, unsafe, crate, super, self
            let valid_prev = ["pub", "async", "const", "unsafe", "crate", "super", "self"];
            if valid_prev.contains(&prev) || i == 1 {
                return !line.starts_with("//") && !line.starts_with("*");
            }
        }
    }
    false
}

fn extract_func_name(line: &str) -> String {
    if let Some(fn_pos) = line.find("fn ") {
        let after_fn = &line[fn_pos + 3..];
        let end = after_fn.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '<')
            .unwrap_or(after_fn.len());
        return after_fn[..end].to_string();
    }
    String::new()
}

fn count_non_empty_lines(lines: &[&str]) -> usize {
    lines.iter().filter(|l| {
        let s = l.trim();
        !s.is_empty() && !s.starts_with("//") && !s.starts_with('*')
    }).count()
}

fn calculate_complexity(lines: &[&str]) -> usize {
    let mut complexity = 1;
    for line in lines {
        let s = line.trim();
        if s.is_empty() || s.starts_with("//") || s.starts_with('*') {
            continue;
        }
        complexity += count_branches(s);
        complexity += s.matches('?').count();
        complexity += s.matches("&&").count();
        complexity += s.matches("||").count();
    }
    complexity
}

fn count_branches(line: &str) -> usize {
    let mut count = 0;
    for word in ["if ", "else", "match ", "for ", "while ", "loop"] {
        count += line.matches(word).count();
    }
    count
}
