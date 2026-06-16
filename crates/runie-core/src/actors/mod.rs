//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod subagent;
mod fff_indexer;

pub use subagent::{SubagentActor, SubagentCommand, SubagentContext, SubagentEvent, SubagentStatus};
pub use fff_indexer::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResult,
    FffSearchResultPayload, FffSearchState,
};
