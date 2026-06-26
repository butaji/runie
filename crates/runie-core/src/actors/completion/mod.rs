//! CompletionActor — owns the authoritative `CompletionState`.

mod actor;
mod messages;

pub use actor::CompletionActor;
pub use messages::{CompletionActorHandle, CompletionMsg};
