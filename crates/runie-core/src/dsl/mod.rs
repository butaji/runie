//! Test DSL for driving AppState in tests.
//!
//! This module provides a fluent test builder for agent turn sequences.
//! The actor composition DSL (flow, runtime) has been deleted as it had
//! broken combinators that ignored their closures.

#[cfg(test)]
mod test_dsl;

#[cfg(test)]
pub use test_dsl::{AgentTurn, AppStateDsl};
