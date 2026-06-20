//! `ProviderActor` — the single interactive builder for LLM providers.
//!
//! The actor delegates concrete provider construction to a [`ProviderFactory`]
//! implementation (lives in `runie-provider`) so that `runie-core` avoids a
//! circular dependency on the concrete provider crate.

mod actor;
mod factory;
mod messages;
#[cfg(test)]
mod tests;

pub use actor::ProviderActor;
pub use factory::{BuiltProvider, ProviderFactory};
pub use messages::{ProviderActorHandle, ProviderMsg, ProviderReply};
