//! Type aliases for backward compatibility with the old sub-enum API.
//!
//! All `FooEvent` aliases resolve to the flat [`Event`] enum. Callers should
//! prefer `Event` directly; these aliases exist to avoid churn during the
//! sub-enum → flat-enum migration.

use super::Event;

pub type AgentEvent = Event;
pub type CommandEvent = Event;
pub type ControlEvent = Event;
pub type DialogEvent = Event;
pub type EditEvent = Event;
pub type InputEvent = Event;
pub type LoginFlowEvent = Event;
pub type ModelConfigEvent = Event;
pub type ScrollEvent = Event;
pub type SessionEvent = Event;
pub type SystemEvent = Event;
