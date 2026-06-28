//! `IoActor` — owns user-initiated blocking IO.

pub mod effects;
pub mod messages;
pub mod ractor_io;

// Ractor-based IoActor.
pub use ractor_io::{RactorIoActor, RactorIoHandle};
pub use messages::IoMsg;
