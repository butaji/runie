//! Regression: MiniMax-style response with inline <think> tags must render
//! a visible assistant message.

use crate::{run_agent_turn, AgentCommand};
use futures::Stream;
use runie_core::message::{ChatMessage, Role};
use runie_core::provider::Provider;
use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_testing::allow_all_gate;
use std::pin::Pin;
use std::sync::Arc;
use parking_lot::Mutex;

struct MinimaxLikeProvider;

impl Provider for MinimaxLikeProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        Box::pin(futures::stream::iter(vec![
            Ok(ProviderEvent::TextStart { id: "text".into() }),
            Ok(ProviderEvent::TextDelta(
                "<think>\nThe user is asking".into(),
            )),
            Ok(ProviderEvent::ThinkingStart {
                id: "reasoning".into(),
            }),
            Ok(ProviderEvent::ThinkingDelta("The user is asking".into())),
            Ok(ProviderEvent::TextDelta(
                " me to say hi. I will respond with a friendly greeting.".into(),
            )),
            Ok(ProviderEvent::ThinkingDelta(
                " me to say hi. I will respond with a friendly greeting.".into(),
            )),
            Ok(ProviderEvent::TextDelta(
                "\n</think>\n\nHi there! How can I help you today?".into(),
            )),
            Ok(ProviderEvent::TextEnd { id: "text".into() }),
            Ok(ProviderEvent::ThinkingEnd {
                id: "reasoning".into(),
            }),
            Ok(ProviderEvent::Finish {
                reason: StopReason::Stop,
            }),
        ]))
    }

    fn generate_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        _tools: Vec<serde_json::Value>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        self.generate(messages)
    }
}

#[tokio::test]
async fn minimax_inline_think_renders_visible_response() {
    let provider = MinimaxLikeProvider;
    let cmd = AgentCommand {
        content: "say hi".to_string(),
        id: "req.0".to_string(),
        provider: "minimax".to_string(),
        model: "MiniMax-M3".to_string(),
        thinking_level: runie_core::model::ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: crate::truncate::TruncationPolicy::default(),
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();
    run_agent_turn(
        &provider,
        &cmd,
        Arc::new(Mutex::new(move |evt| {
            events_clone.lock().push(evt)
        })),
        5,
        allow_all_gate(),
    )
    .await
    .unwrap();

    let mut state = runie_core::AppState::default();
    let config = runie_core::config::Config::default();
    state.apply_config(&config);
    for evt in events.lock().drain(..) {
        state.update(evt);
    }

    let assistants: Vec<String> = state
        .session
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .map(|m| m.content())
        .collect();
    assert!(
        assistants.iter().any(|c| c.contains("Hi there!")),
        "visible answer should appear in assistant messages, got {:?}",
        assistants
    );
    assert!(
        !assistants.iter().any(|c| c.contains("<think>")),
        "assistant messages should not contain raw <think> tags, got {:?}",
        assistants
    );
}
