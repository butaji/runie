//! High-level panel builders for common dialog patterns.
//!
//! These builders replace custom `DialogState` variants with a unified
//! `Panel` + `PanelStack` API. Each builder returns a `PanelStack` ready
//! to be assigned to `AppState::open_dialog`.

mod model;
mod palette;
mod picker;
mod provider_models;
mod session;
mod settings;

#[cfg(test)]
mod tests;

pub use crate::dialog::{ItemAction, Panel, PanelItem, PanelStack};

pub use model::scoped_models;
pub use palette::{command_palette, model_selector};
pub use picker::{file_picker, theme_picker};
pub use provider_models::{is_provider_models_stack, provider_models};
pub use session::{session_list, session_tree, SessionRow};
pub use settings::{settings, SettingsRow};
