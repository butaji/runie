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
pub use aliases::AgentEvent;
pub use aliases::CommandEvent;
pub use aliases::ControlEvent;
pub use aliases::DialogEvent;
pub use aliases::EditEvent;
pub use aliases::InputEvent;
pub use aliases::LoginFlowEvent;
pub use aliases::ModelConfigEvent;
pub use aliases::ScrollEvent;
pub use aliases::SessionEvent;
pub use aliases::SystemEvent;
pub use durable::DurableCoreEvent;
pub use level::TransientLevel;

pub mod aliases;
pub mod constructors;
pub mod durable;
mod level;
pub mod name;
mod names;
pub mod to_durable;
mod variants;
pub mod from_provider_event;
#[cfg(test)]
mod variants_tests;
