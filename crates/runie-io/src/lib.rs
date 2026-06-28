#![warn(clippy::all)]

//! Runie IO — Async actors and IO operations.
//!
//! This crate provides async IO actors and file/network operations.
//! It re-exports everything from `runie_domain` (which includes `runie_core`).

extern crate self as runie_io;

// Re-export domain types for convenience
pub use runie_domain;

#[cfg(test)]
mod tests {
    /// Verify the IO crate works.
    #[test]
    fn io_crate_works() {
        // Can use types from runie_domain through this crate
        let _config = runie_domain::config::Config::default();
    }
}
