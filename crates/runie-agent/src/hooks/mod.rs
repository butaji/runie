//! Hooks module for runie-agent.
//!
//! This module provides hooks for intercepting and modifying agent behavior at
//! key points during execution. Hooks allow the TUI to integrate with the
//! agent's decision-making process without coupling the core agent logic to
//! UI concerns.
//!
//! ## Hook System Architecture
//!
//! The hook system is built on rig-core's `PromptHook` trait, which allows
//! custom behavior to be injected at specific points in the agent's execution
//! flow. Each hook receives context about the current operation and returns
//! a decision that controls how execution proceeds.
//!
//! ### PermissionPromptHook
//!
//! The primary hook in this module is [`PermissionPromptHook`], which intercepts
//! tool calls before they execute. When a tool is invoked:
//!
//! 1. The hook checks if the tool has been previously allowed (cached)
//! 2. If not cached, it sends a [`PermissionEvent::Request`] to the TUI
//! 3. The hook waits for a [`PermissionDecision`] from the UI
//! 4. Based on the decision, execution continues, skips, or is denied
//!
//! The hook communicates with the UI via channels:
//! - Event channel: Hook → TUI (permission requests and results)
//! - Decision channel: TUI → Hook (user decisions)
//!
//! ## Usage
//!
//! ```ignore
//! let (hook, event_rx, decision_tx) = PermissionPromptHook::new(timeout_secs);
//! // Spawn a task to forward events_rx to your UI
//! // Spawn a task to feed decisions from your UI to decision_tx
//! // Register hook with your agent
//! ```
//!
//! See [`PermissionPromptHook`] for detailed API documentation.

pub mod permission_hook;

pub use permission_hook::{PermissionPromptHook, PermissionDecision, PermissionEvent};
