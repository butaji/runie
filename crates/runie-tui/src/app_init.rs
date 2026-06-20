//! Application initialization helpers.

use runie_core::model::AppState;

pub fn apply_trust_on_startup(state: &mut AppState) {
    let cwd = std::env::current_dir().unwrap_or_default();
    let tm = runie_core::TrustManager::load();
    match tm.decision_for(&cwd) {
        Some(runie_core::TrustDecision::Untrusted) => {
            state.config.read_only = true;
        }
        Some(runie_core::TrustDecision::Trusted) => {
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

pub fn init_skills(state: &mut AppState) {
    state.skills = runie_core::skills::load_all();
}
