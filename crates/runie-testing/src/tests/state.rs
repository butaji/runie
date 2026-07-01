//! Shared test helpers for AppState manipulation.
//!
//! Re-exports the canonical implementations from `runie_core` so that both
//! `runie-core` tests and `runie-testing` consumers import the same helpers
//! without duplicating the logic.
//!
//! The canonical source lives in `runie-core/src/tests/support.rs`, which has
//! access to internal seeding APIs (`seed_providers`).  This module re-exports
//! the three shared helpers (`fresh_state`, `type_str`, `exec`) so that
//! `runie-testing` consumers can import from one place.
pub use runie_core::tests_support::{exec, fresh_state, type_str};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_type_str_appends() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        type_str(&mut state, "hello");
        assert_eq!(state.input().input, "hello");
        type_str(&mut state, " world");
        assert_eq!(state.input().input, "hello world");
    }

    #[test]
    fn shared_exec_submits_command() {
        let mut state = fresh_state();
        assert_eq!(state.input().input, "");
        exec(&mut state, "/save");
    }
}
