//! `IoActor` — owns user-initiated blocking IO.

pub mod actor;
pub mod effects;
pub mod messages;
pub mod ractor_io;

// Ractor-based IoActor (recommended).
pub use ractor_io::{RactorIoActor, RactorIoHandle};

// Legacy IoActor using custom trait (deprecated).
#[deprecated(since = "0.3.0", note = "Use RactorIoActor instead")]
pub use actor::IoActor;
pub use messages::{IoActorHandle, IoMsg};
