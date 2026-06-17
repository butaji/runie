use runie_core::trait_resolver::ModelTrait;
use std::fmt;

/// Errors that can occur during planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerError {
    /// LLM call timed out.
    Timeout,
    /// Failed to parse LLM output as JSON after all retries.
    ParseFailed { attempts: usize, last_error: String },
    /// Plan validation failed.
    ValidationFailed(String),
    /// No model matches the required trait for a task.
    NoModelForTrait { trait_: ModelTrait },
    /// Provider returned an error.
    ProviderError(String),
}

impl fmt::Display for PlannerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlannerError::Timeout => write!(f, "planner LLM call timed out"),
            PlannerError::ParseFailed {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "failed to parse plan JSON after {} attempts: {}",
                    attempts, last_error
                )
            }
            PlannerError::ValidationFailed(msg) => write!(f, "plan validation failed: {}", msg),
            PlannerError::NoModelForTrait { trait_ } => {
                write!(f, "no model configured for trait '{}'", trait_)
            }
            PlannerError::ProviderError(msg) => write!(f, "provider error: {}", msg),
        }
    }
}

impl std::error::Error for PlannerError {}
