//! TurnActor module — owns agent turn lifecycle and queues.

pub mod messages;
mod speed_window;
mod state;

pub use messages::{NextIdResponse, TurnMsg};
pub use speed_window::SpeedWindow;
pub use state::TurnState;

// Raptor-based implementation.
pub mod ractor_turn;
pub use ractor_turn::{RactorTurnActor, RactorTurnHandle};
