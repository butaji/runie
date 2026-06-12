//! UI Module — Three-layer architecture (core = elements + transform only)
//!
//! Layers:
//!   elements  :: Pure data structures (Element, Feed)
//!   transform :: State → Elements (pure, lazy, cached)
//!
//! Rendering (ratatui-dependent) lives in runie-tui crate

pub mod dsl_test;
pub mod elements;
pub mod transform;

pub use elements::{Element, Feed};
pub use transform::LazyCache;

#[cfg(test)]
pub mod format_test {
    pub use super::transform::format_test::*;
}
