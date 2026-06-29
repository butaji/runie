//! Vim-navigation selection rendering tests.

// Re-export types used by child test modules so they can `use super::*;`
pub use super::{
    AppState, ChatMessage, Part, Role,
};

mod background;
mod basic;
mod bracket;
mod helpers;
mod wrap_mapping;
