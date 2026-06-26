//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod r#trait;

pub mod turn;

// Re-exports from the actor trait (lives in trait.rs).
pub use r#trait::{spawn_actor, Actor, ActorFuture, ActorHandle, GenericActorHandle, Reply};

mod persistence;
pub use persistence::PersistenceActor;

mod completion;
mod config;
mod fff_indexer;
mod handles;
mod input;
mod io;
pub mod permission;
pub mod provider;
pub mod session;
pub mod trust;
pub mod view;

pub use handles::{ActorHandles, FffIndexerHandle};

pub use completion::{CompletionActor, CompletionActorHandle, CompletionMsg};
pub use turn::{TurnActor, TurnActorHandle, TurnMsg};
pub use config::{ConfigActor, ConfigActorHandle, ConfigMsg};
pub use fff_indexer::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState, format_git_status,
};
pub use input::{InputActor, InputActorHandle, InputMsg};
pub use io::{IoActor, IoActorHandle, IoMsg};
pub use provider::{
    BuiltProvider, ProviderActor, ProviderActorHandle, ProviderFactory, ProviderMsg,
};
pub use permission::{PermissionActor, PermissionActorHandle, PermissionMsg};
pub use session::{
    PersistenceActorHandle, SessionActor, SessionActorHandle, SessionMsg, SessionStoreActorHandle,
};
pub use view::{ViewActor, ViewActorHandle, ViewMsg};
pub use trust::{TrustActor, TrustActorHandle, TrustMsg};
