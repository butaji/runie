//! MCP Tool Schema Cache and Connection Manager
//!
//! This module provides:
//! - [`SchemaCache`] - Config-fingerprinted tool schema cache with disk persistence
//! - [`McpConnectionManager`] - Central manager that owns MCP server lifecycles
//!
//! Architecture: MCP servers are spawned as async tasks managed by `McpConnectionManager`.
//! Tool schemas are cached based on a hash of the server configuration.
//!
//! # Cache Key Computation
//!
//! The cache key is computed as SHA-256 of the canonical JSON serialization of
//! the server configuration (transport, command, url, headers).

mod cache;
mod connection;

pub use cache::SchemaCache;
pub use connection::{McpConnectionManager, McpTool};
