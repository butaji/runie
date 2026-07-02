use base64::Engine;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::{DirEntry, WalkBuilder};
use regex::Regex;
use std::ops::RangeInclusive;
use std::path::Path;
use std::sync::LazyLock;

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

/// Compiled regex for `@path:start-end` file references.
///
/// Pattern: `^(?<path>.+?):(?<start>\d+)-(?<end>\d+)$`
///
/// Guarantees:
/// - One colon separates path from range (regex ensures exactly one `:` in the matched part).
/// - Exactly one hyphen between digits (the `\d+-\d+` sub-pattern).
/// - Both start and end are non-zero (explicit check after capture).
/// - Inverted ranges (`start > end`) fall through to plain-ref behavior.
static FILE_REF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?<path>.+?):(?<start>\d+)-(?<end>\d+)$").expect("file-ref regex is valid")
});

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

    let Some(caps) = FILE_REF_RE.captures(input) else {
        // No range suffix (e.g. "file.txt", "file.txt:100", "file.txt:").
        return Some(ParsedFileRef {
            path: input.to_owned(),
            range: None,
            original: input.to_owned(),
        });
    };

    let path = caps["path"].to_string();
    let start: u32 = caps["start"].parse().ok()?;
    let end: u32 = caps["end"].parse().ok()?;

    // Line 0 is not valid (same guard as the old manual parser).
    if start == 0 || end == 0 {
        return None;
    }

    if start > end {
        // Inverted range — treat as plain path.
        return Some(ParsedFileRef {
            path,
            range: None,
            original: input.to_owned(),
        });
    }

    Some(ParsedFileRef {
        path,
        range: Some(start..=end),
        original: input.to_owned(),
    })
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
    if pattern.is_empty() || pattern == "*" {
        return flat_files(base, limit);
    }

    let is_glob = pattern.contains('*') || pattern.contains('?');
    if is_glob {
        // Build a GlobSet from the pattern (case-insensitive).
        let pattern_lower = pattern.to_lowercase();
        let glob = match Glob::new(&pattern_lower) {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        let globset = GlobSetBuilder::new()
            .add(glob)
            .build()
            .unwrap_or_else(|_| GlobSet::empty());
        walk_ignore(base, limit, move |entry: &DirEntry| {
            let name = entry.file_name().to_string_lossy();
            let path = entry.path().to_string_lossy();
            let name_str = name.to_lowercase();
            let path_str = path.to_lowercase();
            glob_matches_globset(&globset, &name_str, &path_str)
        })
    } else {
        // Case-insensitive substring search.
        let pat_lower = pattern.to_lowercase();
        walk_ignore(base, limit, move |entry: &DirEntry| {
            let name = entry.file_name().to_string_lossy();
            let path = entry.path().to_string_lossy();
            let name_str = name.to_lowercase();
            let path_str = path.to_lowercase();
            name_str.contains(&pat_lower) || path_str.contains(&pat_lower)
        })
    }
}

/// Collect files from base directory (non-recursive) matching limit.
fn flat_files(base: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let Ok(entries) = std::fs::read_dir(base) else {
        return results;
    };
    for entry in entries.flatten().take(limit) {
        let name = entry.file_name().to_string_lossy().to_string();
        results.push(name);
    }
    results.sort_by(|a, b| a.cmp(b));
    results
}

/// Walk directory tree using `ignore::WalkBuilder`, respecting .gitignore,
/// skipping hidden files and target/ directories.
fn walk_ignore<F>(base: &str, limit: usize, matches: F) -> Vec<String>
where
    F: Fn(&DirEntry) -> bool + Send + Sync,
{
    let mut results = Vec::new();
    let walker = WalkBuilder::new(base)
        .hidden(true) // skip hidden files
        .filter_entry(move |entry| {
            let name = entry.file_name().to_string_lossy();
            // Skip target/ directories
            name != "target"
        })
        .build();

    for entry in walker.flatten() {
        if results.len() >= limit {
            break;
        }
        if matches(&entry) {
            results.push(entry.path().to_string_lossy().to_string());
        }
    }
    results
}

/// Check if a path matches a GlobSet.
fn glob_matches_globset(globset: &GlobSet, name_lower: &str, path_lower: &str) -> bool {
    globset.is_match(name_lower) || globset.is_match(path_lower)
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
