//! Path resolution utilities shared across the workspace.
//!
//! Resolves relative paths against a working directory, expands `~` to the home
//! directory, and normalizes `.` and `..`.

use std::path::{Path, PathBuf};

/// Resolve a raw path string to an absolute, normalized path using the current
/// working directory.
pub fn resolve_path(raw: &str) -> PathBuf {
    resolve_path_in(raw, &std::env::current_dir().unwrap_or_default())
}

/// Resolve a raw path string to an absolute, normalized path relative to the
/// given working directory.
pub fn resolve_path_in(raw: &str, working_dir: &Path) -> PathBuf {
    let expanded = expand_tilde(raw);
    let path = Path::new(&expanded);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        working_dir.join(path)
    };
    normalize_path(&absolute)
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), rest);
        }
    }
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    path.to_owned()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(p) => result.push(p.as_os_str()),
            std::path::Component::RootDir => result.push("/"),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::Normal(name) => result.push(name),
        }
    }
    if result.as_os_str().is_empty() {
        path.to_path_buf()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_absolute_unchanged() {
        let resolved = resolve_path_in("/tmp/foo", Path::new("/home"));
        assert_eq!(resolved, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn resolve_relative_joins_working_dir() {
        let resolved = resolve_path_in("src/main.rs", Path::new("/home/user"));
        assert_eq!(resolved, PathBuf::from("/home/user/src/main.rs"));
    }

    #[test]
    fn resolve_tilde_expands() {
        let resolved = resolve_path("~/.runie");
        let home = dirs::home_dir().unwrap();
        assert_eq!(resolved, home.join(".runie"));
    }

    #[test]
    fn resolve_tilde_alone_expands() {
        let resolved = resolve_path("~");
        let home = dirs::home_dir().unwrap();
        assert_eq!(resolved, home);
    }

    #[test]
    fn resolve_normalizes_dot() {
        let resolved = resolve_path_in("./src/main.rs", Path::new("/home/user"));
        assert_eq!(resolved, PathBuf::from("/home/user/src/main.rs"));
    }

    #[test]
    fn resolve_normalizes_dotdot() {
        let resolved = resolve_path_in("foo/../bar", Path::new("/home/user"));
        assert_eq!(resolved, PathBuf::from("/home/user/bar"));
    }
}
