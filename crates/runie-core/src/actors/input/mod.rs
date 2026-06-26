//! InputActor — owns the authoritative `InputState`.

mod actor;
mod messages;

pub use actor::InputActor;
pub use messages::{InputActorHandle, InputMsg};
