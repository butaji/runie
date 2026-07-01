//! Atomic file write helper with advisory locking and restricted permissions.
//!
//! Uses fs2 advisory locks + tempfile + rename for atomicity, and sets
//! Unix permissions to 0o600 for security.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

/// Atomically write content to a file using temp + rename + fs2 advisory lock.
/// Sets Unix permissions to 0o600.
pub fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    // Get the parent directory
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;

    // Create temp file in same directory for atomic rename using tempfile
    let tmp = NamedTempFile::new_in(parent)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Acquire exclusive advisory lock on target file
    let lock_path = path.with_extension("lock");
    let lock_file = File::create(&lock_path)?;
    fs2::FileExt::lock_exclusive(&lock_file)?;

    // Get the underlying file reference for sync and permissions
    let mut tmp_file = tmp.as_file();

    // Write content to temp file
    tmp_file.write_all(content.as_bytes())?;
    tmp_file.sync_all()?;

    // Set permissions to 0o600 (user read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        tmp_file.set_permissions(perms)?;
    }

    // Release lock before rename (rename on same filesystem is atomic)
    drop(lock_file);
    std::fs::remove_file(&lock_path).ok(); // Ignore remove errors

    // Atomically rename temp to target
    tmp.persist(path)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // Set permissions on the final file too (belt and suspenders)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn atomic_write_creates_file() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "runie_atomic_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let path = tmp_dir.join("test.json");
        atomic_write(&path, r#"{"key": "value"}"#).unwrap();

        let mut contents = String::new();
        File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
        assert_eq!(contents, r#"{"key": "value"}"#);
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "runie_atomic_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let path = tmp_dir.join("test.json");
        std::fs::write(&path, "old").unwrap();

        atomic_write(&path, "new").unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_sets_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp_dir = std::env::temp_dir().join(format!(
            "runie_atomic_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let path = tmp_dir.join("test.json");
        atomic_write(&path, "secret").unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        let mode = perms.mode();
        // 0o600 = read/write for owner only
        assert_eq!(mode & 0o777, 0o600, "File should have 0o600 permissions, got {:o}", mode);
    }
}
