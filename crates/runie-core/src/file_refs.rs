use base64::Engine;
use std::path::Path;

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
    let is_image = is_image_file(path);
    let (text, is_image) = if is_image {
        let bytes = std::fs::read(path).map_err(|e| format!("Error reading {}: {}", path, e))?;
        (
            base64::engine::general_purpose::STANDARD.encode(&bytes),
            true,
        )
    } else {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("Error reading {}: {}", path, e))?;
        (text, false)
    };
    Ok(FileRef {
        path: path.to_string(),
        text,
        is_image,
    })
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
        input.to_string()
    }
}
