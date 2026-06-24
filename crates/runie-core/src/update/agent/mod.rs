use crate::event::AgentEvent;
use crate::model::AppState;

mod at_refs;
mod core;
mod model_config;
mod scoped_models;
mod thought;



pub use model_config::model_config_event;

/// Returns true for events that modify state in a way that could place
/// TurnComplete out of order. These events need `ensure_turn_complete_last`
/// called after processing.
fn event_needs_turn_complete_reorder(event: &AgentEvent) -> bool {
    matches!(
        event,
        AgentEvent::Thinking { .. }
            | AgentEvent::ThoughtDone { .. }
            | AgentEvent::ToolStart { .. }
            | AgentEvent::ToolEnd { .. }
            | AgentEvent::Response { .. }
            | AgentEvent::TurnComplete { .. }
            | AgentEvent::Error { .. }
    )
}

pub fn agent_event(state: &mut AppState, event: AgentEvent) {
    // Check before consuming event since handle_llm_event takes ownership
    let needs_reorder = event_needs_turn_complete_reorder(&event);

    use AgentEvent as E;
    match event {
        E::Thinking { id } => state.set_thinking(id),
        E::ThoughtDone { id } => state.add_thought(id),
        E::ToolStart { id, name, .. } => state.start_tool(id, name),
        E::ToolEnd { duration_secs, output, .. } => state.end_tool(duration_secs, output),
        E::ResponseDelta { id, content } => state.on_response_delta(id, content),
        E::Response { id, content } => state.append_response(id, content),
        E::TurnComplete { id, duration_secs } => state.complete_turn(id, duration_secs),
        E::Done { id } => state.finish_turn(id),
        E::Error { id, message } => state.add_error(id, message),
        // LLM lifecycle events — populate parts during streaming
        E::TextStart { .. }
        | E::TextEnd { .. }
        | E::ThinkingStart { .. }
        | E::ThinkingEnd { .. }
        | E::ThinkingDelta { .. }
        | E::AssistantMessageReady { .. } => {}
        // intentionally ignored: other agent events fall through
        _ => {}
    }

    if needs_reorder {
        state.ensure_turn_complete_last();
    }
}
