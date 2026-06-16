//! Centralized Event Types
//!
//! ## Architecture
//!
//! The `Event` enum is split into focused sub-enums to keep match arms small
//! and error messages readable. See [`event-subenums.md`](../tasks/event-subenums.md).
//!
//! Sub-enums (each in its own file):
//! - [`InputEvent`](input::InputEvent) — keyboard, mouse, clipboard, terminal
//! - [`AgentEvent`](agent::AgentEvent) — LLM responses, tool calls
//! - [`ScrollEvent`](scroll::ScrollEvent) — feed scroll navigation
//! - [`ControlEvent`](control::ControlEvent) — quit, reset, abort, external editor
//! - [`ModelConfigEvent`](model_config::ModelConfigEvent) — model/provider/theme switching
//! - [`DialogEvent`](dialog::DialogEvent) — command palette, model selector, path completion
//! - [`EditEvent`](edit::EditEvent) — edit preview and approval
//! - [`SystemEvent`](system::SystemEvent) — transient notifications, diagnostics
//! - [`SessionEvent`](session::SessionEvent) — session tree manipulation
//! - [`CommandEvent`](command::CommandEvent) — slash command execution
//!
//! Durable events for JSONL persistence: [`DurableCoreEvent`](durable::DurableCoreEvent)

pub use names::EVENT_NAMES;
pub use variants::Event;

// Re-export sub-enums for ergonomic external use
pub use agent::AgentEvent;
pub use control::ControlEvent;
pub use dialog_display::DialogEvent;
pub use durable::DurableCoreEvent;
pub use edit::EditEvent;
pub use input::InputEvent;
pub use level::TransientLevel;
pub use login_flow::LoginFlowEvent;
pub use model_config::ModelConfigEvent;
pub use crate::orchestrator_actor::OrchestratorEvent;
pub use scroll::ScrollEvent;
pub use session::SessionEvent;
pub use sidebar::SidebarEvent;
pub use system::SystemEvent;

mod agent;
pub mod command;
pub use command::CommandEvent;
mod control;
mod dialog;
pub mod dialog_display;
pub mod durable;
mod edit;
mod input;
mod level;
mod login_flow;
mod model_config;
mod names;
mod scroll;
mod session;
mod sidebar;
mod system;
mod variants;
