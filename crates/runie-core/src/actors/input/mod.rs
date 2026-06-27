//! InputActor — owns the authoritative `InputState`.

mod actor;
mod messages;
#[cfg(test)]
mod ractor_input;

pub use actor::InputActor;
pub use messages::{InputActorHandle, InputMsg};
