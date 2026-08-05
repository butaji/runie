//! Verifies the output-token-limit guard: tool calls in a `MaxTokens`
//! message are failed, not executed (pi-agent-core parity).
//!
//! A `MaxTokens` stop means the provider was cut off by its output token
//! limit, so every tool call in the message may carry truncated arguments.
//! The loop must synthesize error results instead of running the tools.

mod common;

use std::sync::Arc;

use futures::stream;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AssistantMessageEvent, Model, SimpleStreamOptions, StopReason,
    ToolCall, ToolResultContent, Usage, UserContent, UserMessage,
};

use common::{echo_tool, event_kinds, TestLoopBuilder};

/// A single-turn stream that ends with a `MaxTokens` stop after requesting a
/// tool call.
struct TruncatingStream {
    events: Vec<AssistantMessageEvent>,
}

#[async_trait::async_trait]
impl StreamFn for TruncatingStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Ok(Box::pin(stream::iter(self.events.clone())))
    }
}

fn truncated_stream() -> TruncatingStream {
    TruncatingStream {
        events: vec![
            AssistantMessageEvent::Start,
            AssistantMessageEvent::ToolCallDelta {
                index: 0,
                partial: ToolCall {
                    id: "call-1".into(),
                    name: "echo".into(),
                    arguments: serde_json::json!({ "text": "hello" }),
                },
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::MaxTokens,
                usage: Usage::default(),
            },
        ],
    }
}

#[tokio::test]
async fn max_tokens_tool_calls_are_failed_not_executed() {
    let mut builder = TestLoopBuilder::new(Arc::new(truncated_stream()));
    builder = builder.tool(echo_tool());
    let test = builder.build();

    let outcome = test
        .actor
        .prompt(
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "please echo".into(),
                }],
                timestamp: 1,
            })],
            AgentContext::default(),
        )
        .await
        .unwrap();

    // The tool was reported as an error, not executed: the result must be
    // flagged `is_error` and carry the truncation notice.
    let failed: Vec<_> = outcome
        .iter()
        .filter_map(|m| match m {
            AgentMessage::ToolResult(tr) => Some(tr),
            _ => None,
        })
        .collect();
    assert_eq!(failed.len(), 1, "expected exactly one tool result");
    assert!(failed[0].is_error, "tool should be failed, not executed");
    let text: String = failed[0]
        .content
        .iter()
        .map(|c| match c {
            ToolResultContent::Text { text } => text.clone(),
            _ => String::new(),
        })
        .collect();
    assert!(
        text.contains("was not executed"),
        "error result should explain the truncation, got: {text:?}"
    );

    // The tool was surfaced as start/end events with the error flag.
    let kinds = event_kinds(&test.events.lock());
    assert!(kinds.contains(&"ToolExecutionStart"));
    assert!(kinds.contains(&"ToolExecutionEnd"));
    let ended_error = test.events.lock().iter().any(|e| {
        matches!(
            e,
            runie_core::types::AgentEvent::ToolExecutionEnd { is_error: true, .. }
        )
    });
    assert!(ended_error, "ToolExecutionEnd should carry is_error=true");
}
