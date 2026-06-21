//! UI Module — Three-layer architecture (core = elements + transform only)
//!
//! Layers:
//!   elements  :: Pure data structures (Element, Feed)
//!   transform :: State → Elements (pure, lazy, cached)
//!
//! Rendering (ratatui-dependent) lives in runie-tui crate.
//! Note: the UI AST remains in runie-core because AppState caches the feed
//! and view state; moving it to runie-tui would require moving AppState or
//! introducing a renderer trait injection, which is a larger refactor.

pub mod dsl_test;
pub mod elements;
pub mod posts;
pub mod transform;

pub use elements::{Element, Feed, Post, PostKind};
pub use posts::PostBuilder;
pub use transform::LazyCache;
