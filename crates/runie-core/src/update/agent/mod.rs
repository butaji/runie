use crate::event::AgentEvent;
use crate::model::AppState;

mod at_refs;
mod core;
mod model_config;
mod scoped_models;
mod thought;

pub use model_config::model_config_event;

pub fn agent_event(state: &mut AppState, event: AgentEvent) {
    match event {
        AgentEvent::Thinking { id } => {
            state.set_thinking(id);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ThoughtDone { id } => {
            state.add_thought(id);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ToolStart { id, name, .. } => {
            state.start_tool(id, name);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ToolEnd {
            duration_secs,
            output,
            ..
        } => {
            state.end_tool(duration_secs, output);
            state.ensure_turn_complete_last();
        }
        AgentEvent::ResponseDelta { id, content } => {
            state.append_response_delta(id, content);
        }
        AgentEvent::Response { id, content } => {
            handle_agent_response(state, id, content);
        }
        AgentEvent::TurnComplete { id, duration_secs } => {
            state.complete_turn(id, duration_secs);
            state.ensure_turn_complete_last();
        }
        AgentEvent::Done { id } => state.finish_turn(id),
        AgentEvent::Error { id, message } => {
            state.add_error(id, message);
            state.ensure_turn_complete_last();
        }
        _ => {}
    }
}

fn handle_agent_response(state: &mut AppState, id: String, content: String) {
    state.append_response(id, content);
    state.ensure_turn_complete_last();
}
