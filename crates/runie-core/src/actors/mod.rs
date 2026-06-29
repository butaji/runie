//! Actor definitions for the Runie runtime.
//!
//! The agent uses ractor-based actors: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

pub mod ractor_adapter;
pub use ractor_adapter::{spawn_ractor, RactorHandle, Reply, RpcReply};

pub mod turn;
pub mod config;
pub mod fff_indexer;
pub mod handles;
#[cfg(test)]
mod handles_tests;
pub mod input;
pub mod io;
pub mod leader;
pub mod permission;
pub mod provider;
pub mod session;

mod persistence;
pub use persistence::PersistenceActor;

pub use handles::ActorHandles; // re-export alias (points to LeaderHandle)

pub use turn::{TurnMsg, RactorTurnActor, RactorTurnHandle};
pub use config::{RactorConfigActor, RactorConfigHandle, ConfigMsg};
pub use fff_indexer::{
    FffFileItem, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState, format_git_status, RactorFffIndexerHandle, RactorFffIndexerActor,
};
pub use input::{InputActor, RactorInputHandle, InputMsg};
pub use io::{IoMsg, RactorIoActor, RactorIoHandle};
pub use provider::{
    BuiltProvider, ProviderActorHandle, ProviderFactory, ProviderMsg,
    RactorProviderActor, RactorProviderHandle,
};
pub use permission::{PermissionMsg, RactorPermissionActor, RactorPermissionHandle};
pub use session::{
    RactorSessionActor, RactorSessionHandle, SessionMsg,
};
pub use leader::{Leader, LeaderHandle, LeaderCommand, LeaderStatus};
