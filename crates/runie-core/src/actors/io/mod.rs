//! `IoActor` — owns user-initiated blocking IO.

pub mod effects;
pub mod messages;
pub mod ractor_io;

// Ractor-based IoActor.
pub use messages::IoMsg;
pub use ractor_io::{RactorIoActor, RactorIoHandle};
