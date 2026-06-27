//! Leader - Coordinates all actors in the Runie runtime.
//!
//! The Leader module provides the central coordinator that owns the event bus
//! and spawns all child actors.

mod messages;
mod actor;

pub use actor::{Leader, LeaderHandle};
pub use messages::{LeaderCommand, LeaderStatus};
