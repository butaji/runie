//! Dialog opening and top-level routing.

mod file_pickers;
mod form;
mod form_handler;
mod open;
mod panel_handler;
mod router;
mod tab_complete;
pub mod toggles;

pub use form_handler::{handle_form_dialog, insert_at_ref};
pub use open::{
    open_at_file_picker, open_at_file_picker_all, open_command_palette,
    open_command_palette_with_filter, open_model_selector, open_scoped_models_dialog,
    open_session_tree_dialog, open_settings_dialog,
};
pub(crate) use panel_handler::root_closable;
pub use router::{process_command_result, update_dialog};
pub use toggles::dialog_toggle_event;

#[cfg(test)]
pub use form::{form_panel_action, FormAction};

// File picker helpers used by `open` and `panel` submodules.
pub(crate) use file_pickers::{build_file_picker_panel, rebuild_file_picker};
