//! Tests for AgentActor.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use tokio::time::timeout;

use runie_core::actors::permission::RactorPermissionActor;
use runie_core::actors::provider::{BuiltProvider, ProviderFactory};
use runie_core::bus::EventBus;
use runie_core::config::Config;
use runie_core::event::Event;
use runie_core::message::ChatMessage;
use runie_core::model::ThinkingLevel;
use runie_core::provider::{Provider, ProviderError};
use runie_core::provider_event::{ProviderEvent, StopReason};
use crate::truncate::TruncationPolicy;

use super::{spawn_ractor_agent, AgentMsg};

/// A provider that immediately returns one text chunk then finishes.
struct SimpleTextProvider;

impl Provider for SimpleTextProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn Stream<Item = anyhow::Result<ProviderEvent>> + Send + '_>> {
        let stream = futures::stream::iter([
            Ok(ProviderEvent::TextDelta("hello".into())),
            Ok(ProviderEvent::Usage { input_tokens: 1, output_tokens: 1 }),
            Ok(ProviderEvent::Finish { reason: StopReason::Stop }),
        ]);
        Box::pin(stream)
    }
}

struct TestFactory;

#[async_trait::async_trait]
impl ProviderFactory for TestFactory {
    fn build(
        &self,
        _provider: &str,
        _model: &str,
        _config: &Config,
    ) -> Result<BuiltProvider, ProviderError> {
        Ok(BuiltProvider::new(
            Box::new(SimpleTextProvider),
            "test".into(),
            "test-model".into(),
        ))
    }

    async fn validate_key(&self, _: &str, _: &str) -> anyhow::Result<Vec<String>> {
        Ok(vec!["test-model".into()])
    }

    fn resolve_credentials(&self, _: &str, _: &Config) -> (String, String) {
        ("http://localhost".into(), "sk-test".into())
    }
}

#[tokio::test]
async fn agent_actor_accepts_second_turn_after_first_completes() {
    let bus = EventBus::<Event>::new(10);

    let (config_handle, _, _) =
        runie_core::actors::RactorConfigActor::spawn_default(bus.clone())
            .await
            .unwrap();
    let (provider_handle, _, _) =
        runie_core::actors::provider::RactorProviderActor::spawn(
            bus.clone(),
            config_handle,
            Arc::new(TestFactory),
        )
        .await
        .unwrap();

    let (permission_handle, _, _) =
        RactorPermissionActor::spawn_for_testing(bus.clone()).await.unwrap();

    let (agent_handle, _, _) =
        spawn_ractor_agent(bus.clone(), provider_handle, permission_handle)
            .await
            .unwrap();

    let mut sub = bus.subscribe();

    // --- First turn ---
    let cmd1 = crate::AgentCommand {
        content: "hello".into(),
        id: "turn-1".into(),
        provider: "test".into(),
        model: "test-model".into(),
        thinking_level: ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: TruncationPolicy::default(),
        cancellation_token: tokio_util::sync::CancellationToken::new(),
    };
    let _ = agent_handle.send_message(AgentMsg::Run { command: cmd1 });

    // Wait for first turn to complete.
    timeout(Duration::from_secs(5), async {
        while let Ok(evt) = sub.recv().await {
            if matches!(evt, Event::Done { id } if id == "turn-1") {
                return;
            }
        }
    })
    .await
    .unwrap();

    // Give TurnComplete message time to be processed by the actor.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // --- Second turn ---
    let cmd2 = crate::AgentCommand {
        content: "hello again".into(),
        id: "turn-2".into(),
        provider: "test".into(),
        model: "test-model".into(),
        thinking_level: ThinkingLevel::Off,
        read_only: false,
        skills_context: String::new(),
        system_prompt: String::new(),
        truncation: TruncationPolicy::default(),
        cancellation_token: tokio_util::sync::CancellationToken::new(),
    };
    let _ = agent_handle.send_message(AgentMsg::Run { command: cmd2 });

    // Wait for second turn to complete.
    let mut turn2_done = false;
    let mut turn2_error = false;
    timeout(Duration::from_secs(5), async {
        while let Ok(evt) = sub.recv().await {
            if matches!(&evt, Event::Done { id } if id == "turn-2") {
                turn2_done = true;
                break;
            }
            if matches!(&evt, Event::Error { id, .. } if id == "turn-2") {
                turn2_error = true;
                break;
            }
        }
    })
    .await
    .unwrap();

    assert!(
        turn2_done,
        "second turn must complete; got error={turn2_error}"
    );
    assert!(
        !turn2_error,
        "second turn must not emit an Error event"
    );
}
