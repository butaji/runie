//! Loop entry-point parity: busy guard + continue() validation (p06).

mod common;

use std::sync::Arc;

use futures::stream;
use futures::StreamExt;
use parking_lot::Mutex;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AssistantMessage, AssistantMessageEvent, Model,
    SimpleStreamOptions, StopReason, ToolCall, Usage, UserContent, UserMessage,
};

use common::{echo_tool, MockStreamFn, TestLoopBuilder};

/// StreamFn that signals `started` on its first poll, yields text, then
/// blocks until `release` changes, then finishes with `Done{stop}`. Used to
/// keep a run in flight deterministically.
struct BlockingStream {
    started: tokio::sync::watch::Sender<bool>,
    release: Mutex<Option<tokio::sync::watch::Receiver<bool>>>,
}

struct SignalCaptureStream {
    seen: tokio::sync::watch::Sender<bool>,
}

#[async_trait::async_trait]
impl StreamFn for SignalCaptureStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let _ = self.seen.send(options.and_then(|o| o.signal).is_some());
        Ok(Box::pin(stream::iter([AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            message: None,
        }])))
    }
}

#[async_trait::async_trait]
impl StreamFn for BlockingStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let started = self.started.clone();
        let release = self.release.lock().take();
        let head = futures::stream::once(async move {
            let _ = started.send(true);
            AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "x".into(),
                partial: AssistantMessage::default(),
            }
        });
        let tail = futures::stream::once(async move {
            if let Some(mut rx) = release {
                let _ = rx.changed().await;
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

fn user(text: &str, ts: i64) -> AgentMessage {
    AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: text.into() }],
        timestamp: ts,
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_prompt_rejected_as_busy() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let (release_tx, release_rx) = tokio::sync::watch::channel(false);
    let stream = Arc::new(BlockingStream {
        started: started_tx,
        release: Mutex::new(Some(release_rx)),
    });
    let test = TestLoopBuilder::new(stream).build();

    // First run holds the loop in flight (stream blocks on the release watch).
    let first = {
        let actor = test.actor.clone();
        let prompt = vec![user("first", 1)];
        tokio::spawn(async move { actor.prompt(prompt, AgentContext::default()).await })
    };
    // Deterministically wait until the first run has started streaming (at
    // that point the busy flag is set), then a second prompt must be Busy.
    while !*started_rx.borrow() {
        let _ = started_rx.changed().await;
    }
    let second = test
        .actor
        .prompt(vec![user("second", 2)], AgentContext::default())
        .await;
    assert!(
        matches!(second, Err(runie_core::r#loop::LoopError::Busy)),
        "expected Busy, got {second:?}"
    );

    // Release the first run; it completes normally.
    let _ = release_tx.send(true);
    let first_out = first.await.expect("first task").expect("first prompt ok");
    assert_eq!(first_out.len(), 2, "user + assistant");
}

#[tokio::test]
async fn prompt_propagates_abort_signal_to_provider_options() {
    let (seen_tx, mut seen_rx) = tokio::sync::watch::channel(false);
    let test = TestLoopBuilder::new(Arc::new(SignalCaptureStream { seen: seen_tx })).build();
    test.actor
        .prompt(vec![user("signal", 1)], AgentContext::default())
        .await
        .expect("prompt completes");
    assert!(*seen_rx.borrow_and_update());
}

#[tokio::test(flavor = "multi_thread")]
async fn abort_interrupts_active_stream_and_marks_partial_assistant() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let (release_tx, release_rx) = tokio::sync::watch::channel(false);
    let stream = Arc::new(BlockingStream {
        started: started_tx,
        release: Mutex::new(Some(release_rx)),
    });
    let test = TestLoopBuilder::new(stream).build();
    let actor = test.actor.clone();
    let run = tokio::spawn(async move {
        actor
            .prompt(vec![user("abort", 1)], AgentContext::default())
            .await
    });
    while !*started_rx.borrow() {
        let _ = started_rx.changed().await;
    }

    test.actor.abort();
    let output = run.await.expect("run task").expect("prompt completes");
    let assistant = output
        .iter()
        .find_map(|message| match message {
            AgentMessage::Assistant(message) => Some(message),
            _ => None,
        })
        .expect("partial assistant is retained");
    assert_eq!(assistant.stop_reason, Some(StopReason::Aborted));
    assert_eq!(
        test.state.snapshot().error_message.as_deref(),
        Some("aborted")
    );
    let _ = release_tx.send(true);
}

#[tokio::test]
async fn continue_run_rejects_empty_context() {
    let test = TestLoopBuilder::new(Arc::new(MockStreamFn::hello())).build();
    let err = test
        .actor
        .continue_run(AgentContext::default())
        .await
        .unwrap_err();
    assert!(
        matches!(err, runie_core::r#loop::LoopError::EmptyContext),
        "expected EmptyContext, got {err:?}"
    );
}

#[tokio::test]
async fn continue_run_rejects_last_assistant() {
    let mut ctx = AgentContext::default();
    ctx.messages.push(user("hi", 1));
    ctx.messages.push(AgentMessage::Assistant(
        runie_core::types::AssistantMessage {
            content: vec![],
            stop_reason: Some(StopReason::Stop),
            model: "m".into(),
            timestamp: 2,
            ..Default::default()
        },
    ));
    let test = TestLoopBuilder::new(Arc::new(MockStreamFn::hello())).build();
    let err = test.actor.continue_run(ctx).await.unwrap_err();
    assert!(
        matches!(err, runie_core::r#loop::LoopError::LastIsAssistant),
        "expected LastIsAssistant, got {err:?}"
    );
}

#[tokio::test]
async fn continue_run_from_assistant_consumes_queued_steering() {
    let mut ctx = AgentContext::default();
    ctx.messages.push(user("hi", 1));
    ctx.messages.push(AgentMessage::Assistant(
        runie_core::types::AssistantMessage {
            content: vec![],
            stop_reason: Some(StopReason::Stop),
            model: "m".into(),
            timestamp: 2,
            ..Default::default()
        },
    ));
    let test = TestLoopBuilder::new(Arc::new(MockStreamFn::hello())).build();
    test.actor.steer(user("queued", 3)).await;

    let out = test
        .actor
        .continue_run(ctx)
        .await
        .expect("queued continuation");
    assert_eq!(out.len(), 2, "queued user + assistant");
    assert!(matches!(out[0], AgentMessage::User(_)));
    assert!(matches!(out[1], AgentMessage::Assistant(_)));
}

#[tokio::test]
async fn continue_run_from_user_produces_new_messages_only() {
    // Context ends with a user message (valid continuation point).
    let mut ctx = AgentContext::default();
    ctx.messages.push(user("hi", 1));
    let test = TestLoopBuilder::new(Arc::new(MockStreamFn::hello())).build();
    let out = test.actor.continue_run(ctx).await.unwrap();
    // Only the new assistant message is returned (not the pre-existing user).
    assert_eq!(out.len(), 1);
    assert!(matches!(out[0], AgentMessage::Assistant(_)));
}

/// Stream that keeps requesting tool calls so the loop would auto-continue
/// forever; turn 2 blocks until released. Used to prove `abort()` stops it.
struct AbortStream {
    calls: Mutex<usize>,
    started: tokio::sync::watch::Sender<bool>,
    release: Mutex<Option<tokio::sync::watch::Receiver<bool>>>,
}

#[async_trait::async_trait]
impl StreamFn for AbortStream {
    async fn stream(
        &self,
        _model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        let n = {
            let mut c = self.calls.lock();
            *c += 1;
            *c
        };
        let release = self.release.lock().take();
        let tool_turn = vec![
            AssistantMessageEvent::ToolCallDelta {
                index: 0,
                delta: "{}".into(),
                partial: AssistantMessage::with_tool_call(ToolCall {
                    id: "c1".into(),
                    name: "echo".into(),
                    arguments: serde_json::json!({}),
                    thought_signature: None,
                }),
            },
            AssistantMessageEvent::Done {
                stop_reason: StopReason::ToolUse,
                usage: Usage::default(),
                message: None,
            },
        ];
        if n == 1 {
            let _ = self.started.send(true);
            return Ok(Box::pin(stream::iter(tool_turn)));
        }
        if n == 2 {
            if let Some(mut rx) = release {
                let _ = rx.wait_for(|v| *v).await;
            }
        }
        Ok(Box::pin(stream::iter(tool_turn)))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn abort_stops_a_continuously_tool_using_run() {
    let (started_tx, mut started_rx) = tokio::sync::watch::channel(false);
    let (release_tx, release_rx) = tokio::sync::watch::channel(false);
    let stream = Arc::new(AbortStream {
        calls: Mutex::new(0),
        started: started_tx,
        release: Mutex::new(Some(release_rx)),
    });
    let mut builder = TestLoopBuilder::new(stream);
    builder = builder.tool(echo_tool());
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
    // Wait until turn 1 (tool call) has begun, then abort and release turn 2.
    while !*started_rx.borrow() {
        let _ = started_rx.changed().await;
    }
    test.actor.abort();
    let _ = release_tx.send(true);

    let outcome = run.await.expect("run task").expect("run completes");
    // The state should record the abort.
    let snap = test.state.snapshot();
    assert_eq!(
        snap.error_message.clone(),
        Some("aborted".to_string()),
        "abort should set the error message"
    );
    // The run still returned (terminated cleanly), not busy.
    assert!(!outcome.is_empty());
}
