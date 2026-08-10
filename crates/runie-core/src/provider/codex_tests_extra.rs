use super::*;
#[test]
fn production_adapter_builds_codex_input_from_context() {
    let adapter = CodexWebSocketAdapter::production(None);
    let context = AgentContext {
        system_prompt: "be concise".into(),
        messages: vec![crate::types::AgentMessage::User(
            crate::types::UserMessage {
                content: vec![crate::types::UserContent::Text {
                    text: "hello".into(),
                }],
                timestamp: 1,
            },
        )],
        tools: None,
    };
    let value = (adapter.request_builder)(
        &Model {
            id: "gpt".into(),
            ..Default::default()
        },
        &context,
        None,
    )
    .expect("production request");
    assert_eq!(value["model"], "gpt");
    assert_eq!(value["instructions"], "be concise");
    assert_eq!(value["input"][0]["role"], "user");
}
