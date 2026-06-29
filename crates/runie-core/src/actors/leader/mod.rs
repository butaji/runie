//! Leader - Coordinates all actors in the Runie runtime.
//!
//! ## Bootstrap
//!
//! ```ignore
//! let leader = Leader::new();
//! let (render_tx, handle) = leader.start(factory, agent_factory).await?;
//! // Move render_tx into the UiActor, use handle for actor communication.
//! ```
//!
//! The Leader module provides the central coordinator that owns the event bus
//! and spawns all child actors.

use std::pin::Pin;

mod messages;
mod actor;

pub use actor::{Leader, LeaderHandle, LeaderConfig};
pub use messages::{LeaderCommand, LeaderStatus};

use crate::actors::permission::RactorPermissionHandle;
use crate::actors::provider::RactorProviderHandle;
use crate::bus::EventBus;
use crate::event::Event;
use crate::model::ThinkingLevel;

/// Command passed from the leader to the agent actor.
///
/// This mirrors the fields of `runie_agent::AgentCommand` so that runie-core
/// can send commands to the agent without depending on runie-agent.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LeaderAgentCmd {
    pub content: String,
    pub id: String,
    pub provider: String,
    pub model: String,
    pub thinking_level: ThinkingLevel,
    pub read_only: bool,
    pub skills_context: String,
    pub system_prompt: String,
}

/// Handle type for the agent actor, defined as a trait to allow runie-agent
/// to implement the factory without creating a dependency cycle.
///
/// The concrete type is `runie_agent::RactorAgentHandle`; this trait lets
/// `runie-core` hold a reference without importing the agent crate.
pub trait LeaderAgentHandle: Send + Sync {
    /// Send a run command to the agent (fire-and-forget).
    fn run(&self, cmd: LeaderAgentCmd) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

/// Factory for spawning `AgentActor` instances.
///
/// Implement this trait in `runie-agent` to avoid a `runie-core` → `runie-agent`
/// dependency cycle.
pub trait AgentActorFactory: Send + Sync {
    /// Spawn an agent actor connected to the given event bus and handles.
    fn spawn(
        &self,
        bus: EventBus<Event>,
        provider_handle: RactorProviderHandle,
        permission_handle: RactorPermissionHandle,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Box<dyn LeaderAgentHandle>, ractor::SpawnErr>> + Send>,
    >;
}
