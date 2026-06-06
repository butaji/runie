use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileRef {
    pub path: String,
    pub text: String,
    pub is_image: bool,
}

pub fn find_files(pattern: &str, base: &str, limit: usize) -> Vec<String> {
    find_files_deep(pattern, base, limit)
}

pub fn find_files_shallow(pattern: &str, base: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let pat_lower = pattern.to_lowercase();
    let Ok(entries) = std::fs::read_dir(base) else { return results };
    for entry in entries.flatten() {
        if results.len() >= limit { break; }
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
        collect_glob(base, pattern, &mut results, limit);
    } else {
        let pat_lower = pattern.to_lowercase();
        collect_matches(base, &pat_lower, &mut results, limit);
    }
    results
}

fn collect_glob(dir: &str, pattern: &str, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if out.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().to_string_lossy().to_string();
        let meta = entry.metadata();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        if glob_matches(&name, pattern) || glob_matches(&path, pattern) {
            out.push(path.clone());
        }
        if is_dir && !name.starts_with('.') && name != "target" {
            collect_glob(&path, pattern, out, limit);
        }
    }
}

fn glob_matches(name: &str, pattern: &str) -> bool {
    let name_lower = name.to_lowercase();
    let pat_lower = pattern.to_lowercase();
    if pat_lower.starts_with("*.") {
        let ext = &pat_lower[2..];
        name_lower.ends_with(&format!(".{}", ext))
    } else if pat_lower.contains('*') || pat_lower.contains('?') {
        name_lower.contains(&pat_lower.replace("*", "").replace("?", ""))
    } else {
        name_lower.contains(&pat_lower)
    }
}

fn collect_matches(dir: &str, pat: &str, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if out.len() >= limit {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().to_string_lossy().to_string();
        let meta = entry.metadata();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        if name.to_lowercase().contains(pat) || path.to_lowercase().contains(pat) {
            out.push(path.clone());
        }
        if is_dir && !name.starts_with('.') && name != "target" {
            collect_matches(&path, pat, out, limit);
        }
    }
}

pub fn is_image_file(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg")
}

pub fn read_file_ref(path: &str) -> Result<FileRef, String> {
    let is_image = is_image_file(path);
    if is_image {
        match std::fs::read(path) {
            Ok(bytes) => {
                let b64 = base64::encode(&bytes);
                Ok(FileRef {
                    path: path.to_string(),
                    text: b64,
                    is_image: true,
                })
            }
            Err(e) => Err(format!("Error reading {}: {}", path, e)),
        }
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(FileRef {
                path: path.to_string(),
                text,
                is_image: false,
            }),
            Err(e) => Err(format!("Error reading {}: {}", path, e)),
        }
    }
}

pub fn complete_at_ref(input: &str, base: &str, limit: usize) -> Vec<String> {
    let query = input.split('@').last().unwrap_or("");
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
