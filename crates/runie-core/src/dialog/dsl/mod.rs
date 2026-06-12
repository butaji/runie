//! Dialog DSL Module
//!
//! Provides a fluent API for building panels, forms, and stacks.
//! Re-exports all DSL types for convenient usage.

mod conversions;
mod form;
mod panel;

pub use super::{ItemAction, PanelItem, PanelStack};
pub use conversions::{FromEventExt, FromStringExt};
pub use form::{form, FormPanel};
pub use panel::{list, panel, Panel};
