//! Actor definitions for the Runie runtime.
//!
//! The agent uses ractor-based actors: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

pub mod constants;
pub mod ractor_adapter;
pub use constants::{
    CONFIG_WATCHER_DEBOUNCE_MS, LEADER_CMD_CHANNEL_CAPACITY, SHUTDOWN_TIMEOUT_SECS,
};
pub use ractor_adapter::spawn_ractor;

pub mod config;
pub mod fff_indexer;
pub mod input;
pub mod io;
pub mod leader;
pub mod permission;
pub mod provider;
pub mod session;
pub mod turn;

mod persistence;
pub use persistence::PersistenceActor;

pub use config::{ConfigMsg, RactorConfigActor, RactorConfigHandle};
pub use fff_indexer::{
    FffFileItem, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState, RactorFffIndexerActor, RactorFffIndexerHandle,
};
#[cfg(feature = "git")]
pub use fff_indexer::format_git_status;
pub use input::{InputActor, InputMsg, RactorInputHandle};
pub use io::{IoMsg, RactorIoActor, RactorIoHandle};
pub use leader::{Leader, LeaderCommand, LeaderHandle, LeaderStatus};
pub use permission::{PermissionMsg, RactorPermissionActor, RactorPermissionHandle};
pub use provider::{
    BuiltProvider, ProviderFactory, ProviderMsg, RactorProviderActor, RactorProviderHandle,
};
pub use session::{RactorSessionActor, RactorSessionHandle, SessionMsg};
pub use turn::{RactorTurnActor, RactorTurnHandle, TurnMsg};
