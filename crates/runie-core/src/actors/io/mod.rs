//! `IoActor` — owns user-initiated blocking IO.

pub mod actor;
pub mod messages;

pub use actor::IoActor;
pub use messages::{IoActorHandle, IoMsg};
