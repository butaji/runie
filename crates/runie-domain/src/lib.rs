#![warn(clippy::all)]

//! Runie Domain — Pure domain logic, no async IO.
//!
//! This crate contains all domain types, state, events, and business logic
//! that have no tokio dependencies.

extern crate self as runie_domain;

pub mod placeholder;

#[cfg(test)]
mod tests {
    use crate::placeholder::is_domain;

    #[test]
    fn domain_placeholder_works() {
        assert!(is_domain());
    }
}
