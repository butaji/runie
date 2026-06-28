//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod r#trait;

pub mod plan;
pub mod turn;

// Ractor adapter for incremental migration.
pub mod ractor_adapter;

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
pub mod leader;
pub mod permission;
pub mod provider;
pub mod session;
pub mod trust;
pub mod view;

pub use handles::{ActorHandles, FffIndexerHandle};

pub use completion::{CompletionActor, CompletionActorHandle, CompletionMsg};
pub use turn::{TurnMsg, RactorTurnActor, RactorTurnHandle};
// Legacy exports for backward compatibility during migration.
#[allow(deprecated)]
pub use turn::{TurnActor, TurnActorHandle};
pub use plan::{PlanActor, PlanActorHandle, PlanMsg, PlanState, PlanStepStatus, RactorPlanActor, RactorPlanHandle};
pub use config::{RactorConfigActor, RactorConfigHandle, ConfigActor, ConfigActorHandle, ConfigMsg};
pub use fff_indexer::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState, format_git_status,
};
pub use input::{InputActor, RactorInputHandle, InputMsg};
#[allow(deprecated)]
pub use input::InputActorHandle;
pub use io::{IoActor, IoActorHandle, IoMsg, RactorIoActor, RactorIoHandle};
pub use provider::{
    BuiltProvider, ProviderActor, ProviderActorHandle, ProviderFactory, ProviderMsg,
};
pub use permission::{PermissionActor, PermissionActorHandle, PermissionMsg, RactorPermissionActor, RactorPermissionHandle};
pub use session::{
    PersistenceActorHandle, RactorSessionActor, RactorSessionHandle, SessionActor, SessionActorHandle, SessionMsg, SessionStoreActorHandle,
};
pub use view::{ViewActor, ViewActorHandle, ViewMsg, RactorViewActor, RactorViewHandle};
pub use trust::{TrustActor, TrustActorHandle, TrustMsg};
pub use leader::{Leader, LeaderHandle, LeaderCommand, LeaderStatus};
