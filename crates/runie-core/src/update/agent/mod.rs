use crate::event::AgentEvent;
use crate::model::AppState;

mod at_refs;
mod core;
mod model_config;
mod scoped_models;
mod thought;



pub use model_config::model_config_event;

pub fn agent_event(state: &mut AppState, event: AgentEvent) {
    use AgentEvent as E;
    match event {
        E::Thinking { id } => {
            state.set_thinking(id);
            state.ensure_turn_complete_last();
        }
        E::ThoughtDone { id } => {
            state.add_thought(id);
            state.ensure_turn_complete_last();
        }
        E::ToolStart { id, name, .. } => {
            state.start_tool(id, name);
            state.ensure_turn_complete_last();
        }
        E::ToolEnd { duration_secs, output, .. } => {
            state.end_tool(duration_secs, output);
            state.ensure_turn_complete_last();
        }
        E::ResponseDelta { .. } => state.handle_llm_event(event),
        E::Response { id, content } => handle_agent_response(state, id, content),
        E::TurnComplete { id, duration_secs } => {
            state.complete_turn(id, duration_secs);
            state.ensure_turn_complete_last();
        }
        E::Done { id } => state.finish_turn(id),
        E::Error { id, message } => {
            state.add_error(id, message);
            state.ensure_turn_complete_last();
        }
        // LLM lifecycle events — populate parts during streaming
        E::TextStart { .. }
        | E::TextEnd { .. }
        | E::ThinkingStart { .. }
        | E::ThinkingEnd { .. }
        | E::ThinkingDelta { .. }
        | E::AssistantMessageReady { .. } => state.handle_llm_event(event),
        _ => {}
    }
}

fn handle_agent_response(state: &mut AppState, id: String, content: String) {
    state.append_response(id, content);
    state.ensure_turn_complete_last();
}
