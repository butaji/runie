//! Unified session actor module.

pub mod messages;
#[cfg(test)]
mod tests;

// mpsc-based implementation (primary).
mod session;

// Ractor-based implementation (stub for backward compat).
mod ractor_session_actor;
mod ractor_session_handle;
mod session_handlers;

pub use messages::SessionMsg;
pub use session::{spawn_session_actor, SessionHandle, RactorSessionHandle};
pub use session_handlers::RactorSessionActor;
