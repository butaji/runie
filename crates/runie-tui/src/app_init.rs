//! Application initialization helpers.
//!
//! All startup file I/O is run off the async runtime via `spawn_blocking`
//! so the event loop can never be blocked by disk reads.

use runie_core::model::AppState;

/// Load skills, auth providers, and git info asynchronously and apply them to
/// `state`. Trust and input history are now owned by `PersistenceActor`.
pub async fn bootstrap(state: &mut AppState) {
    let (git_info, cwd_name) =
        tokio::task::spawn_blocking(runie_core::model::init_git_and_cwd)
            .await
            .unwrap_or_default();
    state.set_git_info(git_info);
    state.set_cwd_name(cwd_name);

    let skills = tokio::task::spawn_blocking(runie_core::skills::load_all)
        .await
        .unwrap_or_default();
    state.set_skills(skills);

    let auth = tokio::task::spawn_blocking(runie_core::auth::AuthStorage::load)
        .await
        .unwrap_or_default();
    let providers: Vec<String> = auth.tokens.keys().cloned().collect();
    state.set_auth_providers(providers);
}
