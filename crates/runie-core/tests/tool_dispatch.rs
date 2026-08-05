//! Parallel tool dispatch parity (p10): `tool_execution_end` fires in
//! completion order while toolResult messages are emitted in source order
//! (pi agent-loop.ts:489,540-548).

mod common;

use std::sync::Arc;

use futures::stream;

use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AgentTool, AgentToolResult, AssistantMessageEvent, Model,
    SimpleStreamOptions, StopReason, ToolCall, ToolResultContent, Usage, UserContent, UserMessage,
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
                    },
                },
                AssistantMessageEvent::ToolCallDelta {
                    index: 1,
                    partial: ToolCall {
                        id: "s".into(),
                        name: "slow_tool".into(),
                        arguments: serde_json::json!({}),
                    },
                },
                AssistantMessageEvent::Done {
                    stop_reason: StopReason::ToolUse,
                    usage: Usage::default(),
                },
            ]
        } else {
            vec![AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
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
