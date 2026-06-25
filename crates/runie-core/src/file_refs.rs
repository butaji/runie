use base64::Engine;
use std::ops::RangeInclusive;
use std::path::Path;

/// Parsed result from a `@path` or `@path:start-end` reference.
#[derive(Debug, Clone)]
pub struct ParsedFileRef {
    /// The file path (range suffix stripped if present).
    pub path: String,
    /// The line range if `:start-end` suffix was present.
    pub range: Option<RangeInclusive<u32>>,
    /// The original input string (used to preserve range suffix on insertion).
    pub original: String,
}

/// Parse a file reference string, supporting optional `:start-end` line range suffix.
///
/// Examples:
/// - `"src/main.rs"` → `("src/main.rs", None)`
/// - `"src/main.rs:10-50"` → `("src/main.rs", Some(10..=50))`
/// - `"src/main.rs:10"` → `("src/main.rs:10", None)` — trailing colon not a range
pub fn parse_file_ref(input: &str) -> Option<ParsedFileRef> {
    if input.is_empty() {
        return None;
    }

    let Some(colon_pos) = find_range_separator(input) else {
        return Some(plain_ref(input, input));
    };

    let path = input[..colon_pos].to_string();
    let range_str = &input[colon_pos + 1..];

    if !is_valid_range_str(range_str) {
        return Some(plain_ref(input, input));
    }

    // Try to parse the range.
    let parts: Vec<&str> = range_str.split('-').collect();
    let start: u32 = match parts[0].trim().parse() {
        Ok(n) if n != 0 => n,
        _ => return None, // unparseable or zero — unrecoverable
    };
    let end: u32 = match parts[1].trim().parse() {
        Ok(n) if n != 0 => n,
        _ => return None,
    };

    if start > end {
        // Inverted range — path is the part before the colon.
        return Some(plain_ref(&path, input));
    }

    Some(ParsedFileRef {
        path,
        range: Some(start..=end),
        original: input.to_owned(),
    })
}

/// Return a plain ParsedFileRef with an explicit path and the original input preserved.
fn plain_ref(path: &str, original: &str) -> ParsedFileRef {
    ParsedFileRef {
        path: path.to_owned(),
        range: None,
        original: original.to_owned(),
    }
}

/// Check if a range suffix string has exactly one hyphen (valid for range parsing).
fn is_valid_range_str(s: &str) -> bool {
    s.matches('-').count() == 1 && !s.starts_with('-') && !s.ends_with('-')
}

/// Find the position of the `:` that separates the path from a line range suffix.
/// Requires a `:` followed by at least one digit (valid range start).
fn find_range_separator(input: &str) -> Option<usize> {
    // Find the last `:` in the string.
    let last_colon = input.rfind(':')?;

    // It must be followed by at least one digit (start of line number).
    let after = input.get(last_colon + 1..)?;
    if !after.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }

    Some(last_colon)
}

#[derive(Debug, Clone)]
pub struct FileRef {
    pub path: String,
    pub text: String,
    pub is_image: bool,
}

/// A file entry with directory flag for the @-file picker.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
}

/// List files/folders in base directory, marking dirs with is_dir=true.
/// Names are returned as-is; callers append `/` for display/insertion.
pub fn find_file_entries(base: &str, limit: usize) -> Vec<FileEntry> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(base) else {
        return results;
    };
    for entry in entries.flatten().take(limit) {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        results.push(FileEntry { name, is_dir });
    }
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

pub fn find_files(pattern: &str, base: &str, limit: usize) -> Vec<String> {
    find_files_deep(pattern, base, limit)
}

pub fn find_files_shallow(pattern: &str, base: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let pat_lower = pattern.to_lowercase();
    let Ok(entries) = std::fs::read_dir(base) else {
        return results;
    };
    for entry in entries.flatten() {
        if results.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_lowercase().contains(&pat_lower) {
            results.push(name);
        }
    }
    results.sort();
    results
}

pub fn find_files_deep(pattern: &str, base: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    if pattern.is_empty() || pattern == "*" {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten().take(limit) {
                let name = entry.file_name().to_string_lossy().to_string();
                results.push(name);
            }
        }
        return results;
    }
    let is_glob = pattern.contains('*') || pattern.contains('?');
    if is_glob {
        collect_deep(base, &mut results, limit, &|name, path| {
            glob_matches(name, pattern) || glob_matches(path, pattern)
        });
    } else {
        let pat_lower = pattern.to_lowercase();
        collect_deep(base, &mut results, limit, &|name, path| {
            name.to_lowercase().contains(&pat_lower) || path.to_lowercase().contains(&pat_lower)
        });
    }
    results
}

fn collect_deep<F>(dir: &str, out: &mut Vec<String>, limit: usize, matches: &F)
where
    F: Fn(&str, &str) -> bool,
{
    if out.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().to_string_lossy().to_string();
        let meta = entry.metadata();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        if matches(&name, &path) {
            out.push(path.clone());
        }
        if is_dir && !name.starts_with('.') && name != "target" {
            collect_deep(&path, out, limit, matches);
        }
    }
}

fn glob_matches(name: &str, pattern: &str) -> bool {
    let name_lower = name.to_lowercase();
    let pat_lower = pattern.to_lowercase();
    if let Some(ext) = pat_lower.strip_prefix("*.") {
        name_lower.ends_with(&format!(".{}", ext))
    } else if pat_lower.contains('*') || pat_lower.contains('?') {
        name_lower.contains(&pat_lower.replace("*", "").replace("?", ""))
    } else {
        name_lower.contains(&pat_lower)
    }
}

pub fn is_image_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg"
    )
}

pub fn read_file_ref(path: &str) -> Result<FileRef, String> {
    read_file_ref_with_range(path, None)
}

/// Read a file reference, optionally extracting only the given line range.
/// Returns only those lines (1-indexed, inclusive), clamped to the file's actual bounds.
pub fn read_file_ref_with_range(
    path: &str,
    range: Option<RangeInclusive<u32>>,
) -> Result<FileRef, String> {
    let is_image = is_image_file(path);

    if is_image {
        // Images are always read whole — ranges don't apply.
        let bytes = std::fs::read(path).map_err(|e| format!("Error reading {}: {}", path, e))?;
        return Ok(FileRef {
            path: path.to_owned(),
            text: base64::engine::general_purpose::STANDARD.encode(&bytes),
            is_image: true,
        });
    }

    let full_text =
        std::fs::read_to_string(path).map_err(|e| format!("Error reading {}: {}", path, e))?;

    let text = match range {
        None => full_text,
        Some(r) => extract_lines(&full_text, r)?,
    };

    Ok(FileRef {
        path: path.to_owned(),
        text,
        is_image: false,
    })
}

/// Extract lines from a text string, given a 1-indexed inclusive range.
/// Returns an error for invalid ranges (start > end).
pub fn extract_lines(text: &str, range: RangeInclusive<u32>) -> Result<String, String> {
    let total_lines = text.lines().count();
    if total_lines == 0 {
        return Ok(String::new());
    }

    let total_lines_u32 = total_lines as u32;
    let start = *range.start();
    let end = *range.end();

    if start > end {
        return Err(format!("Invalid range: start ({}) > end ({})", start, end));
    }

    // Clamp to file bounds.
    let start = start.min(total_lines_u32);
    let end = end.min(total_lines_u32);

    // Convert from 1-indexed to 0-indexed.
    let start_idx = (start - 1) as usize;
    let end_idx = (end - 1) as usize;

    let lines: Vec<&str> = text
        .lines()
        .skip(start_idx)
        .take(end_idx - start_idx + 1)
        .collect();
    Ok(lines.join("\n"))
}

pub fn complete_at_ref(input: &str, base: &str, limit: usize) -> Vec<String> {
    let query = input.split('@').next_back().unwrap_or("");
    find_files_shallow(query, base, limit)
}

pub fn insert_at_ref(input: &str, selected: &str) -> String {
    if let Some(pos) = input.rfind('@') {
        let prefix = &input[..pos];
        format!("{}[{}]", prefix, selected)
    } else {
        input.to_owned()
    }
}
