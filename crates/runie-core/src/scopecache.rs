//! ScopedCache - Directory-aware caching for workspace state
//!
//! Features:
//! - Directory path as scope key for workspace isolation
//! - Auto-invalidation on workspace switch (directory change)
//! - LRU eviction within each scope
//! - Thread-safe interior mutability

use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::RwLock;

/// A cache entry with metadata for invalidation
#[derive(Debug)]
struct CacheEntry<V> {
    value: V,
    access_order: usize,
}

/// ScopedCache provides directory-aware caching with auto-invalidation
///
/// When the current working directory changes, all cache entries from
/// the previous directory are automatically invalidated.
pub struct ScopedCache<K: Eq + Hash + Clone, V: Clone> {
    /// Cache storage: scope (directory) -> key -> entry
    cache: RwLock<HashMap<PathBuf, HashMap<K, CacheEntry<V>>>>,
    /// Track access order for LRU within each scope
    access_counter: RwLock<usize>,
    /// Maximum entries per scope
    max_per_scope: usize,
}

impl<K: Eq + Hash + Clone, V: Clone> ScopedCache<K, V> {
    /// Create a new ScopedCache

    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            access_counter: RwLock::new(0),
            max_per_scope: 100,
        }
    }

    /// Create with custom max entries per scope
    pub fn with_max_per_scope(max: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            access_counter: RwLock::new(0),
            max_per_scope: max,
        }
    }

    /// Get current workspace directory
    fn current_scope() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    }

    /// Get a value from cache
    pub fn get(&self, key: &K) -> Option<V> {
        let scope = Self::current_scope();
        let cache = self.cache.read().ok()?;

        if let Some(scope_cache) = cache.get(&scope) {
            if let Some(entry) = scope_cache.get(key) {
                // Clone value before dropping lock
                let value = entry.value.clone();
                drop(cache);
                self.touch_access(key);
                return Some(value);
            }
        }
        None
    }

    /// Insert a value into cache
    pub fn insert(&self, key: K, value: V) {
        let scope = Self::current_scope();
        let mut cache = self.cache.write().ok();

        if let Some(ref mut cache) = cache {
            // Get next access order counter; recover from poison
            let counter = {
                let mut cnt_guard = self.access_counter.write().unwrap_or_else(|p| p.into_inner());
                *cnt_guard += 1;
                *cnt_guard
            };

            cache.entry(scope.clone()).or_insert_with(HashMap::new);

            if let Some(scope_cache) = cache.get_mut(&scope) {
                // Evict if at capacity
                if scope_cache.len() >= self.max_per_scope {
                    Self::evict_lru(scope_cache);
                }

                scope_cache.insert(key, CacheEntry {
                    value,
                    access_order: counter,
                });
            }
        }
    }

    /// Remove a specific key from current scope
    pub fn remove(&self, key: &K) {
        let scope = Self::current_scope();
        let mut cache = self.cache.write().ok();

        if let Some(ref mut cache) = cache {
            if let Some(scope_cache) = cache.get_mut(&scope) {
                scope_cache.remove(key);
            }
        }
    }

    /// Invalidate all entries for current scope (workspace switch)
    pub fn invalidate_scope(&self) {
        let scope = Self::current_scope();
        let mut cache = self.cache.write().ok();

        if let Some(ref mut cache) = cache {
            cache.remove(&scope);
        }
    }

    /// Invalidate a specific scope
    pub fn invalidate(&self, scope: &PathBuf) {
        let mut cache = self.cache.write().ok();

        if let Some(ref mut cache) = cache {
            cache.remove(scope);
        }
    }

    /// Clear entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().ok();

        if let Some(ref mut cache) = cache {
            cache.clear();
        }
    }

    /// Get cache size for current scope
    pub fn len(&self) -> usize {
        let scope = Self::current_scope();
        let cache = self.cache.read().ok();

        cache.and_then(|c| c.get(&scope).map(|s| s.len())).unwrap_or(0)
    }

    /// Check if cache is empty for current scope
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Update access order for LRU tracking
    fn touch_access(&self, key: &K) {
        let scope = Self::current_scope();
        let mut cache = self.cache.write().ok();

        if let Some(ref mut cache) = cache {
            if let Some(scope_cache) = cache.get_mut(&scope) {
                if let Some(entry) = scope_cache.get_mut(key) {
                    if let Ok(mut cnt) = self.access_counter.write() {
                        *cnt = *cnt + 1;
                        entry.access_order = *cnt;
                    }
                }
            }
        }
    }

    /// Evict least recently used entry from a scope cache
    fn evict_lru(scope_cache: &mut HashMap<K, CacheEntry<V>>) {
        if let Some((lru_key, _)) = scope_cache
            .iter()
            .min_by_key(|(_, entry)| entry.access_order)
            .map(|(k, v)| (k.clone(), v.access_order))
        {
            scope_cache.remove(&lru_key);
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Default for ScopedCache<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for scoped cache that auto-invalidates on drop
pub struct ScopedCacheGuard<'a, K: Eq + Hash + Clone + 'a, V: Clone + 'a> {
    cache: &'a ScopedCache<K, V>,
    scope: PathBuf,
}

impl<'a, K: Eq + Hash + Clone + 'a, V: Clone + 'a> ScopedCacheGuard<'a, K, V> {
    /// Enter a scope (captures current directory)
    pub fn enter(cache: &'a ScopedCache<K, V>) -> Self {
        let scope = ScopedCache::<K, V>::current_scope();
        Self { cache, scope }
    }

    /// Get value from the guarded scope
    pub fn get(&self, key: &K) -> Option<V> {
        let cache = self.cache.cache.read().ok()?;
        cache.get(&self.scope).and_then(|s| s.get(key).map(|e| e.value.clone()))
    }

    /// Insert value into the guarded scope
    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.cache.write().ok();
        if let Some(ref mut cache) = cache {
            cache.entry(self.scope.clone()).or_insert_with(HashMap::new);
            if let Some(scope_cache) = cache.get_mut(&self.scope) {
                scope_cache.insert(key, CacheEntry {
                    value,
                    access_order: 0,
                });
            }
        }
    }
}

impl<'a, K: Eq + Hash + Clone + 'a, V: Clone + 'a> Drop for ScopedCacheGuard<'a, K, V> {
    fn drop(&mut self) {
        // Auto-invalidate on scope exit (workspace switch)
        self.cache.invalidate(&self.scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests use the OS working directory as scope key.
    // Tests that change working directory must save/restore it properly.
    // Tests here don't change directory to avoid cross-test pollution.

    #[test]
    fn test_basic_cache_operations() {
        let cache: ScopedCache<String, String> = ScopedCache::new();

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));

        cache.remove(&"key1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_invalidation() {
        let cache: ScopedCache<String, i32> = ScopedCache::new();

        cache.insert("counter".to_string(), 42);
        assert_eq!(cache.get(&"counter".to_string()), Some(42));

        cache.invalidate_scope();
        assert_eq!(cache.get(&"counter".to_string()), None);
    }

    #[test]
    fn test_lru_eviction() {
        let cache: ScopedCache<String, i32> = ScopedCache::with_max_per_scope(3);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3);

        // Access 'a' to make it recently used
        let _ = cache.get(&"a".to_string());

        // Add 'd' which should evict 'b' (least recently used)
        cache.insert("d".to_string(), 4);

        assert!(cache.get(&"a".to_string()).is_some());
        assert!(cache.get(&"b".to_string()).is_none()); // evicted
        assert!(cache.get(&"c".to_string()).is_some());
        assert!(cache.get(&"d".to_string()).is_some());
    }
}
