//! Panel types — struct definition and core types.
//!
//! Split from dialog/panel.rs to stay under the 500-line limit.

use super::{ItemAction, PanelItem};
use crate::Event;

// Re-export helpers
mod helpers;

// Builder methods
mod builders;

// Navigation and filter methods
mod navigation;

// Form methods
mod form_methods;

/// Function that builds the submit event from collected form values.
pub type FormSubmitFn = fn(&std::collections::HashMap<String, String>) -> Event;

/// Visual layout of a panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelView {
    /// Scrollable list with fuzzy search.
    #[default]
    List,
    /// Form with labeled fields and bottom button bar.
    Form,
}

/// A single panel inside a dialog — title + list of items + selection state.
// allow: fn pointer comparison is unavoidable for the FormSubmitFn type in PanelEq
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub items: Vec<PanelItem>,
    pub selected: usize,
    /// Optional filter text when the panel is filterable.
    pub filter: String,
    pub filterable: bool,
    /// When true, activating an item (Enter) does NOT close the dialog.
    /// Useful for previews (e.g. theme picker) and live toggles.
    pub keep_open_on_activate: bool,
    /// When false, the dialog cannot be dismissed from the root panel
    /// (Esc/DialogBack, Abort, Quit are ignored). ForceQuit still works.
    pub closable: bool,
    /// For form panels: stores form values (key -> value)
    pub form_values: std::collections::HashMap<String, String>,
    /// For form panels: factory that turns form values into the submit event.
    pub submit_factory: Option<FormSubmitFn>,
    /// For form panels: the canonical command name, used to route submissions
    /// through the command registry instead of emitting raw events.
    pub cmd_name: Option<String>,
    /// For form panels: ordered list of field keys, used to serialize form
    /// values as positional arguments for the command registry.
    pub field_keys: Vec<String>,
    /// Visual layout of this panel.
    pub view: PanelView,
}
