//! Unified session actor module.
//!
//! This module consolidates the former `persistence`, `session_store` actors
//! and the root `session_actor.rs` into a single actor.

mod actor;
pub mod messages;

pub use actor::SessionActor;
pub use messages::{
    PersistenceActorHandle, SessionActorHandle, SessionMsg, SessionStoreActorHandle,
};
