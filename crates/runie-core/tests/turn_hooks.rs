//! prepare-next-turn / stop-after-turn hooks parity (p07, pi agent-loop.ts:232,247).

#![allow(
    clippy::too_many_lines,
    reason = "hook parity keeps context setup and callback assertions together"
)]

mod common;

use std::sync::Arc;

use futures::stream;
use parking_lot::Mutex;
use runie_core::hooks::{ShouldStopAfterTurnContext, TurnHooks, TurnUpdate};
use runie_core::provider::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};
use runie_core::types::{
    AgentContext, AgentMessage, AssistantMessageEvent, Model, SimpleStreamOptions, StopReason,
    ToolCall, Usage, UserContent, UserMessage, WireMessage,
};

use common::{echo_tool, TestLoopBuilder};

/// Stream that records the model id given to each call. Turn 1 requests a
/// tool call (`tool_use`); later turns return a plain `stop` so the loop
/// auto-continues to a second turn.
struct RecordingStream {
    calls: Mutex<usize>,
    models: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl StreamFn for RecordingStream {
    async fn stream(
        &self,
        model: &Model,
        _context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        self.models.lock().push(model.id.clone());
        let mut n = self.calls.lock();
        *n += 1;
        let events = if *n == 1 {
            vec![
                AssistantMessageEvent::ToolCallDelta {
                    index: 0,
                    partial: ToolCall {
                        id: "c1".into(),
                        name: "echo".into(),
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
            vec![
                AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: "hi".into(),
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

fn user() -> Vec<AgentMessage> {
    vec![AgentMessage::User(UserMessage {
        content: vec![UserContent::Text { text: "go".into() }],
        timestamp: 1,
    })]
}

#[tokio::test]
async fn prepare_next_turn_swaps_model_for_next_turn() {
    let stream = Arc::new(RecordingStream {
        calls: Mutex::new(0),
        models: Mutex::new(Vec::new()),
    });
    let hooks = TurnHooks {
        prepare_next_turn: Some(Arc::new(|_ctx: ShouldStopAfterTurnContext| {
            Some(TurnUpdate {
                model: Some(Model {
                    id: "swapped".into(),
                    ..Default::default()
                }),
                ..Default::default()
            })
        })),
        should_stop_after_turn: None,
    };
    let mut builder = TestLoopBuilder::new(stream.clone());
    builder = builder.tool(echo_tool()).turn_hooks(hooks);
    let test = builder.build();
    test.state
        .set_model(Model {
            id: "base".into(),
            ..Default::default()
        })
        .await;

    test.actor
        .prompt(user(), AgentContext::default())
        .await
        .unwrap();

    let models = stream.models.lock();
    assert_eq!(
        models.len(),
        2,
        "two provider calls (tool_use auto-continue)"
    );
    assert_eq!(models[0], "base", "first turn uses the original model");
    assert_eq!(models[1], "swapped", "second turn uses the prepared model");
}

#[tokio::test]
async fn should_stop_after_turn_ends_the_agent_early() {
    let stream = Arc::new(RecordingStream {
        calls: Mutex::new(0),
        models: Mutex::new(Vec::new()),
    });
    let hook_message_counts = Arc::new(Mutex::new(Vec::new()));
    let hook_message_counts_for_hook = hook_message_counts.clone();
    let hooks = TurnHooks {
        should_stop_after_turn: Some(Arc::new(move |ctx: ShouldStopAfterTurnContext| {
            hook_message_counts_for_hook
                .lock()
                .push(ctx.context.messages.len());
            true
        })),
        prepare_next_turn: None,
    };
    let mut builder = TestLoopBuilder::new(stream.clone());
    builder = builder.tool(echo_tool()).turn_hooks(hooks);
    let test = builder.build();

    let out = test
        .actor
        .prompt(user(), AgentContext::default())
        .await
        .unwrap();

    // The tool call turn ran (with its tool result), but should_stop=true
    // prevents the auto-continue follow-up turn.
    let assistants = out
        .iter()
        .filter(|m| matches!(m, AgentMessage::Assistant(_)))
        .count();
    assert_eq!(assistants, 1, "follow-up turn suppressed by should_stop");
    assert!(
        out.iter().any(|m| matches!(m, AgentMessage::ToolResult(_))),
        "tool result should still be present from the first turn"
    );
    // Only one provider call happened.
    assert_eq!(stream.models.lock().len(), 1);
    assert_eq!(hook_message_counts.lock().as_slice(), &[3]);
}

/// Stream that records the user texts it receives in the context (the LLM
/// wire context after transformContext).
struct RecordingCtxStream {
    texts: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl StreamFn for RecordingCtxStream {
    async fn stream(
        &self,
        _model: &Model,
        context: &AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        for m in &context.messages {
            if let AgentMessage::User(u) = m {
                for c in &u.content {
                    if let UserContent::Text { text } = c {
                        self.texts.lock().push(text.clone());
                    }
                }
            }
        }
        Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            message: None,
        }])))
    }
}

#[tokio::test]
async fn transform_context_filters_messages_before_llm() {
    let stream = Arc::new(RecordingCtxStream {
        texts: Mutex::new(Vec::new()),
    });
    // Drop any user message whose text contains "secret".
    let transform = |messages: Vec<AgentMessage>| {
        let filtered: Vec<_> = messages
            .into_iter()
            .filter(|m| {
                !matches!(m, AgentMessage::User(u)
                    if u.content.iter().any(|c| matches!(c, UserContent::Text { text } if text.contains("secret"))))
            })
            .collect();
        Box::pin(async move { filtered }) as futures::future::BoxFuture<'static, Vec<AgentMessage>>
    };
    let mut builder = TestLoopBuilder::new(stream.clone());
    builder = builder.transform_context(transform);
    let test = builder.build();

    let prompt = vec![
        AgentMessage::User(UserMessage {
            content: vec![UserContent::Text {
                text: "visible".into(),
            }],
            timestamp: 1,
        }),
        AgentMessage::User(UserMessage {
            content: vec![UserContent::Text {
                text: "secret payload".into(),
            }],
            timestamp: 2,
        }),
    ];
    test.actor
        .prompt(prompt, AgentContext::default())
        .await
        .unwrap();

    let texts = stream.texts.lock();
    assert!(
        texts.contains(&"visible".to_string()),
        "visible message should reach the provider"
    );
    assert!(
        !texts.iter().any(|t| t.contains("secret")),
        "secret message should be filtered out by transformContext"
    );
}

#[tokio::test]
async fn supplied_context_reaches_first_provider_request() {
    let stream = Arc::new(RecordingCtxStream {
        texts: Mutex::new(Vec::new()),
    });
    let test = TestLoopBuilder::new(stream.clone()).build();
    let mut context = AgentContext::default();
    context.messages.push(AgentMessage::User(UserMessage {
        content: vec![UserContent::Text {
            text: "prior context".into(),
        }],
        timestamp: 1,
    }));

    test.actor
        .prompt(
            vec![AgentMessage::User(UserMessage {
                content: vec![UserContent::Text {
                    text: "new prompt".into(),
                }],
                timestamp: 2,
            })],
            context,
        )
        .await
        .expect("prompt completes");

    assert_eq!(
        stream.texts.lock().as_slice(),
        ["prior context", "new prompt"]
    );
}

#[tokio::test]
async fn convert_to_llm_replaces_wire_messages_after_transform() {
    let stream = Arc::new(RecordingCtxStream {
        texts: Mutex::new(Vec::new()),
    });
    let convert = Arc::new(|messages: Vec<AgentMessage>| {
        Box::pin(async move {
            messages
                .into_iter()
                .filter_map(|message| match message {
                    AgentMessage::User(user) => Some(WireMessage::User {
                        content: vec![UserContent::Text {
                            text: "converted".into(),
                        }],
                        timestamp: user.timestamp,
                    }),
                    _ => None,
                })
                .collect()
        }) as futures::future::BoxFuture<'static, Vec<WireMessage>>
    });
    let test = TestLoopBuilder::new(stream.clone())
        .convert_to_llm(convert)
        .build();
    test.actor
        .prompt(user(), AgentContext::default())
        .await
        .unwrap();

    assert_eq!(&*stream.texts.lock(), &["converted".to_string()]);
}
