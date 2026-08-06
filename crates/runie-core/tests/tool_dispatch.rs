//! Parallel tool dispatch parity (p10): `tool_execution_end` fires in
//! completion order while toolResult messages are emitted in source order
//! (pi agent-loop.ts:489,540-548).

mod common;

use std::sync::Arc;

use futures::stream;

use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AfterToolCallResult, AgentContext, AgentMessage, AgentTool, AgentToolResult,
    AssistantMessageEvent, BeforeToolCallResult, Model, SimpleStreamOptions, StopReason, ToolCall,
    ToolResultContent, Usage, UserContent, UserMessage,
};

use common::TestLoopBuilder;

/// Fast tool: sends on `go` then returns immediately.
struct FastTool {
    go: tokio::sync::watch::Sender<bool>,
}
#[async_trait::async_trait]
impl AgentTool for FastTool {
    fn name(&self) -> &str {
        "fast_tool"
    }
    fn label(&self) -> &str {
        "Fast"
    }
    fn description(&self) -> &str {
        "Completes immediately."
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let _ = self.go.send(true);
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: "fast".into(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

/// Slow tool: waits until `go` is true before returning, so it always
/// completes after the fast tool.
struct SlowTool {
    go: tokio::sync::watch::Receiver<bool>,
}
#[async_trait::async_trait]
impl AgentTool for SlowTool {
    fn name(&self) -> &str {
        "slow_tool"
    }
    fn label(&self) -> &str {
        "Slow"
    }
    fn description(&self) -> &str {
        "Completes after fast_tool."
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let _ = self.go.clone().wait_for(|v| *v).await;
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: "slow".into(),
            }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

/// Stream that requests two tool calls on the first turn, then a plain stop
/// so the auto-continue terminates.
struct TwoToolStream {
    calls: std::sync::Mutex<usize>,
}
#[async_trait::async_trait]
impl StreamFn for TwoToolStream {
    async fn stream(
        &self,
        _m: &Model,
        _c: &AgentContext,
        _o: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let mut n = self.calls.lock().unwrap();
        *n += 1;
        let events = if *n == 1 {
            vec![
                AssistantMessageEvent::ToolCallDelta {
                    index: 0,
                    partial: ToolCall {
                        id: "f".into(),
                        name: "fast_tool".into(),
                        arguments: serde_json::json!({}),
                        thought_signature: None,
                    },
                },
                AssistantMessageEvent::ToolCallDelta {
                    index: 1,
                    partial: ToolCall {
                        id: "s".into(),
                        name: "slow_tool".into(),
                        arguments: serde_json::json!({}),
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

#[tokio::test(flavor = "multi_thread")]
async fn parallel_end_events_fire_in_completion_order() {
    let (go_tx, go_rx) = tokio::sync::watch::channel(false);
    let mut builder = TestLoopBuilder::new(Arc::new(TwoToolStream {
        calls: std::sync::Mutex::new(0),
    }));
    builder = builder.tool(Arc::new(FastTool { go: go_tx.clone() }));
    builder = builder.tool(Arc::new(SlowTool { go: go_rx }));
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

    // Collect ToolExecutionEnd order from the bus.
    let events = test.events.lock();
    assert_tool_lifecycle_order(&events, "fast_tool");
    assert_tool_lifecycle_order(&events, "slow_tool");
    let ends: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            runie_core::types::AgentEvent::ToolExecutionEnd { tool_name, .. } => {
                Some(tool_name.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(ends.len(), 2, "both tools should end");
    let fast_pos = ends.iter().position(|n| n == "fast_tool").unwrap();
    let slow_pos = ends.iter().position(|n| n == "slow_tool").unwrap();
    assert!(
        fast_pos < slow_pos,
        "tool_execution_end should fire in completion order (fast_tool first)"
    );
}

#[tokio::test]
#[allow(
    clippy::too_many_lines,
    reason = "the hook integration keeps all override assertions in one scenario"
)]
async fn after_tool_call_overrides_result_fields() {
    let (go_tx, go_rx) = tokio::sync::watch::channel(false);
    let before_context_lengths = Arc::new(std::sync::Mutex::new(Vec::new()));
    let before_context_lengths_for_hook = before_context_lengths.clone();
    let hook_context_lengths = Arc::new(std::sync::Mutex::new(Vec::new()));
    let hook_context_lengths_for_hook = hook_context_lengths.clone();
    let mut builder = TestLoopBuilder::new(Arc::new(TwoToolStream {
        calls: std::sync::Mutex::new(0),
    }));
    builder.hooks.before_tool_call = Some(Arc::new(move |input| {
        let lengths = before_context_lengths_for_hook.clone();
        Box::pin(async move {
            assert!(!input.signal.is_cancelled());
            lengths
                .lock()
                .expect("before hook context lock")
                .push(input.context.messages.len());
            BeforeToolCallResult::default()
        })
    }));
    builder.hooks.after_tool_call = Some(Arc::new(move |input| {
        let lengths = hook_context_lengths_for_hook.clone();
        Box::pin(async move {
            assert!(!input.signal.is_cancelled());
            lengths
                .lock()
                .expect("hook context lock")
                .push(input.context.messages.len());
            AfterToolCallResult {
                content: Some(vec![ToolResultContent::Text {
                    text: "overridden".into(),
                }]),
                details: Some(serde_json::json!({"hook": true})),
                is_error: Some(true),
                usage: Some(Usage {
                    output: 7,
                    ..Usage::default()
                }),
                terminate: Some(true),
            }
        })
    }));
    builder = builder.tool(Arc::new(FastTool { go: go_tx }));
    builder = builder.tool(Arc::new(SlowTool { go: go_rx }));
    let test = builder.build();
    let output = test
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
    let results: Vec<_> = output
        .iter()
        .filter_map(|message| match message {
            AgentMessage::ToolResult(result) => Some(result),
            _ => None,
        })
        .collect();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| result.is_error));
    assert!(results.iter().all(|result| result.details["hook"] == true));
    assert!(results
        .iter()
        .all(|result| result.usage.as_ref().is_some_and(|usage| usage.output == 7)));
    assert!(results.iter().all(|result| matches!(
        result.content.first(),
        Some(ToolResultContent::Text { text }) if text == "overridden"
    )));
    assert_eq!(
        hook_context_lengths
            .lock()
            .expect("hook context lock")
            .as_slice(),
        &[2, 2]
    );
    assert_eq!(
        before_context_lengths
            .lock()
            .expect("before hook context lock")
            .as_slice(),
        &[2, 2]
    );
}

fn assert_tool_lifecycle_order(events: &[runie_core::types::AgentEvent], tool_name: &str) {
    let start = events
        .iter()
        .position(|event| {
            matches!(
                event,
                runie_core::types::AgentEvent::ToolExecutionStart { tool_name: name, .. }
                    if name == tool_name
            )
        })
        .expect("tool start event");
    let end = events
        .iter()
        .position(|event| {
            matches!(
                event,
                runie_core::types::AgentEvent::ToolExecutionEnd { tool_name: name, .. }
                    if name == tool_name
            )
        })
        .expect("tool end event");
    assert!(
        start < end,
        "tool start must precede tool end for {tool_name}"
    );
}
