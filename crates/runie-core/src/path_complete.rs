//! Path completion — suggests files and directories for tab completion.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathCompletion {
    pub path: String,
    pub is_dir: bool,
}

/// Generate path completions for a partial input.
///
/// If `partial` is empty, returns entries from `cwd`.
/// If `partial` ends with `/`, returns children of that directory.
/// Otherwise, returns entries in the parent directory that match the filename prefix.
pub fn complete_path(partial: &str, cwd: &Path) -> Vec<PathCompletion> {
    let base: PathBuf = if partial.is_empty() {
        cwd.to_path_buf()
    } else {
        cwd.join(partial)
    };

    if partial.ends_with('/') || partial.is_empty() {
        collect_completions(&base, "")
    } else {
        let parent = base.parent().unwrap_or(cwd).to_path_buf();
        let prefix = base
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        collect_completions(&parent, &prefix)
    }
}

fn collect_completions(dir: &Path, prefix: &str) -> Vec<PathCompletion> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with(prefix) {
                continue;
            }
            if name.starts_with('.') && !prefix.starts_with('.') {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let full = dir.join(&name);
            results.push(PathCompletion {
                path: full.to_string_lossy().to_string(),
                is_dir,
            });
        }
    }
    results.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.path.cmp(&b.path))
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_dir_with(files: &[(&str, bool)]) -> (std::path::PathBuf, Vec<String>) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("runie_path_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut names = Vec::new();
        for (name, is_dir) in files {
            let path = dir.join(name);
            if *is_dir {
                std::fs::create_dir_all(&path).unwrap();
            } else {
                std::fs::write(&path, b"").unwrap();
            }
            names.push(name.to_string());
        }
        (dir, names)
    }

    #[test]
    fn complete_empty_returns_cwd_entries() {
        let (dir, _names) = tmp_dir_with(&[("alpha.txt", false), ("beta", true)]);
        let results = complete_path("", &dir);
        assert!(results.iter().any(|r| r.path.ends_with("alpha.txt") && !r.is_dir));
        assert!(results.iter().any(|r| r.path.ends_with("beta") && r.is_dir));
    }

    #[test]
    fn complete_filters_prefix() {
        let (dir, _names) = tmp_dir_with(&[("src", true), ("main.rs", false), ("lib.rs", false)]);
        let results = complete_path("main", &dir);
        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("main.rs"));
    }

    #[test]
    fn complete_excludes_hidden_by_default() {
        let (dir, _names) = tmp_dir_with(&[(".hidden", false), ("visible", false)]);
        let results = complete_path("", &dir);
        assert!(!results.iter().any(|r| r.path.contains(".hidden")));
        assert!(results.iter().any(|r| r.path.contains("visible")));
    }

    #[test]
    fn complete_includes_hidden_when_prefixed() {
        let (dir, _names) = tmp_dir_with(&[(".hidden", false), ("visible", false)]);
        let results = complete_path(".hid", &dir);
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains(".hidden"));
    }

    #[test]
    fn complete_directories_first() {
        let (dir, _names) = tmp_dir_with(&[("z_dir", true), ("a_file", false)]);
        let results = complete_path("", &dir);
        assert!(results[0].is_dir, "directories should come first");
        assert!(!results[1].is_dir, "files should come after directories");
    }
}
