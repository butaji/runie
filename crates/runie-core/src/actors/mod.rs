//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod config;
mod fff_indexer;
mod io;
pub mod provider;
pub mod session;

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
