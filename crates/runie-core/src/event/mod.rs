//! Centralized Event Types
//!
//! ## Architecture
//!
//! `Event` is a single flat enum with all leaf variants at the top level.
//!
//! Durable events for JSONL persistence: [`DurableCoreEvent`](durable::DurableCoreEvent)

pub use names::EVENT_NAMES;
pub use variants::Event;

pub use durable::DurableCoreEvent;
pub use level::TransientLevel;

pub mod constructors;
pub mod durable;
pub mod from_provider_event;
pub mod intent;
pub(crate) mod intent_impl;
pub mod kind;
mod level;
pub mod name;
mod names;
pub mod to_durable;
mod variants;

pub use kind::EventKind;
#[cfg(test)]
mod variants_tests;
