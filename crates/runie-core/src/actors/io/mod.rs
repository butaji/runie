//! `IoActor` — owns user-initiated blocking IO.

pub mod effects;
pub mod messages;
pub mod ractor_io;

// mpsc-based implementation (primary).
mod io;

pub use messages::IoMsg;
pub use io::{spawn_io_actor, IoActorHandle, RactorIoHandle};
pub use ractor_io::RactorIoActor;
