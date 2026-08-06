//! State-machine projection parity (p12): `is_streaming`, `streaming_message`,
//! `pending_tool_calls` are rebuilt from the run's events (pi AgentState).

#![allow(
    clippy::too_many_lines,
    reason = "projection tests keep setup, checkpoint assertions, and release together"
)]

mod common;

use std::sync::Arc;

use futures::stream;
use futures::StreamExt;
use parking_lot::Mutex;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AgentTool, AgentToolResult, AssistantMessageEvent, Model,
    SimpleStreamOptions, StopReason, ToolCall, ToolResultContent, Usage, UserContent, UserMessage,
};

use common::TestLoopBuilder;

/// Stream that signals `started`, yields text, then blocks until released and
/// finishes with `Done{stop}`. Lets the test observe the streaming projection.
struct PauseStream {
    started: tokio::sync::watch::Sender<bool>,
    release: Mutex<Option<tokio::sync::watch::Receiver<bool>>>,
}

#[async_trait::async_trait]
impl StreamFn for PauseStream {
    async fn stream(
        &self,
        _m: &Model,
        _c: &AgentContext,
        _o: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let started = self.started.clone();
        let release = self.release.lock().take();
        let head = futures::stream::once(async move {
            let _ = started.send(true);
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "hi".into(),
            }
        });
        let tail = futures::stream::once(async move {
            if let Some(mut rx) = release {
                let _ = rx.wait_for(|v| *v).await;
            }
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            }
        });
        Ok(Box::pin(head.chain(tail)))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn streaming_projection_tracks_live_assistant_message() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let (release_tx, release_rx) = tokio::sync::watch::channel(false);
    let stream = Arc::new(PauseStream {
        started: started_tx,
        release: Mutex::new(Some(release_rx)),
    });
    let test = TestLoopBuilder::new(stream).build();

    let run = {
        let actor = test.actor.clone();
        tokio::spawn(async move {
            actor
                .prompt(
                    vec![AgentMessage::User(UserMessage {
                        content: vec![UserContent::Text { text: "hi".into() }],
                        timestamp: 1,
                    })],
                    AgentContext::default(),
                )
                .await
        })
    };
    // Wait until the assistant stream has begun.
    while !*started_rx.borrow() {
        let _ = started_rx.changed().await;
    }
    let mut mid = test.state.snapshot();
    for _ in 0..500 {
        if mid.is_streaming {
            break;
        }
        tokio::task::yield_now().await;
        mid = test.state.snapshot();
    }
    assert!(mid.is_streaming, "is_streaming should be true mid-stream");
    assert!(
        matches!(mid.streaming_message, Some(AgentMessage::Assistant(_))),
        "streaming_message should be Some(assistant) mid-stream"
    );

    let _ = release_tx.send(true);
    run.await.expect("run task").expect("run completes");
    test.state.sync().await;
    let after = test.state.snapshot();
    assert!(
        !after.is_streaming,
        "is_streaming should clear after the turn"
    );
    assert!(after.streaming_message.is_none());
}

/// Tool that blocks until released, so the projection can observe the
/// pending tool call in flight.
struct BlockingTool {
    started: tokio::sync::watch::Sender<bool>,
    release: tokio::sync::watch::Receiver<bool>,
}
#[async_trait::async_trait]
impl AgentTool for BlockingTool {
    fn name(&self) -> &str {
        "block_tool"
    }
    fn label(&self) -> &str {
        "Block"
    }
    fn description(&self) -> &str {
        "Blocks until released."
    }
    async fn execute(
        &self,
        _id: &str,
        _args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let _ = self.started.send(true);
        let mut rx = self.release.clone();
        let _ = rx.wait_for(|v| *v).await;
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text { text: "ok".into() }],
            details: serde_json::Value::Null,
            usage: None,
            added_tool_names: vec![],
            terminate: false,
        })
    }
}

/// Stream that requests a tool call on the first turn, then a plain stop.
struct OneToolStream {
    calls: Mutex<usize>,
}
#[async_trait::async_trait]
impl StreamFn for OneToolStream {
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
                        id: "tool-1".into(),
                        name: "block_tool".into(),
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
async fn pending_tool_calls_projection_tracks_in_flight_call() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let (release_tx, release_rx) = tokio::sync::watch::channel(false);
    let mut builder = TestLoopBuilder::new(Arc::new(OneToolStream {
        calls: Mutex::new(0),
    }));
    builder = builder.tool(Arc::new(BlockingTool {
        started: started_tx,
        release: release_rx,
    }));
    let test = builder.build();

    let run = {
        let actor = test.actor.clone();
        tokio::spawn(async move {
            actor
                .prompt(
                    vec![AgentMessage::User(UserMessage {
                        content: vec![UserContent::Text { text: "go".into() }],
                        timestamp: 1,
                    })],
                    AgentContext::default(),
                )
                .await
        })
    };
    // Wait for the tool's owned execution boundary, then inspect the state
    // projection. This avoids timing-dependent polling without sleeps.
    while !*started_rx.borrow() {
        let _ = started_rx.changed().await;
    }
    assert!(test
        .state
        .snapshot()
        .pending_tool_calls
        .contains(&"tool-1".to_string()));

    let _ = release_tx.send(true);
    run.await.expect("run task").expect("run completes");
    test.state.sync().await;
    let after = test.state.snapshot();
    assert!(
        after.pending_tool_calls.is_empty(),
        "pending_tool_calls should clear after the tool ends"
    );
}
