#![warn(clippy::all)]

//! Runie TUI — Terminal UI Rendering
//!
//! Contains terminal setup (`terminal_setup`, `terminal/`), input handling
//! (`keymap`), side-effect handlers (`effects/`), and all rendering widgets.

pub mod diff;
pub mod markdown_render;
pub mod message;
pub mod pace;
pub mod popups;
pub mod quantize;
pub mod semantic_tokens;
pub mod status_bar;
pub mod syntax;
pub mod theme;
pub mod ui;

// ── Terminal setup (moved from runie-term) ────────────────────────────────────

pub mod app_init;
pub mod dry_run_cmd;
pub mod effects;
pub mod keymap;
pub mod terminal;
pub mod terminal_setup;
pub mod ui_actor;

#[cfg(test)]
mod tests;

/// Re-export [`Stylize`] from ratatui for backward compatibility.
pub use ratatui::style::Stylize;
pub use runie_core::{AppState, ChatMessage};

// Re-export dialog types from runie_core for convenience
pub use runie_core::dialog::builders::{
    command_palette, file_picker, model_selector, scoped_models, session_list, session_tree,
    settings, theme_picker, SessionRow,
};
pub use runie_core::dialog::dsl::{form, get_field, panel, FormPanel};
pub use runie_core::dialog::{EventLabel, ItemAction, Panel, PanelId, PanelItem, PanelStack};
