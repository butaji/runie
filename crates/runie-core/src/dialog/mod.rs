//! Declarative dialog DSL with stackable panels for nested navigation.
//!
//! A `Dialog` holds a stack of `Panel`s. Only the top panel is visible.
//! Push a panel to drill down, pop to go back. Each panel contains items
//! that can be navigated with ↑/↓ and activated with Enter.
//!
//! # Unified DSL (dsl module)
//!
//! ```ignore
//! use crate::dialog::dsl::{panel, form, ItemAction};
//!
//! // Simple panel
//! let p = panel("settings", "Settings")
//!     .toggle("Dark Mode", false, "dark")
//!     .searchable();
//!
//! // Form with submit
//! let stack = form("save", "Save Session")
//!     .field("Name", "session", "name")
//!     .on_submit(Event::SaveSession)
//!     .into_stack();
//! ```
//!
//! # Flow Orchestration (flow module)
//!
//! For multi-step dialogs, use the `flow` module:
//!
//! ```ignore
//! use crate::dialog::flow::{Flow, Step, push, pop, close};
//!
//! let wizard = Flow::new("setup")
//!     .step(|_| Step::show(panel("step1", "Step 1").action("Next", push("step2"))))
//!     .step(|_| Step::show(panel("step2", "Step 2").action("Back", pop()).action("Done", close())));
//! ```

mod panel;
mod stack;
pub mod builders;
pub mod dsl;
pub mod flow;

#[cfg(test)]
mod tests;

pub use panel::{Panel, PanelItem, ItemAction};
pub use stack::{PanelStack, PanelId};
pub use builders::{
    command_palette, model_selector, settings, SettingsRow, SettingsRowKind,
    scoped_models, session_tree, theme_picker, file_picker,
};
