//! TurnActor module — owns agent turn lifecycle and queues.

mod actor;
pub mod messages;
mod state;

pub use actor::{TurnActor, TurnActorHandle};
pub use messages::{NextIdResponse, TurnMsg};
pub use state::{SpeedWindow, TurnState};
