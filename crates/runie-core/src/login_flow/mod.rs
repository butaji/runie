//! Login flow — multi-step dialog for provider authentication.
//!
//! Steps:
//!   1. Provider picker (choose from known providers)
//!   2. API key input (form field)
//!   3. Validating — background `/models` call verifies the API key
//!   4. Model multi-select (toggle items) — populated from the validated
//!      `/models` response
//!   5. Done
//!
//! The user cannot reach the model selector or save a provider until the
//! API key has been successfully verified.

pub use panels::{
    build_done_panel, build_key_input, build_login_root, build_model_selector,
    build_provider_picker, build_validating_panel,
};
pub use state::{LoginFlowState, LoginStep};

mod panels;
mod state;

#[cfg(test)]
mod state_tests;

#[cfg(test)]
mod validation;
