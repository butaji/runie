//! Mock provider flag handling for the TUI binary.

use runie_core::provider::{set_mock_enabled, set_mock_onboarding};

/// If `mock` is true, enable the mock provider for this process.
/// An optional `model` is propagated via `RUNIE_MOCK_MODEL` so config/state
/// resolution and the provider builder can select the right fixture.
///
/// `mock_onboarding` enables the mock provider and forces the onboarding dialog
/// to open so the user can select the mock provider/model explicitly.
pub fn enable_mock_if_requested(mock: bool, mock_onboarding: bool, model: Option<&str>) {
    if mock || mock_onboarding {
        set_mock_enabled(true);
    }
    if mock_onboarding {
        set_mock_onboarding(true);
    }
    if let Some(model) = model {
        std::env::set_var("RUNIE_MOCK_MODEL", model);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::provider::{is_mock_enabled, is_mock_onboarding, mock_model};

    fn reset_mock_state() {
        runie_core::provider::set_mock_enabled(false);
        runie_core::provider::set_mock_onboarding(false);
        std::env::remove_var("RUNIE_MOCK_MODEL");
    }

    #[test]
    fn mock_flag_enables_mock_provider() {
        let _guard = runie_testing::ENV_LOCK.lock().unwrap();
        reset_mock_state();
        assert!(!is_mock_enabled(), "mock should start disabled");

        enable_mock_if_requested(true, false, None);
        assert!(
            is_mock_enabled(),
            "mock should be enabled after --mock flag"
        );
        assert!(
            !is_mock_onboarding(),
            "mock-onboarding should not be enabled by --mock"
        );
        assert_eq!(mock_model(), "echo", "default mock model should be echo");

        reset_mock_state();
    }

    #[test]
    fn mock_model_flag_selects_fixture() {
        let _guard = runie_testing::ENV_LOCK.lock().unwrap();
        reset_mock_state();

        enable_mock_if_requested(true, false, Some("list_dir"));
        assert!(is_mock_enabled(), "mock should be enabled");
        assert_eq!(mock_model(), "list_dir", "mock model should be list_dir");

        reset_mock_state();
    }

    #[test]
    fn mock_onboarding_flag_enables_mock_and_forces_onboarding() {
        let _guard = runie_testing::ENV_LOCK.lock().unwrap();
        reset_mock_state();
        assert!(!is_mock_enabled(), "mock should start disabled");
        assert!(!is_mock_onboarding(), "mock-onboarding should start disabled");

        enable_mock_if_requested(false, true, None);
        assert!(
            is_mock_enabled(),
            "mock provider should be visible with --mock-onboarding"
        );
        assert!(
            is_mock_onboarding(),
            "mock-onboarding mode should be enabled"
        );

        reset_mock_state();
    }
}
