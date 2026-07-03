//! Mock provider flag handling for the TUI binary.

use runie_core::provider::set_mock_enabled;

/// If `mock` is true, enable the mock provider for this process.
/// An optional `model` is propagated via `RUNIE_MOCK_MODEL` so config/state
/// resolution and the provider builder can select the right fixture.
pub fn enable_mock_if_requested(mock: bool, model: Option<&str>) {
    if mock {
        set_mock_enabled(true);
    }
    if let Some(model) = model {
        std::env::set_var("RUNIE_MOCK_MODEL", model);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::provider::{is_mock_enabled, mock_model};

    #[test]
    fn mock_flag_enables_mock_provider() {
        let _guard = runie_testing::ENV_LOCK.lock().unwrap();
        runie_core::provider::set_mock_enabled(false);
        std::env::remove_var("RUNIE_MOCK_MODEL");
        assert!(!is_mock_enabled(), "mock should start disabled");

        enable_mock_if_requested(true, None);
        assert!(
            is_mock_enabled(),
            "mock should be enabled after --mock flag"
        );
        assert_eq!(mock_model(), "echo", "default mock model should be echo");

        runie_core::provider::set_mock_enabled(false);
    }

    #[test]
    fn mock_model_flag_selects_fixture() {
        let _guard = runie_testing::ENV_LOCK.lock().unwrap();
        runie_core::provider::set_mock_enabled(false);
        std::env::remove_var("RUNIE_MOCK_MODEL");

        enable_mock_if_requested(true, Some("list_dir"));
        assert!(is_mock_enabled(), "mock should be enabled");
        assert_eq!(mock_model(), "list_dir", "mock model should be list_dir");

        std::env::remove_var("RUNIE_MOCK_MODEL");
        runie_core::provider::set_mock_enabled(false);
    }
}
