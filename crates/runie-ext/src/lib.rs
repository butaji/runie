//! # runie-ext - Extension System
//!
//! Unified plugin architecture for runie. Extensions are isolated crates
//! that plug into runie via this API.
//!
//! ## Extension Types
//!
//! - **Hooks**: Pre/post action interceptors (on_event, pre_tool, post_tool)
//! - **Plugins**: Full-featured extensions with commands, UI, state
//! - **Skills**: Agent-capable extensions invoked by LLM
//! - **MCP Servers**: External processes communicating via JSON-RPC over stdio
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        runie-tui                            │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │              PluginRegistry                          │   │
//! │  │  ┌──────────┬──────────┬──────────┬──────────┐     │   │
//! │  │  │  Hooks   │ Plugins  │  Skills  │   MCP    │     │   │
//! │  │  │ Registry │ Registry │ Registry │ Registry │     │   │
//! │  │  └──────────┴──────────┴──────────┴──────────┘     │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!          ┌───────────────────┼───────────────────┐
//!          ▼                   ▼                   ▼
//!    ┌───────────┐      ┌───────────┐      ┌───────────┐
//!    │ runie-ext │      │ runie-ext │      │ runie-ext │
//!    │  /hooks   │      │ /plugins  │      │   /mcp    │
//!    └───────────┘      └───────────┘      └───────────┘
//! ```
//!
//! ## Loading Strategy
//!
//! 1. **Static**: Compile-time registration via `runie-ext::plugin!` macro
//! 2. **Dynamic**: Runtime loading from `~/.runie/extensions/` directory
//! 3. **Remote**: Marketplace download + local cache

pub mod plugin;
pub mod registry;
pub mod plugin_loader;
pub mod hooks;
pub mod skills;
pub mod mcp;
pub mod marketplace;
pub mod error;

pub use plugin::{Plugin, PluginEvent, PluginAction, PluginMetadata, SlashCommand,
    ExtensionType, PluginCommand, CommandHandler, MessageRole, FileChangeType,
    NotificationUrgency, UiComponent, Panel, CommandArg};
pub use registry::{ExtensionRegistry, ExtensionId, LoadedExtension};
pub use plugin_loader::{PluginLoader, PluginManifest, GitPlugin, TimerPlugin};
pub use hooks::{Hook, HookEvent, HookResult, HookPriority};
pub use skills::{Skill, SkillInvocation, SkillResult};
pub use mcp::{McpRegistry, McpServerConfig, McpTransport, McpServerInfo, McpToolResult,
    McpRequest, McpResponse, McpTool, McpContent, McpError};
pub use marketplace::{MarketplaceClient, ExtensionListing, ExtensionDetails, ExtensionAsset,
    MarketplaceError};
pub use error::ExtError;

// Re-export runie-core types for convenience
pub use runie_core::{Tool, ToolOutput, Session, Message, Context};
