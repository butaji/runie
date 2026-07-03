//! Common persistence trait for actors that own durable state.
//!
//! `ConfigActor` and `SessionActor` both follow the same pattern:
//! - Load authoritative state from disk on startup.
//! - Mutate state in response to messages.
//! - Persist changes back to disk.
//! - Publish fact events on the shared `EventBus`.
//!
//! This trait documents the shared contract so new persistence actors
//! can adopt the same pattern without duplicating the convention.

use crate::bus::EventBus;
use crate::event::Event;

/// Actor that owns durable state and manages load/persist lifecycle.
///
/// Implementors: `ConfigActor`, `SessionActor`.
pub trait PersistenceActor {
    /// Load all authoritative state from disk and publish relevant facts.
    ///
    /// Called once at actor startup before the message loop begins.
    fn load_all(&mut self, bus: &EventBus<Event>) -> impl std::future::Future<Output = ()> + Send;
}

// Note: The PersistenceActor trait documents the pattern used by config and session actors.
// Ractor-based actors follow the same pattern but the trait is not currently implemented
// on the actor structs directly. The handle types provide the persistence operations.
