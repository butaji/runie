//! Agent profile types re-exported from `runie-core`.
//!
//! The canonical implementation lives in `runie_core::agent_profiles` so that
//! both `runie-core` and `runie-agent` share the same `AgentProfile` type
//! without a dependency cycle.

pub use runie_core::agent_profiles::*;
