//! Domain-specific languages for tests.

#[cfg(test)]
mod test_dsl;
#[cfg(test)]
pub use test_dsl::{AgentTurn, AppStateDsl};
