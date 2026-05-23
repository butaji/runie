//! Comprehensive hotkey tests to prevent keyboard shortcuts from breaking.
//!
//! Tests cover:
//! - Input Box Hotkeys (Ctrl+key while in Chat mode)
//! - App-Wide Hotkeys (global, regardless of mode)

#![allow(clippy::unwrap_used)]
#![cfg(test)]

mod helpers;
mod app_wide;
mod regression;
mod dirty_flag;
mod mode_specific;

// Re-export all test modules for convenience
pub use helpers::*;
pub use app_wide::*;
pub use regression::*;
pub use dirty_flag::*;
pub use mode_specific::*;
