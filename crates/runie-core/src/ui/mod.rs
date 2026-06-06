//! UI Module - DSL and Elements for declarative UI construction
//! 
//! Architecture:
//! - elements.rs: Pure data structures representing UI components
//! - dsl.rs: Operations to build UI from state
//! - format.rs: Rendering elements to display lines

pub mod elements;
pub mod dsl;
pub mod format;

pub use elements::{Element, Feed};
pub use dsl::Dsl;
pub use format::{format_messages, format_and_cache, render_feed, DisplayLine, DisplaySpan, Color};
