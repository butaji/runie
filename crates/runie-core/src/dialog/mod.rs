//! Declarative dialog DSL with stackable panels for nested navigation.
//!
//! A `Dialog` holds a stack of `Panel`s. Only the top panel is visible.
//! Push a panel to drill down, pop to go back. Each panel contains items
//! that can be navigated with ↑/↓ and activated with Enter.
//!
//! # Dialog view types
//!
//! There are two high-level panel layouts:
//!
//! * **List view** (`panel` / `list`) — a scrollable list of actions, toggles,
//!   and selects. List panels have fuzzy search enabled by default: typing while
//!   the dialog is open filters items.
//! * **Form view** (`form`) — labeled input fields with a submit button. Forms
//!   capture keystrokes for editing, so filtering is disabled automatically.
//!
//! Both builders produce a `Panel` (or `PanelStack`) and share the same nested
//! navigation model.
//!
//! # Unified DSL (dsl module)
//!
//! ```ignore
//! use crate::dialog::dsl::{panel, form, ItemAction};
//!
//! // List view (fuzzy-searchable by default)
//! let p = panel("settings", "Settings")
//!     .toggle("Dark Mode", false, "dark")
//!     .select("Theme", "runie", vec!["runie".into(), "dracula".into()], "theme");
//!
//! // Equivalent list-view alias
//! let p = list("settings", "Settings").action("Done", ItemAction::Close);
//!
//! // Form view (filtering disabled, fields editable)
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

pub mod builders;
pub mod dsl;
pub mod flow;
mod panel;
mod stack;

#[cfg(test)]
mod tests;

pub use builders::{
    command_palette, file_picker, model_selector, scoped_models, session_tree, settings,
    theme_picker, SettingsRow, SettingsRowKind,
};
pub use panel::{parse_accel, strip_accel, ItemAction, Panel, PanelItem};
pub use stack::{PanelId, PanelStack};
