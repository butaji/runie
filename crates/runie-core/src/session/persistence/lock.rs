//! File locking utilities using `fs2`.

use std::fs::{File, OpenOptions};
use std::path::Path;
use fs2::FileExt;

/// RAII guard for a shared (read) lock on a file.
pub struct SharedLock {
    _file: File,
}

impl Drop for SharedLock {
    fn drop(&mut self) {
        // Lock is released when File is dropped
    }
}

/// RAII guard for an exclusive (write) lock on a file.
pub struct ExclusiveLock {
    _file: File,
}

impl Drop for ExclusiveLock {
    fn drop(&mut self) {
        // Lock is released when File is dropped
    }
}

/// Acquire a shared (read) lock on a session file.
pub fn shared_lock(path: &Path) -> anyhow::Result<SharedLock> {
    let file = OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    Ok(SharedLock { _file: file })
}

/// Acquire an exclusive (write) lock on a session file.
pub fn exclusive_lock(path: &Path) -> anyhow::Result<ExclusiveLock> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(ExclusiveLock { _file: file })
}
