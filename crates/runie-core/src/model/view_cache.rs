//! Shared view cache type used by AppState and cache/mod.rs.
//!
//! This is a separate module to avoid circular dependencies:
//! - cache/mod.rs imports AppState (to access view.message_gen)
//! - AppState needs to store ViewCache (to cache feed data)
//!
//! By keeping ViewCache in its own module, neither import creates a cycle.

use std::sync::Arc;

use crate::view::elements::{Element, Post};

/// Intermediate view cache built from LazyCache.
/// Stored in AppState and only rebuilt when message_gen changes.
#[derive(Clone)]
pub(crate) struct ViewCache {
    pub elements: Arc<[Element]>,
    pub posts: Arc<[Post]>,
    pub line_counts: Arc<[usize]>,
    pub total_lines: usize,
    pub cached_gen: u64,
}
