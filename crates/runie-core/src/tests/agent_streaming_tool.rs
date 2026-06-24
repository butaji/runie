//! Streaming tool-turn regression tests.

use crate::event::Event;
use crate::model::{AppState, Role};
use crate::tests::fresh_state;
use crate::view::LazyCache;

/// Streaming deltas that carry a tool marker must still produce a visible
/// tool result and final response after the thought is captured.
#[test]
fn streaming_tool_turn_renders_tool_result_and_final_response() {
    let mut state = fresh_state();
    let id = "req.0".to_string();

    for event in [
        Event::Thinking { id: id.clone() },
        Event::ResponseDelta {
            id: id.clone(),
            content: "I'll ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "list ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "the ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "files ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "in ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "the ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "current ".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "directory.\n".to_string(),
        },
        Event::ResponseDelta {
            id: id.clone(),
            content: "TOOL:list_dir:.".to_string(),
        },
        Event::ThoughtDone { id: id.clone() },
        Event::ToolStart {
            id: id.clone(),
            name: "list_dir".to_string(),
            input: serde_json::json!({ "path": "." }),
        },
        Event::ToolEnd {
            id: id.clone(),
            duration_secs: 0.5,
            output: "Cargo.toml\nsrc/".to_string(),
        },
        Event::Thinking { id: id.clone() },
        Event::ResponseDelta {
            id: id.clone(),
            content: "Done.".to_string(),
        },
        Event::ThoughtDone { id: id.clone() },
        Event::TurnComplete {
            id: id.clone(),
            duration_secs: 1.2,
        },
        Event::Done { id },
    ] {
        state.update(event);
    }

    let has_tool_marker = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .any(|m| m.content().contains("TOOL:"));
    assert!(
        !has_tool_marker,
        "assistant messages should not contain raw TOOL: markers"
    );

    let feed = LazyCache::feed(&state);
    let kinds: Vec<&str> = feed
        .elements
        .iter()
        .map(|e| match e {
            crate::view::Element::ToolDone { .. } => "D",
            crate::view::Element::AgentMessage { .. } => "A",
            crate::view::Element::ThoughtMarker { .. } => "T",
            _ => "?",
        })
        .collect();

    assert!(
        kinds.iter().any(|k| *k == "D"),
        "tool result should render in feed, got kinds {:?}",
        kinds
    );
    assert!(
        kinds.iter().any(|k| *k == "A"),
        "final assistant response should render in feed, got kinds {:?}",
        kinds
    );
}
