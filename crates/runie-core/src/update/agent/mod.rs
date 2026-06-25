use crate::model::AppState;
use crate::Event;

mod at_refs;
mod core;
mod core_messages;
mod model_config;
mod scoped_models;
mod thought;

pub use model_config::model_config_event;

/// Helper to apply state mutation and ensure TurnComplete stays last.
macro_rules! with_ordering {
    ($state:expr_2021, $apply:expr_2021) => {{
        $apply;
        $state.ensure_turn_complete_last();
    }};
}

pub fn agent_event(state: &mut AppState, event: crate::Event) {
    use Event as E;
    match event {
        E::Thinking { id } => with_ordering!(state, state.set_thinking(id)),
        E::ThoughtDone { id } => with_ordering!(state, state.add_thought(id)),
        E::ToolStart { id, name, .. } => with_ordering!(state, state.start_tool(id, name)),
        E::ToolEnd {
            duration_secs,
            output,
            ..
        } => {
            with_ordering!(state, state.end_tool(duration_secs, output))
        }
        E::ResponseDelta { .. } => state.handle_llm_event(event),
        E::Response { id, content } => with_ordering!(state, state.append_response(id, content)),
        E::TurnComplete { id, duration_secs } => {
            with_ordering!(state, state.complete_turn(id, duration_secs))
        }
        E::Done { id } => state.finish_turn(id),
        E::Error { id, message } => with_ordering!(state, state.add_error(id, message)),
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
            crate::Event::ToolEnd {
                id: "t1".into(),
                duration_secs: 0.5,
                output: "done".into(),
            },
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
            crate::Event::Response {
                id: "2".into(),
                content: "hello".into(),
            },
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
