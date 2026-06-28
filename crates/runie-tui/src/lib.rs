#![warn(clippy::all)]

//! Runie TUI — Terminal UI Rendering
//!
//! Contains terminal setup (`terminal_setup`, `terminal/`), input handling
//! (`keymap`), side-effect handlers (`effects/`), and all rendering widgets.

pub mod diff;
pub mod dialog;
pub mod markdown_render;
pub mod message;
pub mod pace;
pub mod popups;
pub mod quantize;
pub mod semantic_tokens;
pub mod status_bar;
pub mod stylize;
pub mod syntax;
pub mod theme;
pub mod ui;

// ── Terminal setup (moved from runie-term) ────────────────────────────────────

pub mod app_init;
pub mod dry_run;
pub mod effects;
pub mod keymap;
pub mod terminal;
pub mod terminal_setup;
pub mod ui_actor;

#[cfg(test)]
mod tests;

pub use runie_core::{AppState, ChatMessage};
pub use stylize::Stylize;

// Re-export dialog types for convenience
pub use dialog::{EventLabel, ItemAction, Panel, PanelItem, PanelStack, PanelId};
pub use dialog::dsl::{form, panel, get_field, FormPanel};
pub use dialog::builders::{
    command_palette, file_picker, model_selector, scoped_models, session_list, session_tree,
    settings, theme_picker, SessionRow,
};
