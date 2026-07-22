//! Worktree Pool — Bounded pool for fast subagent isolation.
//!
//! Pre-creates git worktrees in background, provides acquire/release API,
//! cleans up stale worktrees after configurable timeout.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
const DEFAULT_POOL_SIZE: usize = 3;
#[allow(dead_code)]
const DEFAULT_STALE_TIMEOUT_MINS: u64 = 30;

const READY_SUFFIX: &str = ".ready";
const CLAIMED_SUFFIX: &str = ".claimed";
const LAST_USED_SUFFIX: &str = ".last_used";

/// Handle to an acquired worktree. Drop to release back to pool.
pub struct WorktreeHandle {
    pub path: PathBuf,
    pool: Arc<WorktreePool>,
}

impl WorktreeHandle {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorktreeHandle {
    fn drop(&mut self) {
        let path = self.path.clone();
        let pool = self.pool.clone();
        tokio::spawn(async move {
            pool.release_worktree_internal(&path).await;
        });
    }
}

/// Bounded worktree pool for subagent isolation.
pub struct WorktreePool {
    instance_id: String,
    pool_size: usize,
    stale_timeout: Duration,
    worktrees_dir: PathBuf,
    fill_notify: Arc<Notify>,
    ready_notify: Arc<Notify>,
    cancel: CancellationToken,
    _fill_handle: tokio::task::JoinHandle<()>,
}

impl WorktreePool {
    /// Create new pool with bounded size.
    pub fn new(source_path: PathBuf, pool_size: usize, stale_timeout_mins: u64) -> Self {
        let instance_id = uuid::Uuid::new_v4().to_string();
        let worktrees_dir = pool_base_directory().join(&instance_id);
        std::fs::create_dir_all(&worktrees_dir).ok();
        std::fs::write(worktrees_dir.join(".pid"), std::process::id().to_string()).ok();

        let fill_notify = Arc::new(Notify::new());
        let ready_notify = Arc::new(Notify::new());
        let cancel = CancellationToken::new();
        let size = pool_size;

        let fill_handle = {
            let source = source_path.clone();
            let dir = worktrees_dir.clone();
            let cancel = cancel.clone();
            let fill_notify = fill_notify.clone();
            let ready_notify = ready_notify.clone();

            tokio::task::spawn(async move {
                Self::fill_loop(source, dir, size, cancel, fill_notify, ready_notify).await;
            })
        };

        Self {
            instance_id,
            pool_size,
            stale_timeout: Duration::from_secs(stale_timeout_mins * 60),
            worktrees_dir,
            fill_notify,
            ready_notify,
            cancel,
            _fill_handle: fill_handle,
        }
    }

    /// Acquire a worktree from the pool.
    pub async fn acquire_worktree(&self) -> anyhow::Result<WorktreeHandle> {
        loop {
            if let Some(path) = self.try_claim_ready() {
                return Ok(WorktreeHandle { path, pool: self.clone_inner() });
            }

            if self.count_in_progress() == 0 {
                anyhow::bail!("no worktrees available");
            }

            tokio::select! {
                _ = self.ready_notify.notified() => continue,
                _ = self.cancel.cancelled() => anyhow::bail!("pool shutting down"),
            }
        }
    }

    fn try_claim_ready(&self) -> Option<PathBuf> {
        let entries = std::fs::read_dir(&self.worktrees_dir).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(READY_SUFFIX) {
                continue;
            }
            let base = &name[..name.len() - READY_SUFFIX.len()];
            let worktree_dir = self.worktrees_dir.join(base);
            let ready_marker = entry.path();
            let claimed_marker = marker_path(&worktree_dir, CLAIMED_SUFFIX);

            if std::fs::rename(&ready_marker, &claimed_marker).is_ok() {
                return Some(worktree_dir);
            }
        }
        None
    }

    fn count_in_progress(&self) -> usize {
        let entries = match std::fs::read_dir(&self.worktrees_dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };
        entries.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(READY_SUFFIX)).count()
    }

    async fn release_worktree_internal(&self, path: &Path) {
        let _ = tokio::fs::remove_file(marker_path(path, CLAIMED_SUFFIX)).await;
        let p = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("git")
                .args(["reset", "--hard", "HEAD"])
                .current_dir(&p)
                .status();
            let _ = std::process::Command::new("git")
                .args(["clean", "-fdx"])
                .current_dir(&p)
                .status();
        }).await.ok();

        let ready = marker_path(path, READY_SUFFIX);
        let _ = std::fs::write(&ready, "");
        self.fill_notify.notify_one();
    }

    fn clone_inner(&self) -> Arc<WorktreePool> {
        Arc::new(WorktreePool {
            instance_id: self.instance_id.clone(),
            pool_size: self.pool_size,
            stale_timeout: self.stale_timeout,
            worktrees_dir: self.worktrees_dir.clone(),
            fill_notify: self.fill_notify.clone(),
            ready_notify: self.ready_notify.clone(),
            cancel: self.cancel.clone(),
            _fill_handle: tokio::task::spawn(async {}),
        })
    }

    /// Release a worktree back to the pool.
    pub fn release_worktree(&self, handle: WorktreeHandle) {
        drop(handle);
    }

    /// Count ready worktrees in pool.
    pub fn count_ready(&self) -> usize {
        self.count_in_progress()
    }

    /// Clean up stale worktrees.
    pub async fn cleanup_stale(&self) -> usize {
        let timeout = self.stale_timeout;
        let entries = match std::fs::read_dir(&self.worktrees_dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let mut removed = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let last_used_path = marker_path(&path, LAST_USED_SUFFIX);
            if let Ok(content) = std::fs::read_to_string(&last_used_path) {
                if let Ok(ts) = content.parse::<u64>() {
                    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH + Duration::from_secs(ts));
                    if elapsed.map(|d| d > timeout).unwrap_or(false) {
                        let _ = tokio::fs::remove_dir_all(&path).await;
                        removed += 1;
                    }
                }
            }
        }
        removed
    }

    async fn fill_loop(
        source: PathBuf,
        worktrees_dir: PathBuf,
        pool_size: usize,
        cancel: CancellationToken,
        fill_notify: Arc<Notify>,
        ready_notify: Arc<Notify>,
    ) {
        let hard_cap = pool_size.saturating_mul(2).max(6);
        let mut created = 0usize;

        loop {
            let current = Self::count_dir_worktrees(&worktrees_dir);
            if current >= pool_size {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = fill_notify.notified() => continue,
                }
            }

            if created >= hard_cap {
                cancel.cancelled().await;
                return;
            }

            let pool_id = uuid::Uuid::new_v4().to_string();
            let worktree_path = worktrees_dir.join(&pool_id);

            let result = tokio::task::spawn_blocking({
                let src = source.clone();
                let dst = worktree_path.clone();
                move || {
                    std::process::Command::new("git")
                        .args(["worktree", "add", "--detach", "--checkout", &dst.to_string_lossy()])
                        .current_dir(&src)
                        .output()
                }
            }).await;

            if !result.map(|r| r.is_ok()).unwrap_or(false) {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            let ready = marker_path(&worktree_path, READY_SUFFIX);
            let _ = std::fs::write(&ready, "");
            let last_used = marker_path(&worktree_path, LAST_USED_SUFFIX);
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().to_string();
            let _ = std::fs::write(&last_used, now);

            ready_notify.notify_waiters();
            created += 1;
        }
    }

    fn count_dir_worktrees(dir: &Path) -> usize {
        std::fs::read_dir(dir).ok()
            .map(|e| e.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0)
    }
}

fn pool_base_directory() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("runie")
        .join("worktree_pool")
}

fn marker_path(worktree_dir: &Path, suffix: &str) -> PathBuf {
    let mut p = worktree_dir.as_os_str().to_owned();
    p.push(suffix);
    PathBuf::from(p)
}
