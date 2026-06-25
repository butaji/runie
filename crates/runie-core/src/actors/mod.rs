//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod r#trait;

// Re-exports from the actor trait (lives in trait.rs).
pub use r#trait::{spawn_actor, Actor, ActorFuture, ActorHandle, Reply};

mod config;
mod fff_indexer;
mod handles;
mod io;
pub mod permission;
pub mod provider;
pub mod session;

pub use handles::{ActorHandles, FffIndexerHandle};

pub use config::{ConfigActor, ConfigActorHandle, ConfigMsg};
pub use fff_indexer::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState,
};
pub use io::{IoActor, IoActorHandle, IoMsg};
pub use provider::{
    BuiltProvider, ProviderActor, ProviderActorHandle, ProviderFactory, ProviderMsg,
};
pub use permission::{PermissionActor, PermissionActorHandle, PermissionMsg};
pub use session::{
    PersistenceActorHandle, SessionActor, SessionActorHandle, SessionMsg, SessionStoreActorHandle,
};
