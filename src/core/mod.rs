//! Core modules for Anvil
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  TUI (ratatui) — Mission Control                                  │
//! │  Header · Stream · Panels · Input                                 │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Session (core/session.rs) — Event-driven session management      │
//! │  - Subscribes to Agent events                                   │
//! │  - Persists to JSON lines format                                │
//! │  - Tree structure for branching                                  │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Agent (core/agent.rs) — State machine + tool execution          │
//! │  - Event-driven (subscribe/listener pattern like pi)            │
//! │  - Tool registry                                                │
//! │  - Streaming support                                             │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  AI Layer (core/ai.rs) — Model abstraction                      │
//! │  - Provider trait (Anthropic, OpenAI, Ollama, etc.)             │
//! │  - Streaming infrastructure                                     │
//! │  - Token counting + cost tracking                               │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Tools (core/tools.rs) — Built-in tools                         │
//! │  - read, bash, edit, write, grep, find, ls                      │
//! │  - Input/output schema (similar to pi's tool definitions)       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

pub mod intent;
pub mod plan;
pub mod dag;
pub mod executor;

pub mod session;      // NEW: Session management with tree structure
pub mod agent;       // NEW: Event-driven agent (like pi's Agent)
pub mod ai;          // NEW: Model abstraction layer
pub mod tools;        // NEW: Tool system with schema

pub mod git;
pub mod safety;

// Re-exports for convenience
pub use intent::Intent;
pub use plan::{Plan, PlanStep, Action};
pub use dag::{DagExecutor, ExecContext, StepResult, StepState};
pub use executor::{Executor, ExecEvent};

pub use session::{Session, SessionEvent, SessionEntry};
pub use agent::{Agent, AgentEvent, AgentConfig};
pub use ai::{Model, ModelProvider, StreamingOptions, Cost};
pub use tools::{Tool, ToolInput, ToolOutput, ToolRegistry, BashTool, ReadTool, EditTool, WriteTool, GrepTool, FindTool, LsTool};
