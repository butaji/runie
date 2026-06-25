//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod r#trait;

// Re-exports from the actor trait (lives in trait.rs).
pub use r#trait::{Actor, ActorFuture, ActorHandle, Reply, spawn_actor};

mod config;
mod fff_indexer;
mod handles;
mod io;
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
pub use session::{
    PersistenceActorHandle, SessionActor, SessionActorHandle, SessionMsg, SessionStoreActorHandle,
};
