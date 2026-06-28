#![warn(clippy::all)]

//! Runie IO — Async actors and IO operations.
//!
//! This crate contains all tokio-based actors, async file/network operations,
//! and business logic that requires async runtime.

extern crate self as runie_io;

// Re-export domain types for convenience.
pub use runie_domain;

/// Placeholder module to verify the crate compiles
pub mod placeholder {
    use crate::runie_domain::placeholder::is_domain as domain_is_domain;
    
    /// Returns true - this is just a placeholder
    pub fn is_io() -> bool {
        true
    }
    
    /// Returns true from domain
    pub fn has_domain() -> bool {
        domain_is_domain()
    }
}
