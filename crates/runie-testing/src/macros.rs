//! Conditional-test skip macros.

/// Skip the current test if `SEATBELT=1` is set.
#[macro_export]
macro_rules! skip_if_seatbelt {
    () => {
        if std::env::var("SEATBELT").unwrap_or_default() == "1" {
            eprintln!("skipping test under SEATBELT");
            return;
        }
    };
}

/// Skip the current test if `RUNIE_INTEGRATION` is not set.
#[macro_export]
macro_rules! skip_if_integration {
    () => {
        if std::env::var("RUNIE_INTEGRATION").is_err() {
            eprintln!("skipping integration test; set RUNIE_INTEGRATION=1 to run");
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn seatbelt_macro_compiles() {
        std::env::remove_var("SEATBELT");
        skip_if_seatbelt!();
    }

    #[test]
    fn integration_macro_compiles() {
        std::env::remove_var("RUNIE_INTEGRATION");
        skip_if_integration!();
    }
}
