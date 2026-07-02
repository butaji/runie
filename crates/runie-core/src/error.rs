//! Shared error types for runie-core and the runie workspace.
//!
//! This module re-exports typed errors from domain-specific sub-modules.
//!
//! ## Error hierarchy
//!
//! - [`ModelError`] — model/provider errors from `provider_event`
//! - [`ProviderError`] — provider construction/operation errors
//! - [`SanitizeError`] — message sanitization errors
//! - [`ToolParseError`] — tool-call parse errors

pub use crate::provider::ProviderError;
pub use crate::provider_event::ModelError;
pub use crate::proto::message::SanitizeError;
pub use crate::tool::types::ToolParseError;

// NOTE: `RunieError` and `RunieErrorKind` were deleted because they wrapped
// `anyhow::Error` without adding typed structure and were completely unused
// in the codebase. Typed errors (ModelError, ProviderError, etc.) are used
// instead. See tasks/restructure-runieerror-with-typed-variants.md.
