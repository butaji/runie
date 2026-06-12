//! Path resolution utilities for tools.
//!
//! Resolves relative paths against the current working directory,
//! expands `~` to the home directory, and normalizes `.` and `..`.

use std::path::{Path, PathBuf};

/// Resolve a raw path string to an absolute, normalized path.
///
/// - Absolute paths are normalized but otherwise unchanged.
/// - Relative paths are joined against the current working directory.
/// - Leading `~` is expanded to the user's home directory.
/// - `.` and `..` components are normalized.
pub fn resolve_path(raw: &str) -> PathBuf {
    let expanded = expand_tilde(raw);
    let path = Path::new(&expanded);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
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
    path.to_string()
}

/// Normalize a path by resolving `.` and `..` components.
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
        let resolved = resolve_path("/tmp/foo");
        assert_eq!(resolved, PathBuf::from("/tmp/foo"));
    }

    #[test]
    fn resolve_relative_joins_cwd() {
        let resolved = resolve_path("src/main.rs");
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(resolved, cwd.join("src/main.rs"));
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
        let resolved = resolve_path("./src/main.rs");
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(resolved, cwd.join("src/main.rs"));
    }

    #[test]
    fn resolve_normalizes_dotdot() {
        let resolved = resolve_path("foo/../bar");
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(resolved, cwd.join("bar"));
    }
}
