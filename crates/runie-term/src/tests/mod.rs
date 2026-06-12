//! End-to-end tests for the terminal application

pub use runie_core::{AppState, Event, Role, ChatMessage};
pub use runie_tui::ui::view;
pub use ratatui::{backend::TestBackend, Terminal};

#[cfg(test)]
mod flow;
#[cfg(test)]
mod render;
#[cfg(test)]
mod toggle_e2e;
#[cfg(test)]
mod dev_sh;
#[cfg(test)]
mod render_actor;
#[cfg(test)]
mod sticky_bottom;
#[cfg(test)]
mod line_scroll;
#[cfg(test)]
mod status_timer;
#[cfg(test)]
mod autoscroll_render;
#[cfg(test)]
mod semantic_render;

/// Helper: simulate full tool flow
pub fn simulate_list_files_flow(state: &mut AppState) {
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentToolStart { id: "req.0".to_string(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 1.0, output: String::new() });
    state.update(Event::AgentThinking { id: "req.0".to_string() });
    state.update(Event::AgentThoughtDone { id: "req.0".to_string() });
    state.update(Event::AgentResponse { id: "req.0".to_string(), content: "src/main.rs\nlib.rs".to_string() });
    state.update(Event::AgentTurnComplete { id: "req.0".to_string(), duration_secs: 3.0 });
    state.update(Event::AgentDone { id: "req.0".to_string() });
}

/// Helper: simulate one tool call turn
pub fn simulate_tool_call(state: &mut AppState, i: usize) {
    let id = format!("req.{}", i);
    state.update(Event::Input('l'));
    state.update(Event::Submit);
    state.pop_queue();
    state.streaming = true;
    state.update(Event::AgentThinking { id: id.clone() });
    state.update(Event::AgentThoughtDone { id: id.clone() });
    state.update(Event::AgentToolStart { id: id.clone(), name: "list_files".to_string() });
    state.update(Event::AgentToolEnd { duration_secs: 0.5, output: String::new() });
    state.update(Event::AgentThinking { id: id.clone() });
    state.update(Event::AgentThoughtDone { id: id.clone() });
    state.update(Event::AgentResponse { id, content: format!("Files for turn {}\n", i) });
}
