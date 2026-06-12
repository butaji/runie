//! Dialog DSL Module
//!
//! Provides a fluent API for building panels, forms, and stacks.
//! Re-exports all DSL types for convenient usage.

mod panel;
mod form;
mod conversions;

pub use panel::{Panel, panel};
pub use form::{FormPanel, form};
pub use super::{ItemAction, PanelItem, PanelStack};
pub use conversions::{FromStringExt, FromEventExt};
