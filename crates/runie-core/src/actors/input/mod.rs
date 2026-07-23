//! InputActor — owns the authoritative `InputState`.

pub mod input;
pub mod actor;
mod messages;

pub use input::{spawn_input_actor, InputActorBase, InputHandle, RactorInputHandle};
pub use messages::InputMsg;
