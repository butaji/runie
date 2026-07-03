//! Unified session actor module.

pub mod messages;
#[cfg(test)]
mod tests;

// Ractor-based implementation.
mod ractor_session_actor;
mod ractor_session_handle;
mod session_handlers;

pub use messages::SessionMsg;
pub use ractor_session_handle::RactorSessionHandle;
pub use session_handlers::RactorSessionActor;
