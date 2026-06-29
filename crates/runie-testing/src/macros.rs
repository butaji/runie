//! Conditional-test skip helpers.

/// Returns true if the test should be skipped when `SEATBELT=1` is set.
pub fn is_seatbelt_enabled() -> bool {
    std::env::var("SEATBELT").unwrap_or_default() == "1"
}

/// Returns true if the test should be skipped when `RUNIE_INTEGRATION` is not set.
pub fn is_integration_test() -> bool {
    std::env::var("RUNIE_INTEGRATION").is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seatbelt_helper_compiles() {
        std::env::remove_var("SEATBELT");
        assert!(!is_seatbelt_enabled());
    }

    #[test]
    fn integration_helper_compiles() {
        std::env::remove_var("RUNIE_INTEGRATION");
        assert!(is_integration_test());
    }
}
