//! Unified session actor module.
//!
//! This module consolidates the former `persistence`, `session_store` actors
//! and the root `session_actor.rs` into a single actor.

mod actor;
pub mod messages;
mod mutations;
#[cfg(test)]
mod tests;

// Ractor-based implementation
mod ractor_session_actor;
mod ractor_session_handle;

// Ractor-based SessionActor (recommended).
pub use ractor_session_actor::RactorSessionActor;
pub use ractor_session_handle::RactorSessionHandle;

// Legacy SessionActor using custom trait (deprecated).
#[deprecated(since = "0.3.0", note = "Use RactorSessionActor instead")]
pub use actor::SessionActor;
pub use messages::{
    PersistenceActorHandle, SessionActorHandle, SessionMsg, SessionStoreActorHandle,
};
