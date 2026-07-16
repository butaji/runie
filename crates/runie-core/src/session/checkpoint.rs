//! Turn-boundary filesystem snapshots with git/hunk deltas.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// A single checkpoint capturing workspace state at a turn boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindCheckpoint {
    pub prompt_index: usize,
    pub timestamp: Instant,
    pub fs_snapshot: FsSnapshot,
    pub git_state: Option<GitState>,
    pub hunks: Option<HunkDelta>,
    pub agent_id: Option<String>,
}

impl RewindCheckpoint {
    /// Age of this checkpoint relative to another instant
    pub fn age(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.timestamp)
    }

    /// Check if this checkpoint is older than the given duration
    pub fn is_older_than(&self, now: Instant, duration: Duration) -> bool {
        self.age(now) > duration
    }
}

/// Filesystem snapshot - tracks modified file hashes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FsSnapshot {
    files: HashMap<PathBuf, FileHash>,
}

impl FsSnapshot {
    /// Capture filesystem state for a directory
    pub fn capture_dir(dir: &Path) -> std::io::Result<Self> {
        let mut files = HashMap::new();
        Self::walk_dir(dir, dir, &mut files)?;
        Ok(Self { files })
    }

    fn walk_dir(base: &Path, dir: &Path, files: &mut HashMap<PathBuf, FileHash>) -> std::io::Result<()> {
        if Self::is_hidden(dir) {
            return Ok(());
        }

        // Skip common ignored directories
        let skip_dirs = ["node_modules", "target", ".git", "dist", "build", ".cache"];
        if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
            if skip_dirs.contains(&name) {
                return Ok(());
            }
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
            Err(e) => return Err(e),
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if Self::is_hidden(&path) {
                continue;
            }

            if path.is_file() {
                if let Ok(hash) = Self::file_hash(&path) {
                    let rel = path.strip_prefix(base).unwrap_or(&path).to_path_buf();
                    files.insert(rel, hash);
                }
            } else if path.is_dir() && !path.is_symlink() {
                Self::walk_dir(base, &path, files)?;
            }
        }
        Ok(())
    }

    fn is_hidden(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
    }

    fn file_hash(path: &Path) -> std::io::Result<FileHash> {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let content = std::fs::read(path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);

        // Also hash the file metadata for symlink detection
        if let Ok(metadata) = path.metadata() {
            metadata.len().hash(&mut hasher);
            metadata.permissions().readonly().hash(&mut hasher);
        }

        Ok(hasher.finish())
    }

    /// Compute diff between two snapshots
    pub fn diff(&self, other: &FsSnapshot) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        // Files that changed
        for (path, hash) in &self.files {
            if other.files.get(path) != Some(hash) {
                changed.push(path.clone());
            }
        }

        // Files that were deleted
        for path in other.files.keys() {
            if !self.files.contains_key(path) {
                changed.push(path.clone());
            }
        }

        changed.sort();
        changed.dedup();
        changed
    }

    /// Get the number of tracked files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Check if a specific file changed
    pub fn file_changed(&self, other: &FsSnapshot, path: &Path) -> bool {
        self.files.get(path) != other.files.get(path)
    }
}

/// File hash type
type FileHash = u64;

/// Git state at checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    pub head: String,
    pub branch: Option<String>,
    pub staged_files: Vec<PathBuf>,
    pub untracked_files: Vec<PathBuf>,
    pub modified_files: Vec<PathBuf>,
}

impl GitState {
    /// Capture current git state
    pub fn capture(cwd: &Path) -> Option<Self> {
        use std::process::Command;

        let head = Self::git_output(cwd, &["rev-parse", "HEAD"])?;
        let branch = Self::git_output(cwd, &["branch", "--show-current"]);
        let staged = Self::git_output_list(cwd, &["diff", "--name-only", "--cached"]);
        let untracked = Self::git_output_list(cwd, &["ls-files", "--others", "--exclude-standard"]);
        let modified = Self::git_output_list(cwd, &["diff", "--name-only"]);

        Some(Self {
            head,
            branch,
            staged_files: staged,
            untracked_files: untracked,
            modified_files: modified,
        })
    }

    fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    fn git_output_list(cwd: &Path, args: &[&str]) -> Vec<PathBuf> {
        use std::process::Command;

        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .ok()
            .filter(|o| o.status.success())?;

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect()
    }

    /// Check if there are uncommitted changes
    pub fn has_changes(&self) -> bool {
        !self.staged_files.is_empty()
            || !self.untracked_files.is_empty()
            || !self.modified_files.is_empty()
    }

    /// Generate git commands to restore this state
    pub fn restore_commands(&self) -> Vec<String> {
        let mut cmds = Vec::new();

        // Reset to this commit
        cmds.push(format!("git reset --soft {}", self.head));

        // Unstage everything
        cmds.push("git reset HEAD".to_string());

        cmds
    }
}

/// Incremental hunk tracker delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunkDelta {
    pub turn_index: usize,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: Vec<PathBuf>,
}

impl HunkDelta {
    /// Calculate net change
    pub fn net_change(&self) -> i32 {
        self.additions as i32 - self.deletions as i32
    }

    /// Check if this is a significant change
    pub fn is_significant(&self) -> bool {
        self.additions > 10 || self.deletions > 5
    }
}

/// Checkpoint store with durable on-disk storage
pub struct CheckpointStore {
    dir: PathBuf,
    checkpoints: Arc<RwLock<Vec<RewindCheckpoint>>>,
    max_checkpoints: usize,
    cwd: PathBuf,
}

impl CheckpointStore {
    /// Create a new checkpoint store
    pub fn new(cwd: PathBuf) -> anyhow::Result<Self> {
        let dir = cwd.join(".runie/rewind-checkpoints");
        std::fs::create_dir_all(&dir)?;

        let store = Self {
            dir,
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            max_checkpoints: 64,
            cwd,
        };

        store.load_from_disk()?;
        Ok(store)
    }

    /// Get the checkpoint directory
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn load_from_disk(&self) -> anyhow::Result<()> {
        let path = self.dir.join("checkpoints.json");
        if !path.exists() {
            return Ok(());
        }

        let data = std::fs::read(&path)?;
        let loaded: Vec<RewindCheckpoint> = serde_json::from_slice(&data)?;

        // Update timestamps to be relative to program start
        let now = Instant::now();
        let checkpoints = loaded.into_iter().map(|mut c| {
            // Reset timestamp on load (persisted timestamps are absolute)
            c.timestamp = now;
            c
        }).collect();

        *self.checkpoints.write() = checkpoints;
        Ok(())
    }

    fn persist_to_disk(&self, checkpoints: &[RewindCheckpoint]) -> anyhow::Result<()> {
        let tmp = self.dir.join("checkpoints.tmp");
        let data = serde_json::to_vec_pretty(checkpoints)?;
        std::fs::write(&tmp, &data)?;
        std::fs::rename(&tmp, self.dir.join("checkpoints.json"))?;
        Ok(())
    }

    /// Capture a checkpoint at the current state
    pub fn capture(&self, prompt_index: usize) -> anyhow::Result<()> {
        let fs = FsSnapshot::capture_dir(&self.cwd)?;
        let git = GitState::capture(&self.cwd);

        let checkpoint = RewindCheckpoint {
            prompt_index,
            timestamp: Instant::now(),
            fs_snapshot: fs,
            git_state: git,
            hunks: None,
            agent_id: None,
        };

        let mut checkpoints = self.checkpoints.write();
        checkpoints.push(checkpoint);

        // Prune old checkpoints
        while checkpoints.len() > self.max_checkpoints {
            checkpoints.remove(0);
        }

        self.persist_to_disk(&checkpoints)?;
        Ok(())
    }

    /// Capture with agent ID
    pub fn capture_with_agent(&self, prompt_index: usize, agent_id: String) -> anyhow::Result<()> {
        let fs = FsSnapshot::capture_dir(&self.cwd)?;
        let git = GitState::capture(&self.cwd);

        let checkpoint = RewindCheckpoint {
            prompt_index,
            timestamp: Instant::now(),
            fs_snapshot: fs,
            git_state: git,
            hunks: None,
            agent_id: Some(agent_id),
        };

        let mut checkpoints = self.checkpoints.write();
        checkpoints.push(checkpoint);

        while checkpoints.len() > self.max_checkpoints {
            checkpoints.remove(0);
        }

        self.persist_to_disk(&checkpoints)?;
        Ok(())
    }

    /// Get all checkpoints
    pub fn checkpoints(&self) -> Vec<RewindCheckpoint> {
        self.checkpoints.read().clone()
    }

    /// Get the most recent checkpoint
    pub fn latest(&self) -> Option<RewindCheckpoint> {
        self.checkpoints.read().last().cloned()
    }

    /// Get checkpoint by index
    pub fn get(&self, index: usize) -> Option<RewindCheckpoint> {
        self.checkpoints.read().get(index).cloned()
    }

    /// Get checkpoint by prompt index
    pub fn by_prompt_index(&self, prompt_index: usize) -> Option<RewindCheckpoint> {
        self.checkpoints.read()
            .iter()
            .find(|c| c.prompt_index == prompt_index)
            .cloned()
    }

    /// Create a rewind plan to restore state at target index
    pub fn rewind_to(&self, target_index: usize) -> anyhow::Result<RewindPlan> {
        let checkpoints = self.checkpoints.read();

        let target = checkpoints.iter().find(|c| c.prompt_index == target_index);
        let Some(target) = target else {
            anyhow::bail!("No checkpoint found for prompt_index {}", target_index);
        };

        // Find the last checkpoint before target
        let prev = checkpoints.iter()
            .filter(|c| c.prompt_index < target_index)
            .last();

        let Some(prev) = prev else {
            anyhow::bail!("No previous checkpoint found");
        };

        // Compute diff between prev and target
        let files_to_restore = target.fs_snapshot.diff(&prev.fs_snapshot);

        // Collect git commands
        let mut git_commands = Vec::new();
        if let (Some(target_git), Some(prev_git)) = (&target.git_state, &prev.git_state) {
            if target_git.head != prev_git.head {
                git_commands.push(format!("git stash"));
                git_commands.push(format!("git reset --soft {}", target_git.head));
            }
        }

        Ok(RewindPlan {
            prompt_index: target.prompt_index,
            git_commands,
            files_to_restore,
            files_to_delete: Vec::new(),
        })
    }

    /// Clear all checkpoints
    pub fn clear(&self) -> anyhow::Result<()> {
        let mut checkpoints = self.checkpoints.write();
        checkpoints.clear();
        self.persist_to_disk(&checkpoints)?;
        Ok(())
    }

    /// Get checkpoint count
    pub fn len(&self) -> usize {
        self.checkpoints.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.checkpoints.read().is_empty()
    }

    /// Set maximum checkpoints
    pub fn set_max_checkpoints(&mut self, max: usize) {
        self.max_checkpoints = max;
    }
}

/// Plan for rewinding to a previous state
#[derive(Debug, Clone)]
pub struct RewindPlan {
    pub prompt_index: usize,
    pub git_commands: Vec<String>,
    pub files_to_restore: Vec<PathBuf>,
    pub files_to_delete: Vec<PathBuf>,
}

impl RewindPlan {
    /// Get the number of files affected
    pub fn affected_files(&self) -> usize {
        self.files_to_restore.len() + self.files_to_delete.len()
    }

    /// Check if this is a simple rewind (just git reset)
    pub fn is_simple(&self) -> bool {
        self.files_to_restore.is_empty() && self.files_to_delete.is_empty()
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        if self.is_simple() {
            if !self.git_commands.is_empty() {
                return format!("Git reset to checkpoint {}", self.prompt_index);
            }
            return format!("Rewind to checkpoint {}", self.prompt_index);
        }

        format!(
            "Rewind to checkpoint {}: {} files to restore, {} to delete",
            self.prompt_index,
            self.files_to_restore.len(),
            self.files_to_delete.len()
        )
    }
}

/// Checkpoint manager for automatic checkpointing
pub struct CheckpointManager {
    store: CheckpointStore,
    auto_checkpoint: bool,
    min_interval: Duration,
    last_checkpoint: Option<Instant>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(cwd: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            store: CheckpointStore::new(cwd)?,
            auto_checkpoint: true,
            min_interval: Duration::from_secs(30),
            last_checkpoint: None,
        })
    }

    /// Capture a checkpoint if enough time has passed
    pub fn maybe_capture(&mut self, prompt_index: usize) -> anyhow::Result<bool> {
        if !self.auto_checkpoint {
            return Ok(false);
        }

        let now = Instant::now();

        // Check minimum interval
        if let Some(last) = self.last_checkpoint {
            if now.duration_since(last) < self.min_interval {
                return Ok(false);
            }
        }

        self.store.capture(prompt_index)?;
        self.last_checkpoint = Some(now);
        Ok(true)
    }

    /// Force capture a checkpoint
    pub fn capture(&mut self, prompt_index: usize) -> anyhow::Result<()> {
        self.store.capture(prompt_index)?;
        self.last_checkpoint = Some(Instant::now());
        Ok(())
    }

    /// Enable/disable auto checkpointing
    pub fn set_auto_checkpoint(&mut self, enabled: bool) {
        self.auto_checkpoint = enabled;
    }

    /// Set minimum interval between checkpoints
    pub fn set_min_interval(&mut self, interval: Duration) {
        self.min_interval = interval;
    }

    /// Get the underlying store
    pub fn store(&self) -> &CheckpointStore {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_fs_snapshot_capture() {
        let dir = env::current_dir().unwrap();
        let snapshot = FsSnapshot::capture_dir(&dir).unwrap();
        assert!(snapshot.file_count() > 0);
    }

    #[test]
    fn test_fs_snapshot_diff() {
        let dir = env::current_dir().unwrap();
        let snap1 = FsSnapshot::capture_dir(&dir).unwrap();
        let snap2 = FsSnapshot::capture_dir(&dir).unwrap();

        // Same state should have no diff
        let diff = snap1.diff(&snap2);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_git_state_capture() {
        let dir = env::current_dir().unwrap();
        let state = GitState::capture(&dir);
        assert!(state.is_some());
    }

    #[test]
    fn test_checkpoint_store() {
        let temp_dir = std::env::temp_dir().join("checkpoint_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let store = CheckpointStore::new(temp_dir.clone()).unwrap();
        assert!(store.is_empty());

        store.capture(0).unwrap();
        assert_eq!(store.len(), 1);

        let latest = store.latest().unwrap();
        assert_eq!(latest.prompt_index, 0);

        // Cleanup
        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_rewind_plan() {
        let plan = RewindPlan {
            prompt_index: 5,
            git_commands: vec!["git stash".to_string()],
            files_to_restore: vec![PathBuf::from("test.txt")],
            files_to_delete: vec![],
        };

        assert_eq!(plan.affected_files(), 1);
        assert!(!plan.is_simple());
        assert!(plan.summary().contains("Rewind"));
    }
}
