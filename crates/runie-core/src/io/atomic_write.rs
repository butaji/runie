//! Atomic file write helper with advisory locking and restricted permissions.
//!
//! Uses fs2 advisory locks + temp file + rename for atomicity, and sets
//! Unix permissions to 0o600 for security.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Atomically write content to a file using temp + rename + fs2 advisory lock.
/// Sets Unix permissions to 0o600.
pub fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    // Get the parent directory
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;

    // Create temp file in same directory for atomic rename
    let tmp_path = parent.join(format!(
        ".tmp.{}.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));

    // Write to temp file
    let mut tmp = File::create(&tmp_path)?;

    // Acquire exclusive advisory lock
    let lock_path = path.with_extension("lock");
    let lock_file = File::create(&lock_path)?;
    fs2::FileExt::lock_exclusive(&lock_file)?;

    // Write content to temp file
    tmp.write_all(content.as_bytes())?;
    tmp.sync_all()?;

    // Set permissions to 0o600 (user read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        tmp.set_permissions(perms)?;
    }

    // Release lock before rename (rename on same filesystem is atomic)
    drop(lock_file);
    std::fs::remove_file(&lock_path).ok(); // Ignore remove errors

    // Atomically rename temp to target
    std::fs::rename(&tmp_path, path)?;

    // Set permissions on the final file too
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
        let tmp = std::env::temp_dir().join(format!(
            "runie_atomic_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();

        let path = tmp.join("test.json");
        atomic_write(&path, r#"{"key": "value"}"#).unwrap();

        let mut contents = String::new();
        File::open(&path).unwrap().read_to_string(&mut contents).unwrap();
        assert_eq!(contents, r#"{"key": "value"}"#);
    }

    #[test]
    fn atomic_write_overwrites_existing() {
        let tmp = std::env::temp_dir().join(format!(
            "runie_atomic_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();

        let path = tmp.join("test.json");
        std::fs::write(&path, "old").unwrap();

        atomic_write(&path, "new").unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "new");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_sets_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = std::env::temp_dir().join(format!(
            "runie_atomic_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();

        let path = tmp.join("test.json");
        atomic_write(&path, "secret").unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        let mode = perms.mode();
        // 0o600 = read/write for owner only
        assert_eq!(mode & 0o777, 0o600, "File should have 0o600 permissions, got {:o}", mode);
    }
}
