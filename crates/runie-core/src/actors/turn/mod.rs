//! TurnActor module — owns agent turn lifecycle and queues.

#[cfg(test)]
mod tests;

pub mod messages;
mod speed_window;
pub mod state;
pub mod types;

// Handler functions (used by actor.rs).
mod handlers;

// Ractor-based implementation.
pub mod actor;

pub use messages::{DeliverQueuedResponse, DeliverQueuedRpcResult, NextIdResponse, TurnMsg};
pub use speed_window::SpeedWindow;
pub use state::TurnState;
pub use types::RactorTurnHandle;

// Backward-compat stubs
#[allow(unused_imports)]
pub use actor::RactorTurnActor;
#[allow(unused_imports)]
pub use types::RactorTurnHandle as TurnHandle;
