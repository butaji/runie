#[cfg(test)]
#[allow(
    clippy::field_reassign_with_default,
    clippy::too_many_lines,
    clippy::module_inception,
    reason = "serialization tests exercise complete parity payloads in one round-trip"
)]
mod tests {
    use crate::types::*;

    #[test]
    fn agent_context_distinguishes_omitted_and_explicit_empty_tools() {
        assert!(AgentContext::default().tools.is_none());
        assert!(AgentContext {
            tools: Some(Vec::new()),
            ..AgentContext::default()
        }
        .tools
        .expect("explicit empty tools")
        .is_empty());
    }

    #[test]
    fn thinking_level_serde_round_trip() {
        for level in [
            ThinkingLevel::Off,
            ThinkingLevel::Minimal,
            ThinkingLevel::Low,
            ThinkingLevel::Medium,
            ThinkingLevel::High,
            ThinkingLevel::XHigh,
            ThinkingLevel::Max,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: ThinkingLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[test]
    fn assistant_message_round_trips_new_parity_fields() {
        let m = parity_assistant_message();
        let json = serde_json::to_value(&m).unwrap();
        for key in parity_assistant_keys() {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        let back: AssistantMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.usage.input, 10);
        assert_eq!(back.usage.output, 20);
    }

    fn parity_assistant_message() -> AssistantMessage {
        let usage = Usage {
            input: 10,
            output: 20,
            ..Default::default()
        };
        AssistantMessage {
            content: vec![AssistantContent::Text { text: "hi".into() }],
            stop_reason: Some(StopReason::Pending),
            model: "m".into(),
            api: "anthropic".into(),
            provider: "anthropic".into(),
            response_model: Some("claude-3".into()),
            response_id: Some("resp-1".into()),
            diagnostics: vec![AssistantMessageDiagnostic {
                diagnostic_type: "recovery".into(),
                timestamp: 8,
                error: None,
                details: None,
            }],
            usage,
            deferred: None,
            thinking_elapsed_ms: None,
            error_message: Some("boom".into()),
            raw_stop_reason: Some("max_tokens".into()),
            timestamp: 7,
        }
    }

    fn parity_assistant_keys() -> [&'static str; 12] {
        [
            "content",
            "stopReason",
            "model",
            "api",
            "provider",
            "responseModel",
            "responseId",
            "diagnostics",
            "usage",
            "errorMessage",
            "rawStopReason",
            "timestamp",
        ]
    }

    #[test]
    fn agent_message_serialization_injects_pi_roles() {
        let messages = [
            AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "hello".into(),
                }],
                timestamp: 1,
            }),
            AgentMessage::Assistant(AssistantMessage::default()),
            AgentMessage::ToolResult(ToolResultMessage::default()),
            AgentMessage::CompactionSummary(CompactionSummaryMessage {
                summary: "s".into(),
                tokens_before: 1,
                timestamp: 2,
            }),
        ];
        let roles = ["user", "assistant", "toolResult", "compactionSummary"];
        for (message, role) in messages.into_iter().zip(roles) {
            let json = serde_json::to_value(&message).expect("message wire value");
            assert_eq!(json["role"], role);
            let round_trip: AgentMessage = serde_json::from_value(json).expect("message decode");
            assert_eq!(round_trip, message);
        }
    }

    #[test]
    fn tool_result_message_round_trips_new_parity_fields() {
        let m = ToolResultMessage {
            tool_call_id: "c1".into(),
            tool_name: "read".into(),
            content: vec![ToolResultContent::Text { text: "ok".into() }],
            details: serde_json::json!({ "lines": 3 }),
            usage: Some(Usage::default()),
            added_tool_names: vec!["lister".into()],
            is_error: false,
            timestamp: 1,
        };
        let json = serde_json::to_value(&m).unwrap();
        assert_tool_result_wire_keys(&json);
        let back: ToolResultMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.added_tool_names, vec!["lister".to_string()]);
    }

    fn assert_tool_result_wire_keys(json: &serde_json::Value) {
        for key in [
            "toolCallId",
            "toolName",
            "content",
            "details",
            "usage",
            "addedToolNames",
            "isError",
            "timestamp",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn tool_result_message_omits_absent_optional_pi_fields() {
        let json = serde_json::to_value(ToolResultMessage::default()).unwrap();
        assert!(json.get("details").is_none());
        assert!(json.get("usage").is_none());
        assert!(json.get("addedToolNames").is_none());

        let back: ToolResultMessage = serde_json::from_value(json).unwrap();
        assert_eq!(back.details, serde_json::Value::Null);
        assert_eq!(back.usage, None);
        assert!(back.added_tool_names.is_empty());
    }

    #[test]
    fn model_and_usage_round_trip_new_parity_fields() {
        let (m, usage) = parity_model_and_usage();
        assert_model_wire_shape(&m);
        assert_usage_wire_shape(&usage);
    }

    fn parity_model_and_usage() -> (Model, Usage) {
        let cost = CostBreakdown {
            input: 3.25,
            output: 15.75,
            ..Default::default()
        };
        let usage = Usage {
            cache_write_1h: 9,
            reasoning: 4,
            ..Default::default()
        };
        let m = Model {
            id: "m".into(),
            name: "m".into(),
            api: "a".into(),
            provider: "p".into(),
            base_url: "b".into(),
            reasoning: true,
            thinking_level_map: parity_thinking_map(),
            input: vec![InputKind::Text, InputKind::Image],
            cost: parity_model_cost(cost),
            context_window: 128,
            max_tokens: 64,
            sampling_params: parity_sampling_params(),
            headers: parity_headers(),
            compat: Some(serde_json::json!({"supports_reasoning": true})),
        };
        (m, usage)
    }

    fn parity_thinking_map() -> Option<ThinkingLevelMap> {
        Some(ThinkingLevelMap {
            high: Some("extended".into()),
            max: Some("maximum".into()),
            ..Default::default()
        })
    }

    fn parity_model_cost(cost: CostBreakdown) -> ModelCost {
        ModelCost {
            input: cost.input,
            output: cost.output,
            cache_read: cost.cache_read,
            cache_write: cost.cache_write,
            tiers: vec![ModelCostTier {
                input_tokens_above: 10_000,
                ..Default::default()
            }],
        }
    }

    fn parity_sampling_params() -> Option<std::collections::HashMap<String, serde_json::Value>> {
        Some(
            [("temperature".to_string(), serde_json::json!(0.2))]
                .into_iter()
                .collect(),
        )
    }

    fn parity_headers() -> std::collections::HashMap<String, String> {
        [("X-Test".to_string(), "1".to_string())]
            .into_iter()
            .collect()
    }

    fn assert_model_wire_shape(m: &Model) {
        let json = serde_json::to_value(m).unwrap();
        assert_eq!(json["cost"]["input"], serde_json::json!(3.25));
        assert_eq!(json["cost"]["output"], serde_json::json!(15.75));
        assert_eq!(json["cost"]["tiers"][0]["inputTokensAbove"], 10_000);
        assert_eq!(json["compat"]["supports_reasoning"], true);
        assert_eq!(json["thinkingLevelMap"]["high"], "extended");
        assert_eq!(json["thinkingLevelMap"]["max"], "maximum");
        assert_eq!(
            json["samplingParams"]["temperature"],
            serde_json::json!(0.2)
        );
        assert_model_wire_keys(&json);
    }

    fn assert_model_wire_keys(json: &serde_json::Value) {
        for key in [
            "id",
            "name",
            "api",
            "provider",
            "baseUrl",
            "reasoning",
            "thinkingLevelMap",
            "input",
            "cost",
            "contextWindow",
            "maxTokens",
            "samplingParams",
            "headers",
        ] {
            assert!(json.get(key).is_some(), "Model missing {key}");
        }
    }

    fn assert_usage_wire_shape(usage: &Usage) {
        let ujson = serde_json::to_value(usage).unwrap();
        assert!(ujson.get("cacheWrite1h").is_some());
        assert!(ujson.get("totalTokens").is_some());
        assert!(ujson.get("reasoning").is_some());
        let uback: Usage = serde_json::from_value(ujson).unwrap();
        assert_eq!(uback, *usage);
    }

    #[test]
    fn assistant_message_event_subkinds_round_trip() {
        for event in basic_assistant_events()
            .into_iter()
            .chain(tool_assistant_events())
            .chain(terminal_assistant_events())
        {
            assert_assistant_event_round_trip(event);
        }
    }

    fn basic_assistant_events() -> Vec<AssistantMessageEvent> {
        vec![
            AssistantMessageEvent::Start {
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextStart {
                index: 0,
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hi".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::TextEnd {
                index: 0,
                content: "hi".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingStart {
                index: 1,
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingDelta {
                index: 1,
                delta: "think".into(),
                partial: AssistantMessage::default(),
            },
            AssistantMessageEvent::ThinkingEnd {
                index: 1,
                content: "think".into(),
                elapsed_ms: None,
                partial: AssistantMessage::default(),
            },
        ]
    }

    fn tool_assistant_events() -> Vec<AssistantMessageEvent> {
        vec![
            AssistantMessageEvent::ToolCallStart {
                index: 0,
                partial: AssistantMessage::with_tool_call(ToolCall {
                    id: "c".into(),
                    name: "x".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                }),
            },
            AssistantMessageEvent::ToolCallDelta {
                index: 0,
                delta: "{\"path\":\"a.rs\"}".into(),
                partial: AssistantMessage::with_tool_call(ToolCall {
                    id: "c".into(),
                    name: "x".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                }),
            },
            AssistantMessageEvent::ToolCallEnd {
                index: 0,
                tool_call: ToolCall {
                    id: "c".into(),
                    name: "x".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                },
                partial: AssistantMessage::default(),
            },
        ]
    }

    fn terminal_assistant_events() -> Vec<AssistantMessageEvent> {
        vec![
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            },
            AssistantMessageEvent::Error {
                reason: StopReason::Error,
                error: AssistantMessage::with_error(StopReason::Error, "boom"),
            },
        ]
    }

    fn assert_assistant_event_round_trip(event: AssistantMessageEvent) {
        let json = serde_json::to_value(&event).unwrap();
        if matches!(&event, AssistantMessageEvent::ToolCallDelta { .. }) {
            assert_eq!(json["type"], "toolcall_delta");
            assert_eq!(json["contentIndex"], 0);
            assert_eq!(json["delta"], "{\"path\":\"a.rs\"}");
        }
        let back: AssistantMessageEvent = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), json);
    }
}
