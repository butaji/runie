//! TrustActor — owns trust decisions and the derived read-only flag.

mod actor;
mod messages;

mod ractor_trust;

pub use actor::TrustActor;
pub use messages::TrustMsg;

// Ractor-based implementation (migration in progress).
pub use ractor_trust::{RactorTrustActor, RactorTrustHandle};

// TrustActorHandle now points to the ractor-based handle.
pub type TrustActorHandle = RactorTrustHandle;
