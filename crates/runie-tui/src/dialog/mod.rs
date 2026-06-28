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
//! * **List view** (`panel`) — a scrollable list of actions, toggles,
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
//! Every panel has a [`PanelView`] — `List` (default) or `Form`. The view
//! decides how the panel is rendered and whether keystrokes filter items or
//! edit fields.
//!
//! ```
//! use runie_core::dialog::dsl::{form, panel, ItemAction};
//! use runie_core::Event;
//!
//! // List view (fuzzy-searchable by default)
//! let _ = panel("settings", "Settings")
//!     .toggle("Dark Mode", false, ItemAction::Toggle("dark".into()))
//!     .select("Theme", "runie", vec!["runie".into(), "dracula".into()], "theme");
//!
//! // Form view (filtering disabled, fields editable)
//! let _ = form("save", "Save Session")
//!     .field("Name", "session", "name")
//!     .on_submit(|values| Event::RunSaveCommand {
//!         name: values.get("name").cloned().unwrap_or_default(),
//!     })
//!     .into_stack();
//!
//! // Any panel can be switched to form view explicitly, e.g. a loading
//! // panel with a single Cancel button rendered as a form button.
//! let _ = panel("validating", "Validating...")
//!     .form()
//!     .header("Checking API key...")
//!     .action("_Cancel", ItemAction::Emit(Event::Cancel));
//! ```
//!
pub mod builders;
pub mod dsl;
mod item;
mod panel;
pub mod panel_split;
pub(crate) mod score;
mod stack;

#[cfg(test)]
mod tests;

pub use runie_core::settings::SettingValue;
pub use builders::{
    command_palette, file_picker, model_selector, scoped_models, session_list, session_tree,
    settings, theme_picker,
};
pub use item::{parse_accel, strip_accel, EventLabel, ItemAction, PanelItem};
pub use panel::{FormSubmitFn, Panel, PanelView};
pub use stack::{PanelId, PanelStack};
