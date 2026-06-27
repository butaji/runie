//! UI Module — Three-layer architecture (core = elements + transform only)
//!
//! Layers:
//!   elements  :: Pure data structures (Element, Feed)
//!   transform :: State → Elements (pure)
//!
//! Rendering (ratatui-dependent) lives in runie-tui crate.
//! Note: the UI AST remains in runie-core. AppState no longer caches the feed;
//! UiActor owns the Element cache for rendering. The projection is built
//! on-demand when building Snapshot.

pub mod dsl_test;
pub mod elements;
pub mod posts;
pub mod transform;

pub use elements::{Element, Feed, Post, PostKind};
pub use posts::PostBuilder;
pub use transform::LazyCache;
