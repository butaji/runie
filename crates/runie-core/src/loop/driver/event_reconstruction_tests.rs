use super::*;
use crate::types::AssistantContent;

#[test]
fn operation_record_kind_has_pi_wire_names() {
    assert_eq!(
        crate::types::OperationRecordKind::OperationStarted.wire_name(),
        "operation_started"
    );
    assert_eq!(
        crate::types::OperationRecordKind::OperationFinished.wire_name(),
        "operation_finished"
    );
}

#[test]
fn sampling_params_merge_request_over_model_defaults() {
    let model: std::collections::HashMap<_, _> = [
        ("temperature".into(), serde_json::json!(0.2)),
        ("top_p".into(), serde_json::json!(0.9)),
    ]
    .into_iter()
    .collect();
    let request: std::collections::HashMap<_, _> = [("temperature".into(), serde_json::json!(0.7))]
        .into_iter()
        .collect();
    let merged = merge_sampling_params(Some(&model), Some(request)).unwrap();
    assert_eq!(merged["temperature"], serde_json::json!(0.7));
    assert_eq!(merged["top_p"], serde_json::json!(0.9));
}

#[test]
fn headers_merge_request_values_over_model_defaults() {
    let model = [
        ("x-model".into(), "model".into()),
        ("x-shared".into(), "model".into()),
    ]
    .into_iter()
    .collect();
    let request = [
        ("x-request".into(), "request".into()),
        ("x-shared".into(), "request".into()),
    ]
    .into_iter()
    .collect();
    let merged = merge_headers(&model, Some(request)).expect("headers are preserved");
    assert_eq!(merged["x-model"], "model");
    assert_eq!(merged["x-request"], "request");
    assert_eq!(merged["x-shared"], "request");
}

#[test]
fn stream_updates_replace_provider_placeholder_with_owned_partial() {
    let assistant = AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "hello".into(),
        }],
        ..AssistantMessage::default()
    };
    let event = enrich_assistant_partial(
        AssistantMessageEvent::TextDelta {
            index: 0,
            delta: "hello".into(),
            partial: AssistantMessage::default(),
        },
        &assistant,
    );
    let AssistantMessageEvent::TextDelta { partial, .. } = event else {
        panic!("text delta remains a text delta")
    };
    assert_eq!(partial, assistant);
}

#[test]
fn tool_call_markers_reconstruct_one_content_block() {
    let mut assistant = AssistantMessage::default();
    apply_tool_call_sequence(&mut assistant);
    assert_eq!(assistant.content.len(), 1);
    assert!(
        matches!(&assistant.content[0], AssistantContent::ToolCall(call) if call.arguments == serde_json::json!({"path": "a.rs"}))
    );
}

fn apply_tool_call_sequence(assistant: &mut AssistantMessage) {
    let partial = ToolCall {
        id: "call-1".into(),
        name: "read".into(),
        arguments: serde_json::json!({"path": "a"}),
        thought_signature: None,
    };
    apply_event(
        assistant,
        AssistantMessageEvent::ToolCallStart {
            index: 0,
            partial: AssistantMessage::with_tool_call(partial.clone()),
        },
    );
    apply_event(
        assistant,
        AssistantMessageEvent::ToolCallDelta {
            index: 0,
            delta: "{\"path\":\"a.rs\"}".into(),
            partial: AssistantMessage::with_tool_call(ToolCall {
                arguments: serde_json::json!({"path": "a.rs"}),
                ..partial.clone()
            }),
        },
    );
    apply_event(
        assistant,
        AssistantMessageEvent::ToolCallEnd {
            index: 0,
            tool_call: ToolCall {
                arguments: serde_json::json!({"path": "a.rs"}),
                ..partial
            },
            partial: AssistantMessage::default(),
        },
    );
}
