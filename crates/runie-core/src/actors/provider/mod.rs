//! `ProviderActor` — the single interactive builder for LLM providers.

mod factory;
mod messages;
#[cfg(test)]
mod tests;

// Ractor-based ProviderActor.
pub mod ractor_provider;
pub use factory::{BuiltProvider, ProviderFactory};
pub use messages::ProviderMsg;
pub use ractor_provider::{RactorProviderActor, RactorProviderHandle};
