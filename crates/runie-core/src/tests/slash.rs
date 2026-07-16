//! Slash command tests — ensure all /commands work as users expect

pub use crate::tests::support::{exec, minimal_session, tmp_store};

pub mod compact;
pub mod copy;
pub mod goal;
pub mod misc;
pub mod model;
pub mod prompts;
pub mod save_load;
pub mod session;
