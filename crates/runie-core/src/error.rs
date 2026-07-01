//! Shared error types for runie-core and the runie workspace.
//!
//! This module provides typed errors that replace hand-written error
//! implementations. `thiserror` is used for derive-based error types.
//!
//! ## Usage
//!
//! ```rust
//! use runie_core::error::{RunieError, RunieErrorKind};
//! use thiserror::Error;
//!
//! #[derive(Error, Debug)]
//! #[error(transparent)]
//! pub struct MyError(#[from] RunieError);
//! ```
//!
//! ## Error hierarchy
//!
//! - [`RunieError`] — main enum for common error variants
//! - [`ModelError`] — model/provider errors from `provider_event`
//! - [`ProviderError`] — provider construction/operation errors
//! - [`SanitizeError`] — message sanitization errors
//! - [`ToolParseError`] — tool-call parse errors

pub use crate::provider::ProviderError;
pub use crate::provider_event::ModelError;
pub use crate::sanitize::SanitizeError;
pub use crate::tool::types::ToolParseError;

/// Common error variants shared across the workspace.
///
/// This is the umbrella error type for high-level operations that may
/// fail due to multiple underlying causes. Use specific error types
/// in domain-specific APIs.
#[derive(Debug, thiserror::Error)]
#[error("runie error: {source}")]
pub struct RunieError {
    #[from]
    source: anyhow::Error,
}

impl RunieError {
    /// Create a new error from any error source.
    pub fn new<E: Into<anyhow::Error>>(source: E) -> Self {
        Self {
            source: source.into(),
        }
    }
}

impl From<&str> for RunieError {
    fn from(s: &str) -> Self {
        RunieError::new(anyhow::anyhow!("{s}", s = s))
    }
}

/// Kinds of errors that can occur in runie operations.
///
/// Used for categorizing errors without requiring a full error value.
/// This is useful for error handling in UI and logging contexts.
/// Derives `thiserror::Error` to produce static string displays and
/// preserve `#[source]` chains from wrapped error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RunieErrorKind {
    /// Provider configuration or API key error.
    #[error("provider")]
    Provider,
    /// Model returned an error (rate limit, context length, refusal).
    #[error("model")]
    Model,
    /// Session not found or cannot be loaded.
    #[error("session")]
    Session,
    /// Configuration file error.
    #[error("config")]
    Config,
    /// Permission denied or approval required.
    #[error("permission")]
    Permission,
    /// Message sanitization removed content.
    #[error("sanitize")]
    Sanitize,
    /// Tool call parsing failed.
    #[error("tool_parse")]
    ToolParse,
    /// General IO error.
    #[error("io")]
    Io,
    /// Unknown error.
    #[error("unknown")]
    Unknown,
}

impl RunieErrorKind {
    /// Get the human-readable name for this error kind.
    /// Kept for backward compatibility; `Display` via `thiserror` produces the same strings.
    pub fn as_str(&self) -> &'static str {
        match self {
            RunieErrorKind::Provider => "provider",
            RunieErrorKind::Model => "model",
            RunieErrorKind::Session => "session",
            RunieErrorKind::Config => "config",
            RunieErrorKind::Permission => "permission",
            RunieErrorKind::Sanitize => "sanitize",
            RunieErrorKind::ToolParse => "tool_parse",
            RunieErrorKind::Io => "io",
            RunieErrorKind::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_kind_as_str() {
        assert_eq!(RunieErrorKind::Provider.as_str(), "provider");
        assert_eq!(RunieErrorKind::Model.as_str(), "model");
        assert_eq!(RunieErrorKind::Session.as_str(), "session");
        assert_eq!(RunieErrorKind::Config.as_str(), "config");
        assert_eq!(RunieErrorKind::Permission.as_str(), "permission");
        assert_eq!(RunieErrorKind::Sanitize.as_str(), "sanitize");
        assert_eq!(RunieErrorKind::ToolParse.as_str(), "tool_parse");
        assert_eq!(RunieErrorKind::Io.as_str(), "io");
        assert_eq!(RunieErrorKind::Unknown.as_str(), "unknown");
    }

    #[test]
    fn error_kind_display_via_thiserror() {
        // thiserror::Error derive produces Display from #[error("...")] attributes
        assert_eq!(RunieErrorKind::Provider.to_string(), "provider");
        assert_eq!(RunieErrorKind::Model.to_string(), "model");
        assert_eq!(RunieErrorKind::Session.to_string(), "session");
        assert_eq!(RunieErrorKind::ToolParse.to_string(), "tool_parse");
        assert_eq!(RunieErrorKind::Io.to_string(), "io");
        assert_eq!(RunieErrorKind::Unknown.to_string(), "unknown");
    }

    #[test]
    fn runie_error_from_anyhow() {
        let err = RunieError::new(anyhow::anyhow!("test error"));
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn runie_error_from_string() {
        let err: RunieError = "test string error".into();
        assert!(err.to_string().contains("test string error"));
    }
}
