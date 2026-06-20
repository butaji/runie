//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod config;
mod fff_indexer;
mod persistence;
pub mod provider;

pub use config::{ConfigActor, ConfigActorHandle, ConfigMsg, ConfigReply};
pub use fff_indexer::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState,
};
pub use persistence::{PersistenceActor, PersistenceActorHandle, PersistenceMsg};
pub use provider::{
    BuiltProvider, ProviderActor, ProviderActorHandle, ProviderFactory, ProviderMsg,
};
