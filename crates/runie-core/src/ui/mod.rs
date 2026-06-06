//! UI Module — Three-layer architecture (core = elements + transform only)
//!
//! Layers:
//!   elements  :: Pure data structures (Element, Feed)
//!   transform :: State → Elements (pure, lazy, cached)
//!   format    :: Legacy formatting (DisplayLine, DisplaySpan)
//!
//! Rendering (ratatui-dependent) lives in runie-tui crate

pub mod elements;
pub mod transform;
pub mod format;
pub mod dsl_test;

pub use elements::{Element, Feed};
pub use transform::{LazyCache, StreamingMerge};
pub use format::{format_messages, DisplayLine, DisplaySpan};
