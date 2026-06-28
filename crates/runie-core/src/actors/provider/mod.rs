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

// Ractor-based ProviderActor (recommended for production).
pub mod ractor_provider;
pub use ractor_provider::{RactorProviderActor, RactorProviderHandle};

// Legacy custom-trait actor (deprecated, kept for test compatibility).
#[allow(deprecated)]
pub use actor::ProviderActor;
pub use factory::{BuiltProvider, ProviderFactory};
pub use messages::{ProviderActorHandle, ProviderMsg};
