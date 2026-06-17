//! Application initialization helpers.

use runie_core::config_reload;
use runie_core::model::AppState;

pub fn init_scoped_models(state: &mut AppState) {
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    if let Some(scoped) = config.scoped_models() {
        state.config.scoped_models = scoped
            .iter()
            .map(|s| {
                let parts: Vec<&str> = s.split('/').collect();
                if parts.len() == 2 {
                    runie_core::model::ScopedModel {
                        provider: parts[0].to_string(),
                        name: parts[1].to_string(),
                        enabled: true,
                    }
                } else {
                    runie_core::model::ScopedModel {
                        provider: state.config.current_provider.clone(),
                        name: s.clone(),
                        enabled: true,
                    }
                }
            })
            .collect();
    } else {
        // Default: first 10 models from the unified catalog.
        state.config.scoped_models = runie_core::model_catalog::model_catalog()
            .iter()
            .take(10)
            .map(|m| runie_core::model::ScopedModel {
                provider: m.provider.clone(),
                name: m.name.clone(),
                enabled: true,
            })
            .collect();
    }
}

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

pub fn init_prompts(state: &mut AppState) {
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    let prompts_section = config.prompts();
    state.prompts = runie_core::prompts::load_prompts(
        prompts_section.default.as_deref(),
        prompts_section.custom.as_deref(),
    );
}

pub fn init_telemetry(state: &mut AppState) {
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    state.config.telemetry = runie_core::Telemetry::new(config.telemetry_enabled());
    if state.config.telemetry.is_enabled() {
        state
            .config
            .telemetry
            .track_event("startup", std::collections::HashMap::new());
    }
}

pub fn init_truncation(state: &mut AppState) {
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    state.config.truncation = config.truncation;
}

/// Apply vim_mode and other [ui] settings from the config file at startup.
/// The config file watcher only emits provider/model/theme change events, so
/// the initial vim_mode opt-in (and any future [ui] fields) must be loaded
/// here.
pub fn init_ui_config(state: &mut AppState) {
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    state.config.vim_mode = config.vim_mode();
}
