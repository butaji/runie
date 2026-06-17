//! Model trait resolver — maps abstract `ModelTrait` requests to concrete models.
//!
//! The Orchestrator requests models by trait (e.g. `Reasoning`, `Vision`). This
//! module resolves those requests against the configured model profiles.
//!
//! **Auto-derivation:** if a profile has no explicit traits, they are derived
//! from its `ModelCapabilities` (streaming → Fast, reasoning → Reasoning,
//! vision → Vision, context_window > 200k → LongContext, else → General).

mod error;
mod profile;
mod resolver;

pub use crate::orchestrator::ModelTrait;
pub use error::ResolverError;
pub use profile::ModelProfile;
pub use resolver::ModelResolver;

#[cfg(test)]
mod tests;
