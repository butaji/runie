//! InputActor — owns the authoritative `InputState`.

mod actor;
mod messages;
#[cfg(test)]
mod ractor_input;

pub use actor::{InputActor, RactorInputHandle};
pub use messages::InputMsg;

// Deprecated: InputActorHandle is kept for backward compatibility during migration.
// New code should use RactorInputHandle.
#[deprecated(since = "0.2.16", note = "Use RactorInputHandle instead")]
pub type InputActorHandle = RactorInputHandle;
