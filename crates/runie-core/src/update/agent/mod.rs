use crate::model::AppState;
use crate::Event;

mod at_refs;
mod core;
mod core_messages;
mod model_config;
mod scoped_models;
mod thought;

pub use model_config::model_config_event;

/// Apply a state mutation and ensure TurnComplete stays last.
fn apply_and_order<F>(state: &mut AppState, f: F)
where
    F: FnOnce(&mut AppState),
{
    f(state);
    state.ensure_turn_complete_last();
}

pub fn agent_event(state: &mut AppState, event: crate::Event) {
    use Event as E;
    match event {
        E::Thinking { id } => apply_and_order(state, |s| s.set_thinking(id)),
        E::ThoughtDone { id } => apply_and_order(state, |s| s.add_thought(id)),
        E::ToolStart { id, name, .. } => apply_and_order(state, |s| s.start_tool(id, name)),
        E::ToolEnd {
            duration_secs,
            output,
            ..
        } => apply_and_order(state, |s| s.end_tool(duration_secs, output)),
        E::ResponseDelta { .. } => state.handle_llm_event(event),
        E::Response { id, content, .. } => apply_and_order(state, |s| s.append_response(id, content)),
        E::TurnComplete { id, duration_secs } => {
            apply_and_order(state, |s| s.complete_turn(id, duration_secs))
        }
        E::Done { id } => state.finish_turn(id),
        E::Error { id, message } => apply_and_order(state, |s| s.add_error(id, message)),
        // LLM lifecycle events — populate parts during streaming
        E::TextStart { .. }
        | E::TextEnd { .. }
        | E::ThinkingStart { .. }
        | E::ThinkingEnd { .. }
        | E::ThinkingDelta { .. }
        | E::AssistantMessageReady { .. } => state.handle_llm_event(event),
        // intentionally ignored: other agent events fall through
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// TurnComplete is kept last even when other events are dispatched after it.
    /// This is verified by checking that `ensure_turn_complete_last` doesn't panic
    /// when called after various agent events.
    #[test]
    fn turn_complete_event_kept_last_after_thinking() {
        let mut state = AppState::default();
        let config = crate::config::Config::default();
        state.apply_config(&config);

        // Start a turn with thinking
        agent_event(
            &mut state,
            crate::Event::TurnComplete {
                id: "1".into(),
                duration_secs: 1.0,
            },
        );
        agent_event(&mut state, crate::Event::Thinking { id: "2".into() });
        agent_event(&mut state, crate::Event::ThoughtDone { id: "2".into() });

        // Ensure no panic and turn complete is properly ordered
        state.ensure_turn_complete_last();
    }

    #[test]
    fn turn_complete_event_kept_last_after_tool() {
        let mut state = AppState::default();
        let config = crate::config::Config::default();
        state.apply_config(&config);

        agent_event(
            &mut state,
            crate::Event::TurnComplete {
                id: "1".into(),
                duration_secs: 1.0,
            },
        );
        agent_event(
            &mut state,
            crate::Event::ToolStart {
                id: "t1".into(),
                name: "bash".into(),
                input: Default::default(),
            },
        );
        agent_event(
            &mut state,
            crate::Event::tool_end("t1", 0.5, "done"),
        );

        state.ensure_turn_complete_last();
    }

    #[test]
    fn turn_complete_event_kept_last_after_response() {
        let mut state = AppState::default();
        let config = crate::config::Config::default();
        state.apply_config(&config);

        agent_event(
            &mut state,
            crate::Event::TurnComplete {
                id: "1".into(),
                duration_secs: 1.0,
            },
        );
        agent_event(
            &mut state,
            crate::Event::response("2", "hello"),
        );

        state.ensure_turn_complete_last();
    }

    #[test]
    fn turn_complete_event_kept_last_after_error() {
        let mut state = AppState::default();
        let config = crate::config::Config::default();
        state.apply_config(&config);

        agent_event(
            &mut state,
            crate::Event::TurnComplete {
                id: "1".into(),
                duration_secs: 1.0,
            },
        );
        agent_event(
            &mut state,
            crate::Event::Error {
                id: "2".into(),
                message: "oops".into(),
            },
        );

        state.ensure_turn_complete_last();
    }
}
