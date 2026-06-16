#![warn(clippy::all)]

//! Runie TUI — Terminal UI Rendering
//!
//! Contains terminal setup (`terminal_setup`, `terminal/`), input handling
//! (`keymap`), side-effect handlers (`effects/`), and all rendering widgets.

pub mod diff;
pub mod markdown;
pub mod message;
pub mod popups;
pub mod quantize;
pub mod semantic_tokens;
pub mod status_bar;
pub mod syntax;
pub mod theme;
pub mod ui;

// ── Terminal setup (moved from runie-term) ────────────────────────────────────

pub mod app_init;
pub mod effects;
pub mod keymap;
pub mod share;
pub mod terminal;
pub mod terminal_setup;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod theme_tests;

pub use runie_core::{AppState, ChatMessage};
