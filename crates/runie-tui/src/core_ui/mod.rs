//! Re-export of the UI element AST that lives in runie-core.
//!
//! The actual types remain in `runie-core` because `AppState` caches the feed
//! and view state. Moving them to runie-tui would require moving `AppState` or
//! introducing a renderer trait, which is blocked by orphan rules and the
//! current crate dependency graph.

pub use runie_core::ui::{
    Element, Feed, LazyCache, Post, PostKind, PostBuilder,
};
