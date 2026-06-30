//! InputActor — owns the authoritative `InputState`.

mod actor;
mod messages;

pub use actor::{InputActor, RactorInputHandle};
pub use messages::InputMsg;
