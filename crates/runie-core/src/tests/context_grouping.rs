use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::view::{Element, LazyCache};

fn tool_message(name: &str, output: &str, ts: f64) -> ChatMessage {
    ChatMessage {
        role: Role::Tool,
        parts: vec![Part::Text {
            content: format!("✓ {} 0.5s\n{}", name, output),
        }],
        timestamp: ts,
        id: format!("tool.{}.{}", name, ts),
        ..Default::default()
    }
}

#[test]
fn single_context_tool_is_not_grouped() {
    let mut state = AppState::default();
    state
        .session
        .messages
        .push(tool_message("list_dir", "a.txt", 1.0));
    state.messages_changed();

    let elems = LazyCache::rebuild(&state);
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, Element::ToolDone { name, .. } if name == "list_dir")),
        "single context tool should remain a ToolDone element"
    );
    assert!(
        !elems
            .iter()
            .any(|e| matches!(e, Element::ContextGroup { .. })),
        "single context tool should not become a ContextGroup"
    );
}

#[test]
fn multiple_context_tools_are_grouped() {
    let mut state = AppState::default();
    state
        .session
        .messages
        .push(tool_message("list_dir", "a.txt", 1.0));
    state
        .session
        .messages
        .push(tool_message("read_file", "content", 2.0));
    state
        .session
        .messages
        .push(tool_message("grep", "match", 3.0));
    state.messages_changed();

    let elems = LazyCache::rebuild(&state);
    let groups: Vec<_> = elems
        .iter()
        .filter_map(|e| match e {
            Element::ContextGroup { tools, .. } => Some(tools.len()),
            _ => None,
        })
        .collect();
    assert_eq!(
        groups,
        vec![3],
        "three consecutive context tools should form one group"
    );
    assert!(
        !elems.iter().any(|e| matches!(e, Element::ToolDone { .. })),
        "grouped tools should not also appear as standalone ToolDone elements"
    );
}

#[test]
fn action_tool_breaks_context_group() {
    let mut state = AppState::default();
    state
        .session
        .messages
        .push(tool_message("list_dir", "a.txt", 1.0));
    state
        .session
        .messages
        .push(tool_message("execute_command", "out", 2.0));
    state
        .session
        .messages
        .push(tool_message("read_file", "content", 3.0));
    state.messages_changed();

    let elems = LazyCache::rebuild(&state);
    assert!(
        !elems
            .iter()
            .any(|e| matches!(e, Element::ContextGroup { .. })),
        "single context tools separated by an action tool should not form groups"
    );
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, Element::ToolDone { name, .. } if name == "list_dir")),
        "first context tool should remain standalone"
    );
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, Element::ToolDone { name, .. } if name == "execute_command")),
        "action tool should remain a standalone ToolDone"
    );
    assert!(
        elems
            .iter()
            .any(|e| matches!(e, Element::ToolDone { name, .. } if name == "read_file")),
        "second context tool should remain standalone"
    );
}

#[test]
fn assistant_parts_render_into_elements() {
    let mut state = AppState::default();
    state.session.messages.push(ChatMessage {
        role: Role::Assistant,
        timestamp: 1.0,
        id: "a1".into(),
        parts: vec![
            Part::Text {
                content: "Let me search.".into(),
            },
            Part::Reasoning {
                content: "I need files first.".into(),
            },
            Part::ToolCall {
                id: "call_1".into(),
                name: "list_dir".into(),
                args: serde_json::json!({"path": "."}),
            },
            Part::ToolResult {
                id: "call_1".into(),
                output: "a.txt".into(),
            },
        ],
        ..Default::default()
    });
    state.messages_changed();

    let elems = LazyCache::rebuild(&state);
    assert!(elems.iter().any(
        |e| matches!(e, Element::AgentMessage { content, .. } if content == "Let me search.")
    ));
    assert!(elems.iter().any(
        |e| matches!(e, Element::ThoughtMarker { content, .. } if content == "I need files first.")
    ));
    assert!(elems
        .iter()
        .any(|e| matches!(e, Element::ToolDone { name, .. } if name == "list_dir")));
    assert!(elems
        .iter()
        .any(|e| matches!(e, Element::ToolDone { name, .. } if name == "tool")));
}
