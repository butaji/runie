//! InputActor — owns the authoritative `InputState`.

pub mod actor;
pub mod input;
mod messages;

pub use input::{spawn_input_actor, InputActorBase, InputHandle, RactorInputHandle};
pub use messages::InputMsg;
