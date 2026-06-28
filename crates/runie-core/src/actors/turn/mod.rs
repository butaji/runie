//! TurnActor module — owns agent turn lifecycle and queues.

pub mod messages;
mod state;

pub use messages::{NextIdResponse, TurnMsg};
pub use state::{SpeedWindow, TurnState};

// Ractor-based implementation.
pub mod ractor_turn;
pub use ractor_turn::{RactorTurnActor, RactorTurnHandle};
