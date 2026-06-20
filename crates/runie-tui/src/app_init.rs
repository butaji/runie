//! Application initialization helpers.
//!
//! All startup file I/O is run off the async runtime via `spawn_blocking`
//! so the event loop can never be blocked by disk reads.

use runie_core::model::AppState;
use runie_core::TrustDecision;

/// Load trust, skills, auth providers, and git info asynchronously and apply
/// them to `state`. This is the single production initialization path.
pub async fn bootstrap(state: &mut AppState) {
    let (git_info, cwd_name) =
        tokio::task::spawn_blocking(runie_core::model::init_git_and_cwd)
            .await
            .unwrap_or_default();
    state.git_info = git_info;
    state.cwd_name = cwd_name;

    let cwd = std::env::current_dir().unwrap_or_default();
    let decision = tokio::task::spawn_blocking({
        let cwd = cwd.clone();
        move || {
            let tm = runie_core::TrustManager::load();
            (cwd.clone(), tm.decision_for(&cwd))
        }
    })
    .await
    .ok();
    if let Some((cwd, decision)) = decision {
        apply_trust(state, &cwd, decision);
    }

    let skills = tokio::task::spawn_blocking(runie_core::skills::load_all)
        .await
        .unwrap_or_default();
    state.skills = skills;

    let auth = tokio::task::spawn_blocking(runie_core::auth::AuthStorage::load)
        .await
        .unwrap_or_default();
    let providers: Vec<String> = auth.tokens.keys().cloned().collect();
    state.set_auth_providers(providers);
}

fn apply_trust(
    state: &mut AppState,
    cwd: &std::path::Path,
    decision: Option<TrustDecision>,
) {
    match decision {
        Some(TrustDecision::Untrusted) => {
            state.config.read_only = true;
        }
        Some(TrustDecision::Trusted) => {
            state.config.read_only = false;
        }
        None => {
            state.config.read_only = false;
            state.session.messages.push(runie_core::ChatMessage {
                role: runie_core::Role::System,
                content: format!(
                    "Welcome to runie in {}.\n\nThis project is not yet trusted. \
                    Run /trust to enable write tools, or /untrust to enforce read-only mode.",
                    cwd.display()
                ),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0),
                id: "trust_welcome".to_string(),
                ..Default::default()
            });
            state.messages_changed();
        }
    }
}
