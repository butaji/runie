//! `runie-core` — Rust port of `@earendil-works/pi-agent-core`.
//!
//! Implements the agent loop, state, events, tools, and steering/follow-up
//! queues using a single-source-of-truth actors + events architecture.
//!
//! Event sequence matches the TS original exactly:
//!
//! ```text
//! prompt("X")
//! ├─ agent_start
//! ├─ turn_start
//! ├─ message_start { userMessage }
//! ├─ message_end   { userMessage }
//! ├─ message_start { assistantMessage }
//! ├─ message_update (assistant only, possibly many)
//! ├─ message_end   { assistantMessage }
//! ├─ [tool_execution_start/update/end + toolResult messages]   (if tool calls)
//! ├─ turn_end   { message, toolResults: [] }
//! └─ agent_end  { messages: [...] }
//! ```
//!
//! See `tasks/` for the implementation plan; the test suite is the
//! behavioural contract.

pub mod background;
pub mod command_actor;
pub mod commands;
pub mod convert;
pub mod diagnostics;
mod event_dsl;
pub mod event_memo;
pub mod event_trace_yaml;
pub mod model_catalog;
pub mod noninteractive;
pub mod types;

pub mod events;
pub mod r#loop;
pub mod pi_event;
pub mod plugins;
pub mod provider;
pub mod provider_registry;
pub mod queues;
pub mod session;
pub mod session_search;
pub mod state;
pub mod tools;

pub mod hooks;

pub mod task_owner;
pub mod telemetry;

pub use event_memo::{EventMemo, SharedSnapshot};
pub use event_trace_yaml::replay_yaml;
pub use pi_event::PiAgentEvent;
pub use task_owner::ReducerActor;
pub use types::{
    AfterToolCallContext, AfterToolCallResult, AgentContext, AgentEvent, AgentMessage,
    AgentMessageExt, AgentState, AgentTool, AgentToolResult, AssistantMessage,
    AssistantMessageEvent, BeforeToolCallContext, BeforeToolCallResult, CompactionSummaryMessage,
    DeferredHandle, ImageContent, Model, QueueMode, SimpleStreamOptions, StopReason, TextContent,
    ThinkingLevel, ToolExecutionMode, ToolResultMessage, Usage, UserMessage,
};
