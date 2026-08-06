//! Tool argument preparation + validation parity (p09, pi agent-loop.ts:586,
//! validation logic in pi/packages/ai/src/utils/validation.ts).

#![allow(
    clippy::too_many_lines,
    reason = "tool preparation tests keep the complete wire-contract setup together"
)]

mod common;

use std::sync::Arc;

use futures::stream;
use parking_lot::Mutex;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AgentTool, AgentToolResult, AssistantMessageEvent, Model,
    SimpleStreamOptions, StopReason, ToolCall, ToolResultContent, Usage, UserContent, UserMessage,
};

use common::TestLoopBuilder;

fn tool_call_stream(name: &str) -> impl StreamFn {
    struct S {
        name: String,
        calls: Mutex<usize>,
    }
    #[async_trait::async_trait]
    impl StreamFn for S {
        async fn stream(
            &self,
            _m: &Model,
            _c: &AgentContext,
            _o: Option<SimpleStreamOptions>,
        ) -> Result<AssistantMessageEventStream, StreamError> {
            let mut n = self.calls.lock();
            *n += 1;
            let events = if *n == 1 {
                vec![
                    AssistantMessageEvent::ToolCallDelta {
                        index: 0,
                        partial: ToolCall {
                            id: "c1".into(),
                            name: self.name.clone(),
                            arguments: serde_json::json!({ "raw": 1 }),
                            thought_signature: None,
                        },
                    },
                    AssistantMessageEvent::Done {
                        stop_reason: StopReason::ToolUse,
                        usage: Usage::default(),
                        message: None,
                    },
                ]
            } else {
                vec![AssistantMessageEvent::Done {
                    stop_reason: StopReason::Stop,
                    usage: Usage::default(),
                    message: None,
                }]
            };
            Ok(Box::pin(stream::iter(events)))
        }
    }
    S {
        name: name.into(),
        calls: Mutex::new(0),
    }
}

/// Tool that records the args it executed with and prepares them first.
struct RecordingTool {
    received: Arc<Mutex<Vec<serde_json::Value>>>,
}
#[async_trait::async_trait]
impl AgentTool for RecordingTool {
    fn name(&self) -> &str {
        "rec"
    }
    fn label(&self) -> &str {
        "Rec"
    }
    fn description(&self) -> &str {
        "Records prepared args."
    }
    fn prepare_arguments(&self, args: &serde_json::Value) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "prepared": true, "orig": args }))
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        self.received.lock().push(args.clone());
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text { text: "ok".into() }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

#[tokio::test]
async fn prepare_arguments_replaces_args_before_execution() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let tool = Arc::new(RecordingTool {
        received: received.clone(),
    });
    let mut builder = TestLoopBuilder::new(Arc::new(tool_call_stream("rec")));
    builder = builder.tool(tool);
    let test = builder.build();

    test.actor
        .prompt(
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: "go".into() }],
                timestamp: 1,
            })],
            AgentContext::default(),
        )
        .await
        .unwrap();

    let received = received.lock();
    assert_eq!(received.len(), 1, "tool should have executed once");
    assert_eq!(
        received[0]["prepared"].as_bool(),
        Some(true),
        "tool should receive the prepared args"
    );
    assert_eq!(received[0]["orig"]["raw"], 1);
}

/// Tool whose validation always fails.
struct RejectingTool;
#[async_trait::async_trait]
impl AgentTool for RejectingTool {
    fn name(&self) -> &str {
        "reject"
    }
    fn label(&self) -> &str {
        "Reject"
    }
    fn description(&self) -> &str {
        "Always fails validation."
    }
    fn validate_arguments(&self, _args: &serde_json::Value) -> Result<(), String> {
        Err("invalid arg `x`: expected a number".into())
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        Err("should not run".into())
    }
}

#[tokio::test]
async fn validation_failure_produces_pi_formatted_error() {
    let tool = Arc::new(RejectingTool);
    let mut builder = TestLoopBuilder::new(Arc::new(tool_call_stream("reject")));
    builder = builder.tool(tool);
    let test = builder.build();

    let out = test
        .actor
        .prompt(
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text { text: "go".into() }],
                timestamp: 1,
            })],
            AgentContext::default(),
        )
        .await
        .unwrap();

    let result = out
        .iter()
        .find_map(|m| match m {
            AgentMessage::ToolResult(tr) => Some(tr),
            _ => None,
        })
        .expect("a tool result should be produced");
    assert!(
        result.is_error,
        "validation failure should be an error result"
    );
    let text: String = result
        .content
        .iter()
        .map(|c| match c {
            ToolResultContent::Text { text } => text.clone(),
            _ => String::new(),
        })
        .collect();
    assert!(
        text.contains("Validation failed for tool \"reject\""),
        "error should carry the pi validation header, got: {text:?}"
    );
    assert!(
        text.contains("Received arguments:"),
        "should include the args"
    );
}
