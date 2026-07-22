//! Workspace-wide lint checks: file size and function size.
//! Run with: cargo run -p lint-check
//!
//! Exits 0 on success, non-zero on any violation.
//!
//! Exemptions are configured in `lint-check.toml` at the workspace root.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

const MAX_FILE_LINES: usize = 500;
const MAX_FUN_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

fn main() {
    // CARGO_MANIFEST_DIR is lint-check/, go up one level to workspace root
    let ws_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let crates = ws_root.join("crates");
    let config_path = ws_root.join("lint-check.toml");

    // Load exemptions
    let config = if config_path.exists() {
        load_config(&config_path)
    } else {
        Config::default()
    };

    let mut failed = false;

    for entry in fs::read_dir(&crates).expect("crates dir not found") {
        let crate_dir = entry.expect("read_dir entry").path();
        if !crate_dir.is_dir() {
            continue;
        }

        let src_dir = crate_dir.join("src");
        if !src_dir.exists() {
            continue;
        }

        check_dir(&src_dir, &ws_root, &config, &mut failed);
    }

    if failed {
        std::process::exit(1);
    }
    println!("All lint checks passed.");
}

// ──────────────────────────────────────────────────────────────────────────────
// Config
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Config {
    exempt_files: HashSet<String>,
    exempt_functions: Vec<(String, String)>, // (file, function prefix)
}

fn load_config(path: &Path) -> Config {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut config = Config::default();
    let mut current_section = "";
    let mut current_file: String = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') {
            current_section = trimmed.trim_matches(|c| "[ ]\"".contains(c));
            if current_section == "exempt_functions" {
                current_file = String::new();
            }
            continue;
        }

        if current_section == "exempt_files" {
            if let Some(key) = trimmed.split('=').next() {
                let key = key.trim().trim_matches('"');
                // Store just the filename for substring matching against full paths
                let key = if key.contains('/') {
                    key.rsplit('/').next().unwrap_or(key)
                } else {
                    key
                };
                config.exempt_files.insert(key.to_string());
            }
        } else if current_section == "exempt_functions" {
            // Format (inline table): "path/to/file.rs" = { "fn prefix" = "reason" }
            // Format (function-only): "fn prefix" = "reason"  (uses current_file)
            if trimmed.starts_with('"') {
                if let Some(file) = trimmed.split('"').nth(1) {
                    current_file = file.to_string();
                }
            }

            if trimmed.starts_with('"') && trimmed.contains("= {") {
                // Inline table: file from nth(1), fn from nth(3)
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    let file_pat_raw = parts[1].trim();
                    let fun_pat = parts[3].trim();
                    let file_key = if file_pat_raw.contains('/') {
                        file_pat_raw
                            .rsplit('/')
                            .next()
                            .unwrap_or(file_pat_raw)
                            .to_string()
                    } else {
                        file_pat_raw.to_string()
                    };
                    if !file_key.is_empty() {
                        config
                            .exempt_functions
                            .push((file_key, fun_pat.to_string()));
                    }
                }
            } else if trimmed.starts_with('"') && !trimmed.contains("= {") {
                // Function-only line: use current_file as file pattern
                if let Some(name) = trimmed.split('"').nth(1) {
                    let name = name.trim();
                    let file_key = if current_file.contains('/') {
                        current_file
                            .rsplit('/')
                            .next()
                            .unwrap_or(&current_file)
                            .to_string()
                    } else {
                        current_file.clone()
                    };
                    if !file_key.is_empty() {
                        config.exempt_functions.push((file_key, name.to_string()));
                    }
                }
            }
        }
    }

    config
}

fn is_exempt_file(ws_root: &Path, path: &Path, config: &Config) -> Option<String> {
    let rel = path.strip_prefix(ws_root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    // Check exact match
    if config.exempt_files.contains(&*rel_str) {
        return Some(rel_str.to_string());
    }

    // Check partial match
    for exempt in &config.exempt_files {
        if rel_str.contains(exempt) || exempt.contains(&*rel_str) {
            return Some(exempt.clone());
        }
    }

    None
}

// Strip visibility, async, and fn keywords iteratively from the start of a
// function declaration so we can extract the bare function name.
fn strip_prefixes(s: &str) -> &str {
    let mut current = s;
    for prefix in ["pub ", "pub(crate) ", "pub(super) ", "async ", "fn "] {
        if let Some(after) = current.strip_prefix(prefix) {
            current = after;
        }
    }
    current
}

fn is_exempt_fun(line: &str, path: &Path, ws_root: &Path, config: &Config) -> bool {
    let rel = path.strip_prefix(ws_root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    // Extract function name from declaration
    let trimmed_line = line.trim();
    let after_prefix = strip_prefixes(trimmed_line);
    let fun_name: String = after_prefix
        .split('(')
        .next()
        .unwrap_or(after_prefix)
        .split('<')
        .next()
        .unwrap_or(after_prefix)
        .trim()
        .to_string();

    for (file_pat, fun_pat) in &config.exempt_functions {
        let rel_str_ref: &str = &rel_str;
        if rel_str_ref.contains(file_pat) || file_pat.is_empty() || file_pat == rel_str_ref {
            // Strip visibility/async/fn keywords from the pattern to get the bare
            // function name for comparison against the extracted fun_name.
            let after_pat = strip_prefixes(fun_pat);
            let fun_pat_stripped: String = after_pat
                .split('(')
                .next()
                .unwrap_or(after_pat)
                .split('<')
                .next()
                .unwrap_or(after_pat)
                .trim()
                .to_string();

            if fun_name.starts_with(&fun_pat_stripped) {
                return true;
            }
        }
    }

    false
}

// ──────────────────────────────────────────────────────────────────────────────
// Checking
// ──────────────────────────────────────────────────────────────────────────────

fn check_dir(dir: &Path, ws_root: &Path, config: &Config, failed: &mut bool) {
    let skip_dirs = ["tests", "benches", "target"];

    for entry in walkdir::WalkDir::new(dir).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !skip_dirs.contains(&name.as_ref()) && !name.starts_with('.')
    }) {
        let entry = entry.expect("walkdir entry");
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                check_file(path, ws_root, config, failed);
            }
        }
    }
}

fn check_file(path: &Path, ws_root: &Path, config: &Config, failed: &mut bool) {
    let content = fs::read_to_string(path).expect("read file");

    // Files with #[allow(lint_check)] on line 1 opt out of limits
    if content
        .lines()
        .next()
        .is_some_and(|l| l.contains("#[allow(lint_check)]"))
    {
        return;
    }

    let rel = path.strip_prefix(ws_root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();
    let in_test_file = rel_str.contains("/tests/");

    let lines: usize = content.lines().count();

    // Check file line limit (skip test files and if exempted)
    if lines > MAX_FILE_LINES && !in_test_file {
        if let Some(exempt) = is_exempt_file(ws_root, path, config) {
            println!("  (EXEMPT) {} ({lines} lines) — {}", rel.display(), exempt);
        } else {
            eprintln!(
                "ERROR: {} has {} lines (max {MAX_FILE_LINES})",
                rel.display(),
                lines,
            );
            *failed = true;
        }
    }

    // Check function line counts
    let in_test_file = rel_str.contains("/tests/");
    check_function_sizes(&content, path, ws_root, config, in_test_file, failed);
}

fn check_function_sizes(
    content: &str,
    path: &Path,
    ws_root: &Path,
    config: &Config,
    in_test_file: bool,
    failed: &mut bool,
) {
    let rel = path.strip_prefix(ws_root).unwrap_or(path);

    // Track #[cfg(test)] depth
    let mut in_cfg_test = false;
    let mut cfg_test_depth = 0usize;

    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Track #[cfg(test)] blocks
        if trimmed == "#[cfg(test)]" {
            in_cfg_test = true;
            cfg_test_depth = 1;
        } else if in_cfg_test {
            if trimmed.starts_with("#[cfg(") && trimmed != "#[cfg(test)]" {
                cfg_test_depth += 1;
            }
            if (trimmed.starts_with("fn ") || trimmed.starts_with("async fn ")) && cfg_test_depth == 1 {
                // entering a test fn
                continue;
            }
            if trimmed == "}" {
                cfg_test_depth = cfg_test_depth.saturating_sub(1);
                if cfg_test_depth == 0 {
                    in_cfg_test = false;
                }
            }
        }

        // Match function declarations
        if !(trimmed.starts_with("async fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub async fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(crate) async fn ")
            || trimmed.starts_with("pub(super) fn ")
            || trimmed.starts_with("pub(super) async fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub async fn "))
        {
            continue;
        }

        // Skip test files / #[cfg(test)] blocks / #[test] functions
        if in_test_file || in_cfg_test {
            continue;
        }
        if i > 0 {
            let prev = content.lines().nth(i - 1).unwrap_or("").trim();
            if prev == "#[test]" || prev.starts_with("#[test(") {
                continue;
            }
        }

        // Check exemption
        if is_exempt_fun(trimmed, path, ws_root, config) {
            continue;
        }

        // Functions with #[allow(lint_check::max_lines)] or #[allow(lint_check::complexity)] opt out
        if i > 0 {
            let prev_line = content.lines().nth(i - 1).unwrap_or("");
            if prev_line.contains("#[allow(lint_check::max_lines)]")
                || prev_line.contains("#[allow(lint_check::complexity)]")
            {
                continue;
            }
        }

        // Count lines and cyclomatic complexity until closing brace at same indent
        let indent = line.len() - line.trim_start().len();
        let mut depth = 0usize;
        let mut fun_lines = 1usize;
        let mut complexity = 1usize; // base complexity

        for l in content.lines().skip(i + 1) {
            let li = l.trim();
            if li.is_empty() {
                fun_lines += 1;
                continue;
            }
            let li_indent = l.len() - l.trim_start().len();
            if li_indent < indent && !li.contains('{') && !li.contains('}') {
                break;
            }
            depth += li.matches('{').count();
            depth = depth.saturating_sub(li.matches('}').count());
            fun_lines += 1;

            // Strip line comments and string literals before counting complexity
            let stripped: String = li
                .lines()
                .next()
                .unwrap_or("")
                .split_once("//")
                .map(|(code, _)| code)
                .unwrap_or(li)
                .split_once('"')
                .map(|(pre, rest)| {
                    let post = rest.replace('"', "~Q~");
                    format!("{}{}", pre, post)
                })
                .unwrap_or_else(|| li.to_owned());

            // Count branch patterns
            for pat in ["if ", "match ", "while ", "for ", "loop ", "&&", "||", "? "] {
                complexity += stripped.matches(pat).count();
            }
            complexity += stripped.matches(" else").count();

            if depth == 0 {
                break;
            }
        }

        if fun_lines > MAX_FUN_LINES {
            eprintln!(
                "ERROR: {}:{} function '{trimmed}' has {fun_lines} lines (max {MAX_FUN_LINES})",
                rel.display(),
                i + 1,
            );
            *failed = true;
        }

        if complexity > MAX_COMPLEXITY {
            eprintln!(
                "ERROR: {}:{} function '{trimmed}' has complexity {complexity} (max {MAX_COMPLEXITY})",
                rel.display(),
                i + 1,
            );
            *failed = true;
        }
    }
}
