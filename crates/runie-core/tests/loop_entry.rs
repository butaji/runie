//! Loop entry-point parity: busy guard + continue() validation (p06).

mod common;

use std::sync::Arc;

use futures::stream;
use futures::StreamExt;
use parking_lot::Mutex;
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AssistantMessageEvent, Model, SimpleStreamOptions, StopReason,
    Usage, UserContent, UserMessage,
};

use common::{MockStreamFn, TestLoopBuilder};

/// StreamFn that signals `started` on its first poll, yields text, then
/// blocks until `release` changes, then finishes with `Done{stop}`. Used to
/// keep a run in flight deterministically.
struct BlockingStream {
    started: tokio::sync::watch::Sender<bool>,
    release: Mutex<Option<tokio::sync::watch::Receiver<bool>>>,
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
            AssistantMessageEvent::TextDelta { delta: "x".into() }
        });
        let tail = futures::stream::once(async move {
            if let Some(mut rx) = release {
                let _ = rx.changed().await;
            }
            AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
            }
        });
        Ok(Box::pin(head.chain(tail)))
    }
}

fn user(text: &str, ts: i64) -> AgentMessage {
    AgentMessage::User(UserMessage {
        content: vec![UserContent::Text {
            text: text.into(),
        }],
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