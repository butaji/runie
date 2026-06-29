//! Unified session actor module.

pub mod messages;
#[cfg(test)]
mod tests;

// Ractor-based implementation.
mod ractor_session_actor;
mod ractor_session_handle;

pub use messages::SessionMsg;
pub use ractor_session_actor::RactorSessionActor;
pub use ractor_session_handle::RactorSessionHandle;
