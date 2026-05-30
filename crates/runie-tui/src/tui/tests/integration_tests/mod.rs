//! Integration tests for full agent conversation flows.
//!
//! Tests verify complete conversation scenarios:
//! - Multi-turn conversations with proper turn separators
//! - Tool execution and results display
//! - Error handling and recovery
//! - Permission request flow
//!
//! Note: Some events (PermissionGranted, PermissionDenied, ContextCompacted) are
//! handled by the agent loop externally and are classified as Ignored by the TUI.

pub mod helpers;
pub mod conversation_flow_tests;
pub mod tool_permission_tests;
pub mod error_lifecycle_tests;
pub mod token_usage_tests;

pub use helpers::{agent_message, default_token_usage};
