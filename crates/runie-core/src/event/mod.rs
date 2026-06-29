//! Centralized Event Types
//!
//! ## Architecture
//!
//! `Event` is a single flat enum with all leaf variants at the top level.
//!
//! Durable events for JSONL persistence: [`DurableCoreEvent`](durable::DurableCoreEvent)

pub use variants::Event;

pub use durable::DurableCoreEvent;
pub use level::TransientLevel;

pub mod constructors;
pub mod durable;
pub mod from_provider_event;
pub mod generated;
pub mod headless;
pub mod intent;
pub mod kind;
mod level;
pub mod name;
pub mod to_durable;
mod variants;

pub use generated::EventCategory;
pub use generated::EVENT_NAMES;
pub use kind::EventKind;

#[cfg(test)]
mod tests;
