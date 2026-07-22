//! End-to-end tests for the terminal application
#![allow(clippy::too_many_lines)] // test helpers are intentionally comprehensive

pub use crate::ui::view;
pub use ratatui::{backend::TestBackend, Terminal};
pub use runie_core::{commands::DialogKind, AppState, ChatMessage, Element, Event, Part, Role, ScopedModel, Snapshot};

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
mod agent_run_guard;
#[cfg(test)]
mod bootstrap_e2e;
#[cfg(test)]
mod frame_rate;
#[cfg(test)]
mod input_actor_routing;
#[cfg(test)]
mod login_flow_e2e;
#[cfg(test)]
mod login_flow_form;
#[cfg(test)]
mod model_cycle;
#[cfg(test)]
mod onboarding_e2e;
#[cfg(test)]
mod onboarding_input;
#[cfg(test)]
mod onboarding_render;
#[cfg(test)]
mod pattern_turn;
#[cfg(test)]
mod permission_dialog;
#[cfg(test)]
mod provider_config_e2e;
#[cfg(test)]
mod providers_e2e;
#[cfg(test)]
mod quit_shortcut;
#[cfg(test)]
mod toggle_e2e;
#[cfg(test)]
mod uiactor_init;
#[cfg(test)]
mod vim_mode;

#[cfg(test)]
mod vim_nav_enter_routing;

/// Helper: give a default state a connected model so input/status render.
pub fn connect_model(state: &mut AppState) {
    state.config.current_provider = "openai".to_string();
    state.config.current_model = "gpt-4o".to_string();
}

/// Helper: render AppState to String using the default 80x24 terminal size.
/// Use this instead of duplicating the terminal/buffer boilerplate in each test.
pub fn render_content(state: &mut AppState) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

/// Helper: render AppState with a custom terminal size.
/// Sets the viewport dimensions in state so scroll math and content wrapping
/// use the correct values matching the test backend size.
pub fn render_with_size(state: &mut AppState, width: u16, height: u16) -> String {
    // Set viewport dimensions so cache math uses correct values.
    // Content width accounts for 1-cell left/right margin applied in ui.rs.
    state.set_last_content_width(width.saturating_sub(2));
    // Message viewport: full terminal minus input box, status bar, and margins.
    // This mirrors the calculation in handle_terminal_resize().
    let viewport_height = height.saturating_sub(8).max(3);
    state.set_last_visible_height(viewport_height);

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| view(f, state)).expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
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
    runie_core::provider::config::set_test_config_path(path);
    for (name, models) in providers {
        let _ = runie_core::provider::config::save_provider_config(name, "http://test", "key", models);
    }
}

/// Helper: load the current thread's test config into `AppState`.
pub fn apply_test_config_to_state(state: &mut runie_core::AppState) {
    let path = runie_core::provider::config::config_path();
    let config = runie_core::config::Config::load(Some(&path));
    *state.config_mut().model_providers_mut() = config.model_providers;
}

/// Helper: simulate full tool flow
pub fn simulate_list_files_flow(state: &mut AppState) {
    state.update(Event::Thinking { id: "req.0".to_string() });
    state.update(Event::ThoughtDone { id: "req.0".to_string() });
    state.update(Event::ToolStart {
        id: "req.0".to_string(),
        name: "list_files".to_string(),
        input: serde_json::Value::Null,
    });
    state.update(Event::ToolEnd { id: "".to_string(), input: None, duration_secs: 1.0, output: String::new() });
    state.update(Event::Thinking { id: "req.0".to_string() });
    state.update(Event::ThoughtDone { id: "req.0".to_string() });
    state.update(Event::Response {
        id: "req.0".to_string(),
        content: "src/main.rs\nlib.rs".to_string(),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
    state.update(Event::TurnComplete { id: "req.0".to_string(), duration_secs: 3.0 });
    state.update(Event::Done { id: "req.0".to_string() });
}

/// Helper: simulate one tool call turn
pub fn simulate_tool_call(state: &mut AppState, i: usize) {
    let id = format!("req.{}", i);
    state.update(Event::Input('l'));
    state.update(Event::Submit);
    state.pop_queue();
    state.agent.streaming = true;
    state.update(Event::Thinking { id: id.clone() });
    state.update(Event::ThoughtDone { id: id.clone() });
    state.update(Event::ToolStart { id: id.clone(), name: "list_files".to_string(), input: serde_json::Value::Null });
    state.update(Event::ToolEnd { id: "".to_string(), input: None, duration_secs: 0.5, output: String::new() });
    state.update(Event::Thinking { id: id.clone() });
    state.update(Event::ThoughtDone { id: id.clone() });
    state.update(Event::Response {
        id,
        content: format!("Files for turn {}\n", i),
        role: String::new(),
        timestamp: 0.0,
        provider: String::new(),
    });
}
