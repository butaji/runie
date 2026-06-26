#![warn(clippy::all)]

//! Runie TUI — Terminal UI Rendering
//!
//! Contains terminal setup (`terminal_setup`, `terminal/`), input handling
//! (`keymap`), side-effect handlers (`effects/`), and all rendering widgets.

pub mod diff;
pub mod markdown;
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
