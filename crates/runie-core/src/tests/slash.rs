//! Slash command tests — ensure all /commands work as users expect

pub use crate::tests::support::{minimal_session, tmp_store};
pub use runie_testing::exec;

pub mod compact;
pub mod copy;
pub mod misc;
pub mod model;
pub mod prompts;
pub mod save_load;
pub mod session;
