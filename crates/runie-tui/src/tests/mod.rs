//! End-to-end tests for the terminal application

pub use crate::ui::view;
pub use ratatui::{backend::TestBackend, Terminal};
use runie_core::event::{AgentEvent, InputEvent};
pub use runie_core::{AppState, ChatMessage, Event, Role};

#[cfg(test)]
mod core;
#[cfg(test)]
mod render;
#[cfg(test)]
mod smoke;
#[cfg(test)]
mod snapshot;

#[cfg(test)]
mod autoscroll_render;
#[cfg(test)]
mod dev_sh;
#[cfg(test)]
mod flow;
#[cfg(test)]
mod line_scroll;

#[cfg(test)]
mod render_actor;
#[cfg(test)]
mod semantic_render;
#[cfg(test)]
mod status_timer;
#[cfg(test)]
mod sticky_bottom;
#[cfg(test)]
mod terminal_setup;

#[cfg(test)]
mod login_flow_e2e;
#[cfg(test)]
mod model_cycle;
#[cfg(test)]
mod onboarding_e2e;
#[cfg(test)]
mod onboarding_input;
#[cfg(test)]
mod onboarding_render;
#[cfg(test)]
mod provider_config_e2e;
#[cfg(test)]
mod providers_e2e;
#[cfg(test)]
mod toggle_e2e;
#[cfg(test)]
mod vim_mode;

/// Helper: give a default state a connected model so input/status render.
pub fn connect_model(state: &mut AppState) {
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
}

/// Helper: configure connected providers for tests in this crate.
pub fn configure_test_providers(providers: &[(String, Vec<String>)]) {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = PathBuf::from(format!(
        "/tmp/runie_tui_config_{}_{}.toml",
        std::process::id(),
        n
    ));
    runie_core::login_config::set_test_config_path(path);
    for (name, models) in providers {
        let _ = runie_core::login_config::save_provider_config(name, "http://test", "key", models);
    }
}

/// Helper: simulate full tool flow
pub fn simulate_list_files_flow(state: &mut AppState) {
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::ToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 1.0,
        output: String::new(),
    });
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::Response {
        id: "req.0".to_string(),
        content: "src/main.rs\nlib.rs".to_string(),
    });
    state.update(AgentEvent::TurnComplete {
        id: "req.0".to_string(),
        duration_secs: 3.0,
    });
    state.update(AgentEvent::Done {
        id: "req.0".to_string(),
    });
}

/// Helper: simulate one tool call turn
pub fn simulate_tool_call(state: &mut AppState, i: usize) {
    let id = format!("req.{}", i);
    state.update(InputEvent::Input('l'));
    state.update(InputEvent::Submit);
    state.pop_queue();
    state.agent.streaming = true;
    state.update(AgentEvent::Thinking { id: id.clone() });
    state.update(AgentEvent::ThoughtDone { id: id.clone() });
    state.update(AgentEvent::ToolStart {
        id: id.clone(),
        name: "list_files".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(AgentEvent::ToolEnd {
        id: "".to_string(),
        duration_secs: 0.5,
        output: String::new(),
    });
    state.update(AgentEvent::Thinking { id: id.clone() });
    state.update(AgentEvent::ThoughtDone { id: id.clone() });
    state.update(AgentEvent::Response {
        id,
        content: format!("Files for turn {}\n", i),
    });
}
