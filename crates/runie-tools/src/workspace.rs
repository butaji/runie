use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use runie_core::ToolError;

/// P2-8 FIX: File locking mechanism for concurrent edit safety.
/// Prevents lost updates when multiple tools edit the same file simultaneously.
#[derive(Debug, Clone)]
pub struct FileLock {
    pub path: PathBuf,
    locked: Arc<Mutex<()>>,
}

impl FileLock {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            locked: Arc::new(Mutex::new(())),
        }
    }
    
    pub fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.locked.lock().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    // P2-8 FIX: File locks for concurrent edit safety
    file_locks: Arc<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>,
}

impl Workspace {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            file_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // P2-8 FIX: Acquire an exclusive lock on a file path.
    /// Returns a guard that releases the lock when dropped.
    pub fn with_lock<F, R>(&self, path: &Path, f: F) -> Result<R, ToolError>
    where
        F: FnOnce() -> R,
    {
        // Get or create lock for this path
        let lock = {
            let mut locks = self.file_locks.lock().unwrap();
            locks
                .entry(path.to_path_buf())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        
        // Acquire exclusive lock
        let _guard = lock.lock().unwrap();
        
        // Execute the operation while holding the lock
        Ok(f())
    }
    
    // P2-8 FIX: Atomic write - write to temp file then rename.
    /// Performs an atomic write operation that prevents partial writes.
    pub async fn atomic_write(&self, path: &Path, content: &str) -> Result<(), ToolError> {
        use tokio::fs;
        use tokio::io::AsyncWriteExt;
        
        // Get parent directory and filename
        let parent = path.parent().unwrap_or(Path::new("."));
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("temp");
        
        // Create temp file in same directory for atomic rename
        let temp_path = parent.join(format!(".{}.tmp", filename));
        
        // Write to temp file
        let mut file = fs::File::create(&temp_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create temp file: {}", e)))?;
        file.write_all(content.as_bytes()).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write temp file: {}", e)))?;
        file.sync_all().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to sync temp file: {}", e)))?;
        drop(file);
        
        // Atomic rename (on POSIX systems, rename is atomic if src and dst are on same filesystem)
        fs::rename(&temp_path, path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to rename temp file: {}", e)))?;
        
        Ok(())
    }

    pub fn resolve(&self, path: &str) -> Result<PathBuf, ToolError> {
        let resolved = self.root.join(path);
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(ToolError::InvalidArguments(format!(
                "Path '{}' is outside workspace", path
            )))
        }
    }

    pub fn contains(&self, path: &Path) -> bool {
        let canonical_root = match self.root.canonicalize() {
            Ok(root) => root,
            Err(_) => return false,
        };

        let absolute_path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        let normalized = if absolute_path.is_relative() {
            canonical_root.join(absolute_path)
        } else {
            absolute_path
        };

        normalized.starts_with(&canonical_root)
    }
}
