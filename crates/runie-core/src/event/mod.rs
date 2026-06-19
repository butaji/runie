//! Centralized Event Types
//!
//! ## Architecture
//!
//! `Event` is a single flat enum with all leaf variants at the top level.
//! The old sub-enums are preserved as type aliases for backward compatibility,
//! so `InputEvent::Submit` resolves to `Event::Submit`.
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
pub use scroll::ScrollEvent;
pub use session::SessionEvent;
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
mod system;
mod variants;
#[cfg(test)]
mod variants_tests;
