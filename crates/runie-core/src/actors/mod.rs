//! Actor definitions for the Runie runtime.
//!
//! The agent uses a lightweight actor model: each actor is a tokio task
//! receiving typed messages. This module contains the actors that live inside
//! a running session.

mod fff_indexer;
mod subagent;

pub use fff_indexer::{
    FffFileItem, FffIndexerActor, FffSearchRequest, FffSearchResult, FffSearchResultPayload,
    FffSearchState,
};
pub use subagent::{
    SubagentActor, SubagentCommand, SubagentContext, SubagentEvent, SubagentStatus,
};
