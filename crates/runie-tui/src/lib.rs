#![warn(clippy::all)]

//! Runie TUI - Terminal UI Rendering
pub mod diff;
pub mod markdown;
pub mod popups;
pub mod syntax;
pub mod theme;
pub mod ui;

#[cfg(test)]
mod tests;

pub use runie_core::{AppState, ChatMessage};
