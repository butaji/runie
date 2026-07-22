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
///
/// The advisory lock is held through the entire operation (temp write + rename) to
/// prevent concurrent writers from racing during the critical section.
pub fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    // Get the parent directory
    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;

    // Create temp file in same directory for atomic rename using tempfile
    let tmp = NamedTempFile::new_in(parent).map_err(io::Error::other)?;

    // Acquire exclusive advisory lock on target file
    let lock_path = path.with_extension("lock");
    let lock_file = File::create(&lock_path)?;
    fs2::FileExt::lock_exclusive(&lock_file)?;

    // Hold lock through entire write + rename operation.
    // Using a scope ensures the lock is dropped AFTER persist completes.
    let result = (|| {
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

        // Atomically rename temp to target while still holding the lock
        tmp.persist(path).map_err(io::Error::other)?;

        // Set permissions on the final file too (belt and suspenders)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)?;
        }

        Ok::<(), io::Error>(())
    })();

    // Release lock after rename completes (or fails)
    drop(lock_file);
    std::fs::remove_file(&lock_path).ok(); // Ignore remove errors

    result
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
        File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();
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
        assert_eq!(
            mode & 0o777,
            0o600,
            "File should have 0o600 permissions, got {:o}",
            mode
        );
    }

    /// Stress test: multiple concurrent writers should not corrupt the file.
    /// The final file content must be exactly one of the written values.
    #[cfg(unix)]
    #[test]
    #[allow(clippy::too_many_lines)]
    fn atomic_write_concurrent_stress() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let tmp_dir = std::env::temp_dir().join(format!(
            "runie_atomic_concurrent_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp_dir).unwrap();

        let path = tmp_dir.join("concurrent.json");
        let num_writers = 8;
        let writes_per_writer = 20;
        let total_writes = num_writers * writes_per_writer;

        // Track all possible written values
        let all_values: Arc<Vec<String>> = Arc::new((0..total_writes).map(|i| format!("value_{}", i)).collect());

        // Shared counter for assigning unique values
        let counter = Arc::new(AtomicUsize::new(0));

        // Shared path for all writers
        let path = Arc::new(path);

        // Spawn concurrent writers
        let handles: Vec<_> = (0..num_writers)
            .map(|_| {
                let counter = Arc::clone(&counter);
                let path = Arc::clone(&path);
                let all_values = Arc::clone(&all_values);

                thread::spawn(move || {
                    for _ in 0..writes_per_writer {
                        let idx = counter.fetch_add(1, Ordering::Relaxed);
                        let value = all_values[idx].clone();
                        let _ = atomic_write(&path, &value);
                    }
                })
            })
            .collect();

        // Wait for all writers to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Read final content - it must be exactly one of the written values
        let final_content = std::fs::read_to_string(&*path).unwrap();

        // The final content must be one of the values we wrote
        assert!(
            all_values.contains(&final_content),
            "Final content '{}' is not one of the expected values",
            final_content
        );
    }
}
