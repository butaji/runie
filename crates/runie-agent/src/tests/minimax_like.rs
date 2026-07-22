//! Regression: MiniMax-style response with inline <think> tags must render
//! a visible assistant message.

use crate::{agent_command_builder::agent_cmd, run_agent_turn};
use futures::Stream;
use runie_core::message::{ChatMessage, Role};
use runie_core::provider::Provider;
use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_testing::{allow_all_gate, capture_events};
use std::pin::Pin;

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
            Ok(ProviderEvent::ThinkingStart { id: "reasoning".into() }),
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
            Ok(ProviderEvent::ThinkingEnd { id: "reasoning".into() }),
            Ok(ProviderEvent::Finish { reason: StopReason::Stop }),
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
    let cmd = agent_cmd("say hi")
        .provider("minimax")
        .model("MiniMax-M3")
        .build();
    let (events, emit) = capture_events();
    run_agent_turn(&provider, &cmd, emit, 5, allow_all_gate())
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
