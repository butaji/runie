//! Resolution error type for trait resolution failures.

use crate::orchestrator::ModelTrait;
use std::fmt;

/// Error returned when trait resolution fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverError {
    /// No model profile matches the requested trait.
    NoMatch { trait_: ModelTrait },
    /// No models are configured at all.
    NoModelsConfigured,
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolverError::NoMatch { trait_ } => {
                write!(f, "no model configured with trait '{}'", trait_.label())
            }
            ResolverError::NoModelsConfigured => {
                write!(f, "no models configured")
            }
        }
    }
}

impl std::error::Error for ResolverError {}
