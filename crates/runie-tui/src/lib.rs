#![warn(clippy::all)]

//! Runie TUI - Terminal UI Rendering
pub mod diff;
pub mod markdown;
pub mod message;
pub mod popups;
pub mod quantize;
pub mod status_bar;
pub mod syntax;
pub mod theme;
pub mod ui;

#[cfg(test)]
mod tests;

pub use runie_core::{AppState, ChatMessage};
