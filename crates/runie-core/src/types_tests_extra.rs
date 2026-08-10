use super::*;

#[allow(clippy::too_many_lines)]
mod extra {
    use super::*;
    #[test]
    fn agent_message_timestamp_dispatch() {
        let user = AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: "hi".into() }],
            timestamp: 42,
        });
        assert_eq!(user.timestamp(), 42);
    }

    #[test]
    fn image_content_uses_pi_base64_wire_strings() {
        let message = AgentMessage::User(UserMessage {
            content: vec![UserContent::Image {
                data: "aGVsbG8=".into(),
                mime_type: "image/png".into(),
            }],
            timestamp: 42,
        });
        let json = serde_json::to_value(&message).expect("image message serializes");
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"][0]["type"], "image");
        assert_eq!(json["content"][0]["data"], "aGVsbG8=");
        assert_eq!(json["content"][0]["mimeType"], "image/png");
        let decoded: AgentMessage = serde_json::from_value(json).expect("image message decodes");
        assert_eq!(decoded, message);

        let thinking = serde_json::to_value(AssistantContent::Thinking {
            text: "considering".into(),
        })
        .expect("thinking content serializes");
        assert_eq!(thinking["thinking"], "considering");
        assert!(thinking.get("text").is_none());
    }

    #[test]
    fn user_message_accepts_pi_string_content_sugar() {
        let message: UserMessage = serde_json::from_value(serde_json::json!({
            "content": "hello",
            "timestamp": 42
        }))
        .expect("pi string content decodes");
        assert_eq!(
            message.content,
            vec![UserContent::Text {
                text: "hello".into()
            }]
        );
        assert_eq!(message.timestamp, 42);
    }

    #[test]
    #[allow(
        clippy::cognitive_complexity,
        reason = "one wire-contract test keeps all pi event key assertions together"
    )]
    fn event_wire_shapes_use_pi_tags_and_camel_case_fields() {
        assert_tool_event_wire_shapes();
        assert_message_update_wire_shape();
        assert_stream_event_wire_shapes();
    }

    fn assert_tool_event_wire_shapes() {
        let event = AgentEvent::ToolExecutionStart {
            tool_call_id: "call-1".into(),
            tool_name: "read".into(),
            args: serde_json::json!({"path": "README.md"}),
        };
        let json = serde_json::to_value(&event).expect("agent event serializes");
        assert_eq!(json["type"], "tool_execution_start");
        assert_eq!(json["toolCallId"], "call-1");
        assert!(json.get("tool_call_id").is_none());

        let lifecycle = [
            AgentEvent::ToolExecutionUpdate {
                tool_call_id: "call-1".into(),
                tool_name: "read".into(),
                args: serde_json::json!({"path": "README.md"}),
                partial_result: serde_json::json!({"output": "line"}),
            },
            AgentEvent::ToolExecutionEnd {
                tool_call_id: "call-1".into(),
                tool_name: "read".into(),
                result: serde_json::json!({"content": [{"type": "text", "text": "done"}]}),
                is_error: false,
            },
        ];
        for event in lifecycle {
            let json = serde_json::to_value(event).expect("tool lifecycle serializes");
            assert_eq!(json["toolCallId"], "call-1");
            assert_eq!(json["toolName"], "read");
            assert!(json.get("tool_call_id").is_none());
            assert!(json.get("tool_name").is_none());
        }
    }

    fn assert_message_update_wire_shape() {
        let message_update = AgentEvent::MessageUpdate {
            message: AgentMessage::Assistant(AssistantMessage::default()),
            event: AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hi".into(),
                partial: AssistantMessage::default(),
            },
        };
        let update_json = serde_json::to_value(message_update).expect("message update serializes");
        assert!(update_json.get("event").is_none());
        assert_eq!(update_json["assistantMessageEvent"]["type"], "text_delta");
    }

    fn assert_stream_event_wire_shapes() {
        assert_text_and_thinking_wire_shapes();
        assert_terminal_stream_wire_shapes();
    }

    fn assert_text_and_thinking_wire_shapes() {
        assert_text_event_wire_shapes();
        assert_thinking_event_wire_shapes();
    }

    fn assert_text_event_wire_shapes() {
        assert_text_start_wire_shape();
        assert_text_delta_wire_shape();
        assert_text_end_wire_shape();
    }

    fn assert_text_start_wire_shape() {
        let stream_start = serde_json::to_value(AssistantMessageEvent::TextStart {
            index: 2,
            partial: AssistantMessage::default(),
        })
        .expect("text start serializes");
        assert_eq!(stream_start["contentIndex"], 2);
        assert!(stream_start.get("index").is_none());

        let assistant_start = serde_json::to_value(AssistantMessageEvent::Start {
            partial: AssistantMessage::default(),
        })
        .expect("assistant start serializes");
        assert_eq!(assistant_start["type"], "start");
        assert!(assistant_start["partial"].is_object());
    }

    fn assert_text_delta_wire_shape() {
        let text_delta = serde_json::to_value(AssistantMessageEvent::TextDelta {
            index: 2,
            delta: "hi".into(),
            partial: AssistantMessage::default(),
        })
        .expect("text delta serializes");
        assert_eq!(text_delta["contentIndex"], 2);
        assert_eq!(text_delta["delta"], "hi");
        assert!(text_delta["partial"].is_object());
    }

    fn assert_text_end_wire_shape() {
        let text_end = serde_json::to_value(AssistantMessageEvent::TextEnd {
            index: 2,
            content: "hello".into(),
            partial: AssistantMessage::default(),
        })
        .expect("text end serializes");
        assert_eq!(text_end["contentIndex"], 2);
        assert_eq!(text_end["content"], "hello");
        assert!(text_end["partial"].is_object());
    }

    fn assert_thinking_event_wire_shapes() {
        let thinking_delta = serde_json::to_value(AssistantMessageEvent::ThinkingDelta {
            index: 3,
            delta: "considering".into(),
            partial: AssistantMessage::default(),
        })
        .expect("thinking delta serializes");
        assert_eq!(thinking_delta["contentIndex"], 3);
        assert!(thinking_delta["partial"].is_object());

        let thinking_end = serde_json::to_value(AssistantMessageEvent::ThinkingEnd {
            index: 3,
            content: "considering".into(),
            elapsed_ms: Some(500),
            partial: AssistantMessage::default(),
        })
        .expect("thinking end serializes");
        assert_eq!(thinking_end["contentIndex"], 3);
        assert_eq!(thinking_end["content"], "considering");
        assert_eq!(thinking_end["elapsedMs"], 500);
        assert!(thinking_end["partial"].is_object());
    }

    fn assert_terminal_stream_wire_shapes() {
        assert_done_wire_shape();
        assert_error_wire_shape();
        assert_tool_call_end_wire_shape();
    }

    fn assert_done_wire_shape() {
        let done = serde_json::to_value(AssistantMessageEvent::Done {
            stop_reason: StopReason::ToolUse,
            usage: Usage::default(),
            message: None,
        })
        .expect("done event serializes");
        assert_eq!(done["reason"], "toolUse");
        assert!(done.get("stopReason").is_none());
        assert!(done.get("message").is_some());
    }

    fn assert_error_wire_shape() {
        let error = serde_json::to_value(AssistantMessageEvent::Error {
            reason: StopReason::Aborted,
            error: AssistantMessage::with_error(StopReason::Aborted, "aborted"),
        })
        .expect("error event serializes");
        assert_eq!(error["reason"], "aborted");
        assert_eq!(error["error"]["errorMessage"], "aborted");
    }

    fn assert_tool_call_end_wire_shape() {
        let stream = AssistantMessageEvent::ToolCallEnd {
            index: 0,
            tool_call: ToolCall {
                id: "call-1".into(),
                name: "read".into(),
                arguments: serde_json::json!({}),
                thought_signature: Some("sig".into()),
            },
            partial: AssistantMessage::default(),
        };
        let stream_json = serde_json::to_value(&stream).expect("stream event serializes");
        assert_eq!(stream_json["type"], "toolcall_end");
        assert!(stream_json.get("tool_call").is_none());
        assert!(stream_json.get("toolCall").is_some());
        assert_eq!(stream_json["toolCall"]["thoughtSignature"], "sig");
        assert!(stream_json["partial"].is_object());
    }

    #[test]
    fn background_work_events_round_trip_with_pi_style_tags() {
        let events = [
            AgentEvent::BackgroundWorkStarted {
                work_id: "worker-1".into(),
                description: "inspect".into(),
                background: true,
            },
            AgentEvent::BackgroundWorkProgress {
                work_id: "worker-1".into(),
                description: "inspect".into(),
                activity: "reading".into(),
            },
            AgentEvent::BackgroundWorkFinished {
                work_id: "worker-1".into(),
                description: "inspect".into(),
                is_error: true,
                elapsed_ms: Some(900),
                error: Some("provider stopped".into()),
            },
        ];
        for event in events {
            let json = serde_json::to_value(&event).expect("background event serializes");
            assert!(json["type"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("background_work_")));
            assert_eq!(json["workId"], "worker-1");
            let decoded: AgentEvent =
                serde_json::from_value(json).expect("background event decodes");
            assert_eq!(
                serde_json::to_value(decoded).expect("decoded event serializes"),
                serde_json::to_value(event).expect("original event serializes")
            );
        }
    }

    #[test]
    fn waiting_reason_labels_match_grok_subjects() {
        assert_eq!(WaitingReason::Model.label(), "Waiting for response…");
        assert_eq!(WaitingReason::Subagent.label(), "Waiting on subagent…");
        assert_eq!(WaitingReason::TasksComplete.label(), "Waiting on tasks…");
        assert_eq!(WaitingReason::Sleep.label(), "Sleeping…");
        assert_eq!(
            WaitingReason::TaskOutput {
                task_ids: vec!["t1".into()],
                subject: "compile project".into(),
            }
            .label(),
            "compile project…"
        );
    }

    #[test]
    fn tool_hook_payloads_use_pi_camel_case_keys() {
        let result = AgentToolResult {
            content: vec![],
            details: serde_json::json!({"ok": true}),
            usage: None,
            added_tool_names: vec!["search".into()],
            terminate: true,
        };
        let json = serde_json::to_value(result).expect("tool result serializes");
        assert_eq!(json["addedToolNames"], serde_json::json!(["search"]));
        assert_eq!(json["terminate"], true);
        assert!(json.get("added_tool_names").is_none());
        let empty_json =
            serde_json::to_value(AgentToolResult::default()).expect("empty tool result serializes");
        assert!(empty_json.get("addedToolNames").is_none());
        assert!(empty_json.get("terminate").is_none());

        let override_result = AfterToolCallResult {
            content: None,
            details: None,
            is_error: Some(true),
            usage: None,
            terminate: Some(false),
        };
        let override_json = serde_json::to_value(override_result).expect("override serializes");
        assert_eq!(override_json["isError"], true);
        assert_eq!(override_json["terminate"], false);
    }

    #[test]
    fn operation_record_wire_names_decode_only_known_pi_families() {
        assert_eq!(
            OperationRecordKind::from_wire_name("operation_started"),
            Some(OperationRecordKind::OperationStarted)
        );
        assert_eq!(
            OperationRecordKind::OperationFinished.wire_name(),
            "operation_finished"
        );
        assert_eq!(
            OperationRecordKind::from_wire_name("extension_record"),
            None
        );
    }

    #[test]
    fn provider_outcome_preserves_normalized_and_raw_finish_data() {
        let message = AssistantMessage {
            stop_reason: Some(StopReason::MaxTokens),
            raw_stop_reason: Some("max_output_tokens".into()),
            response_id: Some("response-1".into()),
            response_model: Some("routed-model".into()),
            usage: Usage {
                total_tokens: 12,
                ..Usage::default()
            },
            ..AssistantMessage::default()
        };
        let outcome = message.provider_outcome();
        assert_eq!(outcome.finish_reason, Some(StopReason::MaxTokens));
        assert_eq!(
            outcome.raw_finish_reason.as_deref(),
            Some("max_output_tokens")
        );
        assert_eq!(outcome.usage.total_tokens, 12);
        let round_trip: ProviderOutcome =
            serde_json::from_value(serde_json::to_value(outcome).unwrap()).unwrap();
        assert_eq!(round_trip, message.provider_outcome());
    }

    #[test]
    fn model_input_capabilities_are_data_queries() {
        let model = Model {
            input: vec![InputKind::Text, InputKind::Image],
            ..Model::default()
        };
        assert!(model.supports_input(InputKind::Text));
        assert!(model.supports_images());
        assert!(!Model::default().supports_images());
    }

    #[test]
    fn image_content_constructor_validates_media_data() {
        assert!(ImageContent::new("image/png", "aGVsbG8=").is_ok());
        assert!(ImageContent::new("text/plain", "aGVsbG8=").is_err());
        assert!(ImageContent::new("image/png", "not base64!").is_err());
    }

    #[test]
    fn video_content_and_capability_are_typed_data() {
        let model = Model {
            input: vec![InputKind::Video],
            ..Model::default()
        };
        assert!(model.supports_input(InputKind::Video));
        assert!(VideoContent::new("video/mp4", "aGVsbG8=").is_ok());
        assert!(VideoContent::new("image/png", "aGVsbG8=").is_err());
        let content = UserContent::Video {
            data: "aGVsbG8=".into(),
            mime_type: "video/mp4".into(),
        };
        assert_eq!(serde_json::to_value(content).unwrap()["type"], "video");
    }
}
