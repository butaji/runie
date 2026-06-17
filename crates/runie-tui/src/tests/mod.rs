//! End-to-end tests for the terminal application

pub use ratatui::{backend::TestBackend, Terminal};
pub use runie_core::{AppState, ChatMessage, Event, Role};
pub use crate::ui::view;
use runie_core::event::{AgentEvent, InputEvent};

#[cfg(test)]
mod smoke;
#[cfg(test)]
mod snapshot;
#[cfg(test)]
mod render;

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
mod terminal_setup;
#[cfg(test)]
mod status_timer;
#[cfg(test)]
mod sticky_bottom;

#[cfg(test)]
mod toggle_e2e;
#[cfg(test)]
mod vim_mode;

/// Helper: simulate full tool flow
pub fn simulate_list_files_flow(state: &mut AppState) {
    state.update(AgentEvent::Thinking {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::ThoughtDone {
        id: "req.0".to_string(),
    });
    state.update(AgentEvent::ToolStart { id: "req.0".to_string(), name: "list_files".to_string(), input: serde_json::Value::Null });
    state.update(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 1.0, output: String::new(),
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
    state.update(AgentEvent::ToolStart { id: id.clone(), name: "list_files".to_string(), input: serde_json::Value::Null });
    state.update(AgentEvent::ToolEnd { id: "".to_string(), duration_secs: 0.5, output: String::new(),
     });
    state.update(AgentEvent::Thinking { id: id.clone() });
    state.update(AgentEvent::ThoughtDone { id: id.clone() });
    state.update(AgentEvent::Response {
        id,
        content: format!("Files for turn {}\n", i),
    });
}
