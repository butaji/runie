//! Dialog opening and top-level routing.

mod fff;
mod file_picker;
mod form;
mod form_handler;
mod model_selector;
mod open;
mod panel;
mod router;
mod tab_complete;
pub mod toggle;

pub use form_handler::{handle_form_dialog, insert_at_ref};
pub use open::{
    open_at_file_picker, open_at_file_picker_all, open_command_palette, open_model_selector,
    open_provider_models_dialog, open_scoped_models_dialog, open_session_tree_dialog,
    open_settings_dialog,
};
pub(crate) use panel::root_closable;
pub use router::{process_command_result, update_dialog};
pub use toggle::dialog_toggle_event;

#[cfg(test)]
pub use form::{form_panel_action, FormAction};

// File picker helpers used by `open` and `panel` submodules.
pub(crate) use file_picker::{build_file_picker_panel, rebuild_file_picker};
