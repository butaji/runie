//! Runie Provider - Concrete LLM provider implementations

pub mod mock;
pub mod model;

pub use mock::MockProvider;
pub use model::{ModelId, ModelRegistry};

#[cfg(test)]
mod tests;
