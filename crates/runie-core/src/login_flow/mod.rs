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

// Re-exports from handlers module.
pub use handlers::{login_flow_cancel, login_flow_event, login_flow_start};
pub(crate) use panel_ops::rebuild_login_dialog;

pub(crate) mod handlers;
pub(crate) mod panel_ops;
mod panels;
mod state;

#[cfg(test)]
mod validation;

#[cfg(test)]
mod tests;
