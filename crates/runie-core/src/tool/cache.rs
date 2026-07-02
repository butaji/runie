//! Tool result cache with TTL expiry.
//!
//! Stores results of read-only tool calls keyed by `tool_name + SHA-256(args)`.
//! Cache entries carry a configurable TTL; expired entries are evicted lazily
//! on access and a background task periodically sweeps stale entries.
//!
//! Only read-only tools (`read_file`, `list_dir`, `grep`, `find`, `fetch_docs`,
//! `search`, `find_definitions`) are cached. Write tools (`bash`, `write_file`,
//! `edit_file`) are never cached because they have side effects.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Read-only tool names that can be cached.
pub const CACHEABLE_TOOL_NAMES: &[&str] = &[
    "read_file",
    "list_dir",
    "grep",
    "find",
    "fetch_docs",
    "search",
    "find_definitions",
];

/// Check if a tool can be cached.
#[inline]
pub fn is_cacheable_tool(name: &str) -> bool {
    CACHEABLE_TOOL_NAMES.contains(&name)
}

/// Cached tool result entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub tool_name: String,
    pub output: String,
    pub bytes_transferred: Option<u64>,
    /// Unix timestamp in seconds when this entry was cached.
    pub cached_at: u64,
}

/// Tool result cache with TTL-based expiry.
pub struct ToolResultCache {
    /// In-memory cache: cache_key -> CacheEntry
    cache: RwLock<HashMap<u64, CacheEntry>>,
    /// TTL in seconds. Zero disables the cache.
    ttl_secs: u64,
}

impl ToolResultCache {
    /// Create a new cache with the given TTL in seconds.
    pub fn new(ttl_secs: u64) -> Arc<Self> {
        Arc::new(Self {
            cache: RwLock::new(HashMap::new()),
            ttl_secs,
        })
    }

    /// Spawn a background sweep task that evicts expired entries every `max(1, ttl_secs / 2)`.
    ///
    /// Returns a `JoinHandle` for the spawned task.
    pub fn spawn_sweep(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let cache = Arc::clone(self);
        // Ensure interval is at least 1 second to avoid tokio panic.
        let sweep_interval_secs = cache.ttl_secs.saturating_sub(cache.ttl_secs / 2).max(1);
        let sweep_interval = Duration::from_secs(sweep_interval_secs);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(sweep_interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let removed = cache.evict_expired();
                        if removed > 0 {
                            tracing::debug!(removed, "tool-result-cache: evicted expired entries");
                        }
                    }
                }
            }
        })
    }

    /// Compute a 64-bit cache key from tool name and JSON args.
    ///
    /// Uses the first 8 bytes of SHA-256 of the canonical JSON of
    /// `{"tool": name, "args": args}`. This is deterministic and has
    /// negligible collision risk for the expected dataset size.
    pub fn compute_key(tool_name: &str, args: &serde_json::Value) -> u64 {
        let payload = serde_json::json!({
            "tool": tool_name,
            "args": args,
        });
        let canonical = serde_json::to_string(&payload).unwrap_or_default();
        let hash = Sha256::digest(canonical.as_bytes());
        u64::from_le_bytes([hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]])
    }

    /// Check if the cache is enabled (TTL > 0).
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.ttl_secs > 0
    }

    /// Get a cached result, evicting expired entries lazily.
    ///
    /// Returns `None` if the entry is absent or expired.
    pub fn get(&self, key: u64) -> Option<CacheEntry> {
        if !self.is_enabled() {
            return None;
        }
        let entry = self.cache.read().get(&key)?.clone();
        if self.is_expired(&entry) {
            self.cache.write().remove(&key);
            return None;
        }
        Some(entry)
    }

    /// Store a result in the cache.
    ///
    /// No-op if the cache is disabled.
    pub fn put(&self, key: u64, entry: CacheEntry) {
        if !self.is_enabled() {
            return;
        }
        self.cache.write().insert(key, entry);
    }

    /// Evict all expired entries from the cache.
    ///
    /// Returns the number of entries removed.
    pub fn evict_expired(&self) -> usize {
        if !self.is_enabled() {
            return 0;
        }
        let now = current_unix_secs();
        let mut removed = 0;
        let mut guard = self.cache.write();
        guard.retain(|_, entry| {
            let expired = now.saturating_sub(entry.cached_at) >= self.ttl_secs;
            if expired {
                removed += 1;
            }
            !expired
        });
        removed
    }

    /// Returns the current number of cached entries (for testing/debugging).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.cache.read().len()
    }

    #[inline]
    fn is_expired(&self, entry: &CacheEntry) -> bool {
        current_unix_secs().saturating_sub(entry.cached_at) >= self.ttl_secs
    }
}

/// Get current Unix timestamp in seconds.
fn current_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Key computation ───────────────────────────────────────────────────────────

    #[test]
    fn cache_key_deterministic() {
        let args = serde_json::json!({"path": "Cargo.toml"});
        let key1 = ToolResultCache::compute_key("read_file", &args);
        let key2 = ToolResultCache::compute_key("read_file", &args);
        assert_eq!(key1, key2, "same name+args must produce same key");
    }

    #[test]
    fn cache_key_different_for_different_args() {
        let args1 = serde_json::json!({"path": "Cargo.toml"});
        let args2 = serde_json::json!({"path": "README.md"});
        let key1 = ToolResultCache::compute_key("read_file", &args1);
        let key2 = ToolResultCache::compute_key("read_file", &args2);
        assert_ne!(key1, key2, "different args must produce different keys");
    }

    #[test]
    fn cache_key_different_for_different_tool() {
        let args = serde_json::json!({"path": "."});
        let key1 = ToolResultCache::compute_key("read_file", &args);
        let key2 = ToolResultCache::compute_key("list_dir", &args);
        assert_ne!(key1, key2, "different tool names must produce different keys");
    }

    #[test]
    fn cache_key_stable_across_json_order() {
        // serde_json canonicalization must produce identical output
        let args1 = serde_json::json!({"a": 1, "b": 2});
        let args2 = serde_json::json!({"b": 2, "a": 1});
        let key1 = ToolResultCache::compute_key("grep", &args1);
        let key2 = ToolResultCache::compute_key("grep", &args2);
        assert_eq!(key1, key2, "canonical JSON must be order-independent");
    }

    // ── Basic get/put ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn cache_hit_returns_stored_output() {
        let cache = ToolResultCache::new(300);
        let key = ToolResultCache::compute_key("read_file", &serde_json::json!({"path": "Cargo.toml"}));
        let entry = CacheEntry {
            tool_name: "read_file".to_string(),
            output: "version = \"0.1\"".to_string(),
            bytes_transferred: Some(42),
            cached_at: current_unix_secs(),
        };

        cache.put(key, entry.clone());
        let found = cache.get(key);
        assert!(found.is_some(), "cache hit must return entry");
        assert_eq!(found.unwrap().output, "version = \"0.1\"");
    }

    #[tokio::test]
    async fn cache_miss_returns_none() {
        let cache = ToolResultCache::new(300);
        let key = ToolResultCache::compute_key("bash", &serde_json::json!({"cmd": "echo hi"}));
        let found = cache.get(key);
        assert!(found.is_none(), "non-existent key must return None");
    }

    // ── Expiry ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn cache_expiry_returns_none_after_ttl() {
        let cache = ToolResultCache::new(1); // 1-second TTL
        let key = ToolResultCache::compute_key("list_dir", &serde_json::json!({"path": "."}));
        let entry = CacheEntry {
            tool_name: "list_dir".to_string(),
            output: "src/\nCargo.toml".to_string(),
            bytes_transferred: None,
            cached_at: current_unix_secs() - 2, // 2 seconds ago → expired
        };

        cache.put(key, entry);
        let found = cache.get(key);
        assert!(found.is_none(), "expired entry must return None");
    }

    #[tokio::test]
    async fn evict_expired_removes_stale_entries() {
        let cache = ToolResultCache::new(1);

        let key1 = ToolResultCache::compute_key("list_dir", &serde_json::json!({"path": "a"}));
        let key2 = ToolResultCache::compute_key("list_dir", &serde_json::json!({"path": "b"}));

        // Fresh entry
        cache.put(key1, CacheEntry {
            tool_name: "list_dir".to_string(),
            output: "a".to_string(),
            bytes_transferred: None,
            cached_at: current_unix_secs(),
        });
        // Stale entry
        cache.put(key2, CacheEntry {
            tool_name: "list_dir".to_string(),
            output: "b".to_string(),
            bytes_transferred: None,
            cached_at: current_unix_secs() - 10,
        });

        assert_eq!(cache.len(), 2);
        let removed = cache.evict_expired();
        assert_eq!(removed, 1);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(key1).is_some());
        assert!(cache.get(key2).is_none());
    }

    #[tokio::test]
    async fn sweep_removes_all_expired() {
        let cache = ToolResultCache::new(1);

        for i in 0..5u8 {
            let key = ToolResultCache::compute_key("grep", &serde_json::json!({"pattern": i}));
            let cached_at = if i < 3 {
                current_unix_secs() - 10 // expired
            } else {
                current_unix_secs() // fresh
            };
            cache.put(key, CacheEntry {
                tool_name: "grep".to_string(),
                output: format!("line{}", i),
                bytes_transferred: None,
                cached_at,
            });
        }

        assert_eq!(cache.len(), 5);
        let removed = cache.evict_expired();
        assert_eq!(removed, 3);
        assert_eq!(cache.len(), 2);
    }

    // ── Disabled cache ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn cache_disabled_when_ttl_zero() {
        let cache = ToolResultCache::new(0);
        let key = ToolResultCache::compute_key("read_file", &serde_json::json!({"path": "Cargo.toml"}));
        let entry = CacheEntry {
            tool_name: "read_file".to_string(),
            output: "version".to_string(),
            bytes_transferred: None,
            cached_at: current_unix_secs(),
        };

        assert!(!cache.is_enabled());
        cache.put(key, entry);
        let found = cache.get(key);
        assert!(found.is_none(), "disabled cache must always return None");
        assert_eq!(cache.evict_expired(), 0, "no-op eviction on disabled cache");
    }

    // ── Smoke test ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn tool_cache_smoke_test() {
        let cache = ToolResultCache::new(300);

        let key = ToolResultCache::compute_key("list_dir", &serde_json::json!({"path": "."}));
        let entry = CacheEntry {
            tool_name: "list_dir".to_string(),
            output: "src\ntests".to_string(),
            bytes_transferred: Some(8),
            cached_at: current_unix_secs(),
        };

        cache.put(key, entry);
        let result = cache.get(key).expect("should retrieve cached entry");
        assert_eq!(result.tool_name, "list_dir");
        assert_eq!(result.output, "src\ntests");
        assert_eq!(result.bytes_transferred, Some(8));
    }

    // ── spawn_sweep ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn spawn_sweep_runs_and_evicts() {
        let cache = ToolResultCache::new(1);
        let handle = cache.spawn_sweep();

        // Add stale entries
        for i in 0..3u8 {
            let key = ToolResultCache::compute_key("grep", &serde_json::json!({"i": i}));
            cache.put(key, CacheEntry {
                tool_name: "grep".to_string(),
                output: format!("line{}", i),
                bytes_transferred: None,
                cached_at: current_unix_secs() - 10,
            });
        }

        // Wait for sweep interval (max(1, 1/2) = 1s) + tolerance
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(cache.len(), 0, "sweep should have removed all stale entries");

        handle.abort();
    }

    // ── is_cacheable_tool ───────────────────────────────────────────────────────

    #[test]
    fn is_cacheable_tool_returns_true_for_read_only_tools() {
        for name in CACHEABLE_TOOL_NAMES {
            assert!(is_cacheable_tool(name), "{} should be cacheable", name);
        }
    }

    #[test]
    fn is_cacheable_tool_returns_false_for_write_tools() {
        assert!(!is_cacheable_tool("bash"));
        assert!(!is_cacheable_tool("write_file"));
        assert!(!is_cacheable_tool("edit_file"));
        assert!(!is_cacheable_tool("unknown_tool"));
    }
}
