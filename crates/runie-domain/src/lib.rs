#![warn(clippy::all)]

//! Runie Domain — Pure domain logic + runie-core facade.
//!
//! This crate provides the pure domain logic types that are used by both
//! the TUI and headless modes. For headless-only use cases, this crate
//! can be used directly without the async actor infrastructure.
//!
//! Internally, this crate re-exports all types from `runie-core` for
//! backward compatibility. The async actor types are available through
//! `runie_io`.

extern crate self as runie_domain;

// Re-export everything from runie-core for convenience
// This maintains backward compatibility while establishing the domain layer
pub use runie_core::*;

#[cfg(test)]
mod tests {
    /// Verify the domain facade works.
    #[test]
    fn domain_facade_works() {
        // Can use types from runie-core through this facade
        let _config = runie_core::config::Config::default();
    }
}
