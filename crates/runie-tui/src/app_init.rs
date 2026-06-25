//! Application initialization helpers.
//!
//! All startup file I/O is run off the async runtime via `spawn_blocking`
//! so the event loop can never be blocked by disk reads.

use runie_core::model::AppState;

/// Load skills, auth providers, and git info asynchronously and apply them to
/// `state`. Trust and input history are now owned by `PersistenceActor`.
///
/// Environment detection (git info, cwd name) is sent to `IoActor` as an
/// intent; the actor emits `Event::EnvDetected` which is then applied to
/// `AppState` through the normal event dispatch path.
pub async fn bootstrap(state: &mut AppState) {
    // Send environment detection to IoActor (async, off the blocking thread)
    if let Some(ref io_handle) = state.actor_handles().and_then(|h| h.io.as_ref()) {
        io_handle.detect_env().await;
    }

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
