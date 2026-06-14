//! Login flow — multi-step dialog for provider authentication.
//!
//! Steps:
//!   1. Provider picker (choose from known providers)
//!   2. API key input (form field)
//!   3. Model multi-select (toggle items) — pre-populated with the
//!      provider's `models` list. A background fetch from the provider's
//!      `/models` endpoint enriches the list when it succeeds; failures
//!      show a non-blocking warning and the defaults are kept.
//!   4. Done
//!
//! The flow is **non-blocking**: submitting an API key transitions
//! immediately to the model selector. The user is never gated on a network
//! round-trip, so the UI can never get "stuck" on validation.

pub use panels::{
    build_done_panel, build_key_input, build_login_root, build_model_selector,
    build_provider_picker,
};
pub use state::{LoginFlowState, LoginStep};

mod panels;
mod state;

#[cfg(test)]
mod validation;
