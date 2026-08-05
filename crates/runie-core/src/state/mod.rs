//! Agent state actor: the single source of truth for all `AgentState`
//! fields. Other actors and the loop read `AgentStateSnapshot` and write
//! via the command mailbox.

pub mod actor;
pub mod snapshot;

pub use actor::{AgentStateActor, StateCommand};
pub use snapshot::AgentStateSnapshot;
