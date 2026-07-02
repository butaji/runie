//! TurnActor module — owns agent turn lifecycle and queues.

pub mod messages;
mod speed_window;
mod state;
pub mod types;

// Handler functions (used by actor.rs).
mod handlers;

// Ractor-based implementation.
pub mod actor;

pub use messages::{DeliverQueuedResponse, DeliverQueuedRpcResult, NextIdResponse, TurnMsg};
pub use speed_window::SpeedWindow;
pub use state::TurnState;
pub use types::RactorTurnHandle;

// Re-export RactorTurnActor from actor module for backward compatibility.
pub use actor::RactorTurnActor;
