//! Agent event handler tests.
//!
//! Comprehensive tests for `handle_agent_event()` covering:
//! - Message flow (start, update, end)
//! - Tool execution
//! - Permission requests
//! - Error handling
//! - Token usage and cost calculation
//! - Agent lifecycle

#![allow(clippy::unwrap_used)]

pub mod message_flow;
pub mod tool_execution;
pub mod permission;
pub mod error_timeout;
pub mod token_cost;
pub mod lifecycle;