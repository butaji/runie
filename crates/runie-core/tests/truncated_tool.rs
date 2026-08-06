//! Verifies the output-token-limit guard: tool calls in a `MaxTokens`
//! message are failed, not executed (pi-agent-core parity).
//!
//! A `MaxTokens` stop means the provider was cut off by its output token
//! limit, so every tool call in the message may carry truncated arguments.
//! The loop must synthesize error results instead of running the tools, then
//! auto-continue to a follow-up turn (pi agent-loop.ts:216 + 405).

#![allow(
    clippy::too_many_lines,
    reason = "the truncation parity scenario keeps stream, tool, and event assertions together"
)]

mod common;

use std::sync::Arc;

use futures::stream;
use parking_lot::Mutex;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AssistantMessage, AssistantMessageEvent, Model,
    SimpleStreamOptions, StopReason, ToolCall, ToolResultContent, Usage, UserContent, UserMessage,
};

use common::{echo_tool, event_kinds, TestLoopBuilder};

/// A multi-turn stream: the first call ends with `MaxTokens` after a tool
/// call; subsequent calls return a plain `stop` so the auto-continue
/// terminates.
struct TruncatingStream {
    calls: Mutex<usize>,
}

#[async_trait::async_trait]
impl StreamFn for TruncatingStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let mut n = self.calls.lock();
        *n += 1;
        let events = if *n == 1 {
            vec![
                AssistantMessageEvent::Start,
                AssistantMessageEvent::ToolCallDelta {
                    index: 0,
                    partial: ToolCall {
                        id: "call-1".into(),
                        name: "echo".into(),
                        arguments: serde_json::json!({ "text": "hello" }),
                        thought_signature: None,
                    },
                },
                AssistantMessageEvent::Done {
                    stop_reason: StopReason::MaxTokens,
                    usage: Usage::default(),
                    message: None,
                },
            ]
        } else {
            vec![
                AssistantMessageEvent::Start,
                AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: "after".into(),
                    partial: AssistantMessage::default(),
                },
                AssistantMessageEvent::Done {
                    stop_reason: StopReason::Stop,
                    usage: Usage::default(),
                    message: None,
                },
            ]
        };
        Ok(Box::pin(stream::iter(events)))
    }
}

#[tokio::test]
async fn max_tokens_tool_calls_are_failed_not_executed() {
    let mut builder = TestLoopBuilder::new(Arc::new(TruncatingStream {
        calls: Mutex::new(0),
    }));
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

    // The loop auto-continued to a follow-up assistant turn (pi: the
    // truncated batch returns terminate:false -> hasMoreToolCalls true).
    assert!(
        outcome.iter().any(|m| matches!(m,
            AgentMessage::Assistant(a) if a.content.iter().any(|c| matches!(c, runie_core::types::AssistantContent::Text { text } if text == "after")))),
        "loop should continue after injecting the failed results"
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
