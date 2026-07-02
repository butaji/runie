//! Path resolution utilities shared across the workspace.
//!
//! Resolves relative paths against a working directory, expands `~` to the home
//! directory, and normalizes `.` and `..`. Uses `shellexpand` for tilde expansion
//! and `path-absolutize` for cross-platform path normalization.

use std::path::{Path, PathBuf};
use path_absolutize::Absolutize;

/// Resolve a raw path string to an absolute, normalized path using the current
/// working directory.
pub fn resolve_path(raw: &str) -> PathBuf {
    resolve_path_in(raw, std::env::current_dir().unwrap_or_default())
}

/// Resolve a raw path string to an absolute, normalized path relative to the
/// given working directory.
pub fn resolve_path_in(raw: &str, working_dir: impl AsRef<Path>) -> PathBuf {
    let working_dir = working_dir.as_ref();
    let expanded = shellexpand::tilde(raw).into_owned();
    let path = Path::new(&expanded);
    // Use path-absolutize for cross-platform absolute + normalized paths.
    // This handles `.` and `..` components, as well as Windows paths properly.
    
    if path.is_absolute() {
        path.absolutize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        let joined = working_dir.join(path);
        joined.absolutize().unwrap_or(joined)
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
