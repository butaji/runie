//! Ractor-based TurnActor implementation.
//!
//! This module re-exports the actor implementation.
//! Implementation details are split into focused modules:
//! - `handlers.rs` — all message handler functions
//! - `actor.rs` — the Actor trait impl and spawn function
//! - `state.rs` — TurnState struct
//! - `types.rs` — TurnActorState and RactorTurnHandle
//! - `tests.rs` — unit and contract tests

// Re-export the actor for backward compatibility.
pub use crate::actors::turn::actor::RactorTurnActor;
