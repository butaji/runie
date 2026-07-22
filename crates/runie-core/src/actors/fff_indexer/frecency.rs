//! Frecency store — MRU-based file access scoring.
//!
//! A simple MRU-based frecency store. Each file gets a score =
//! (access_count * recency_boost) where recency_boost decays with time
//! since last access.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A simple MRU-based frecency store.
/// Each file gets a score = (access_count * recency_boost) where recency_boost
/// decays with time since last access.
#[derive(Debug)]
pub(super) struct FrecencyStore {
    /// Map from relative path to (access_count, last_access_timestamp).
    accesses: HashMap<String, (u32, u64)>,
}

impl FrecencyStore {
    /// Create a new empty store.
    pub(super) fn new() -> Self {
        Self { accesses: HashMap::new() }
    }

    /// Record an access for the given path.
    pub(super) fn record(&mut self, path: &std::path::Path) {
        let key = path.to_string_lossy().into_owned();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = self.accesses.entry(key).or_insert((0, 0));
        entry.0 += 1; // Increment access count.
        entry.1 = now; // Update timestamp.
    }

    /// Get the frecency score for a path (0.0 if never accessed).
    pub(super) fn score(&self, path: &str) -> f64 {
        let Some((count, last_access)) = self.accesses.get(path) else {
            return 0.0;
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Decay: score halves every hour (3600 seconds per hour).
        const SECS_PER_HOUR: f64 = 3600.0;
        let hours = (now.saturating_sub(*last_access)) as f64 / SECS_PER_HOUR;
        let decay = 0.5_f64.powf(hours);

        *count as f64 * decay
    }
}

impl Default for FrecencyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frecency_new_file_has_zero_score() {
        let store = FrecencyStore::new();
        assert_eq!(store.score("new_file.txt"), 0.0);
    }

    #[test]
    fn frecency_single_access_gives_score_of_one() {
        let mut store = FrecencyStore::new();
        store.record(std::path::Path::new("file.txt"));
        // Single access = 1 * 1.0 (no decay) = 1.0
        assert_eq!(store.score("file.txt"), 1.0);
    }

    #[test]
    fn frecency_multiple_accesses_increase_score() {
        let mut store = FrecencyStore::new();
        store.record(std::path::Path::new("file.txt"));
        store.record(std::path::Path::new("file.txt"));
        // Two accesses = 2 * 1.0 (no decay) = 2.0
        assert_eq!(store.score("file.txt"), 2.0);
    }
}
