//! TrustActor — owns trust decisions and the derived read-only flag.

mod actor;
mod messages;

pub use actor::TrustActor;
pub use messages::{TrustActorHandle, TrustMsg};
