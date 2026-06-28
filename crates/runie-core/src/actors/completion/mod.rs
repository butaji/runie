//! CompletionActor — owns the authoritative `CompletionState`.

mod actor;
mod messages;

mod ractor_completion;

pub use actor::CompletionActor;
pub use messages::CompletionMsg;

// Ractor-based implementation (migration in progress).
#[allow(unused_imports)]
pub use ractor_completion::{RactorCompletionActor, RactorCompletionHandle};

// CompletionActorHandle now points to the ractor-based handle.
pub type CompletionActorHandle = RactorCompletionHandle;
