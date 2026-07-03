//! View cache type for the Element/Post projection.
//!
//! This module contains the `ViewCache` struct which holds the projection
//! results (elements, posts, line counts). Used by `cache/mod.rs` when
//! building the view projection.

use std::sync::Arc;

use crate::view::elements::{Element, Post};

/// Intermediate view cache built from LazyCache.
/// Stored in AppState and only rebuilt when message_gen changes.
#[derive(Clone, Debug)]
#[allow(
    dead_code,
    reason = "cached_gen written by view_cache(); read by UiActor after decouple"
)]
pub(crate) struct ViewCache {
    pub cached_gen: u64,
    pub elements: Arc<[Element]>,
    pub posts: Arc<[Post]>,
    pub line_counts: Arc<[usize]>,
    pub total_lines: usize,
}
