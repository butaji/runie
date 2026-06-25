//! Dialog DSL Module
//!
//! Provides a fluent API for building panels, forms, and stacks.
//! All constructors return `crate::dialog::Panel` values.

mod form;
mod panel;

pub use super::{ItemAction, PanelItem, PanelStack};
pub use form::{form, get_field, FormPanel};
pub use panel::panel;
