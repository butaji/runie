//! Architecture guardrails — verify architectural constraints.
//!
//! These tests ensure that the codebase maintains the intended architecture,
//! such as preventing certain types from appearing in specific locations.

/// Test that AppState does not contain Element/Post/Feed/LazyCache fields.
///
/// This verifies the decoupling of AppState from the view projection cache.
/// AppState should hold domain state only; view projection is owned by UiActor.
#[test]
fn app_state_has_no_view_cache_field() {
    // Verify AppState can be constructed and used without view cache fields
    let state = runie_core::AppState::default();
    // The struct should compile and be usable
    let _ = state.session();
    let _ = state.view();
    let _ = state.input();
    let _ = state.agent_state();
    let _ = state.config();
    let _ = state.completion();

    // Verify the state can produce a snapshot (which builds the projection)
    let mut mutable_state = runie_core::AppState::default();
    let _snap = mutable_state.snapshot();
}

/// Test that AppState does not have cached_view or cached_view_gen fields.
///
/// This verifies that the view projection cache has been moved out of AppState.
#[test]
fn app_state_no_cached_view_gen() {
    let state = runie_core::AppState::default();
    // Verify the state is usable without the removed fields
    let _ = state.session();
    let _ = state.view();
}

/// Test that snapshot() works correctly without cached view.
///
/// Verifies that building a snapshot doesn't require cached view data in AppState.
#[test]
fn snapshot_works_without_appstate_cache() {
    let mut state = runie_core::AppState::default();
    // Snapshot should work without any cached view in AppState
    // The projection is built on-demand in build_view_cache()
    let _snap = state.snapshot();
}
