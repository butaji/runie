//! Unit tests for `ProviderActor`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::actors::config::ConfigActor;
use crate::actors::provider::{BuiltProvider, ProviderActor, ProviderActorHandle, ProviderFactory};
use crate::bus::EventBus;
use crate::config::Config;
use crate::event::Event;
use crate::llm_event::LLMEvent;
use crate::message::ChatMessage;
use crate::provider::{Provider, ProviderError};

struct DummyProvider;

impl Provider for DummyProvider {
    fn generate(
        &self,
        _messages: Vec<ChatMessage>,
    ) -> Pin<Box<dyn futures::Stream<Item = anyhow::Result<LLMEvent>> + Send + '_>> {
        Box::pin(futures::stream::empty())
    }
}

struct MockFactory {
    build_result: std::sync::Mutex<Option<Result<BuiltProvider, ProviderError>>>,
    validate_result: std::sync::Mutex<Option<Result<Vec<String>, String>>>,
    credentials: Option<(String, String)>,
}

impl MockFactory {
    fn ok(provider: Box<dyn Provider>, key: &str, model: &str) -> Self {
        Self {
            build_result: std::sync::Mutex::new(Some(Ok(BuiltProvider {
                provider,
                key: key.into(),
                model: model.into(),
            }))),
            validate_result: std::sync::Mutex::new(None),
            credentials: Some(("http://localhost".into(), "sk-test".into())),
        }
    }

    fn err(error: ProviderError) -> Self {
        Self {
            build_result: std::sync::Mutex::new(Some(Err(error))),
            validate_result: std::sync::Mutex::new(None),
            credentials: None,
        }
    }

    fn validate_ok(models: Vec<String>) -> Self {
        Self {
            build_result: std::sync::Mutex::new(None),
            validate_result: std::sync::Mutex::new(Some(Ok(models))),
            credentials: Some(("http://localhost".into(), "sk-test".into())),
        }
    }
}

impl ProviderFactory for MockFactory {
    fn build(
        &self,
        _provider: &str,
        _model: &str,
        _config: &Config,
    ) -> Result<BuiltProvider, ProviderError> {
        self.build_result
            .lock()
            .unwrap()
            .take()
            .expect("mock build result not configured")
    }

    fn validate_key(
        &self,
        _base_url: &str,
        _api_key: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<String>>> + Send + '_>> {
        let result = self.validate_result.lock().unwrap().take();
        let result = result.expect("mock validate result not configured");
        Box::pin(async move { result.map_err(anyhow::Error::msg) })
    }

    fn resolve_credentials(&self, _provider: &str, _config: &Config) -> (String, String) {
        self.credentials.clone().unwrap_or_default()
    }
}

fn spawn_actor(
    factory: Arc<dyn ProviderFactory>,
) -> (
    ProviderActorHandle,
    crate::actor::ActorHandle,
    crate::actor::ActorHandle,
) {
    let bus = EventBus::<Event>::new(1);
    let (config_handle, config_actor) = ConfigActor::spawn(bus.clone(), None);
    let (provider_handle, provider_actor) = ProviderActor::spawn(bus, config_handle, factory);
    (provider_handle, provider_actor, config_actor)
}

#[tokio::test]
async fn provider_actor_builds_mock_provider() {
    let factory = Arc::new(MockFactory::ok(Box::new(DummyProvider), "mock", "echo"));
    let (handle, _provider_actor, _config_actor) = spawn_actor(factory);

    let built = handle.build("mock".into(), "echo".into()).await.unwrap();

    assert_eq!(built.key, "mock");
    assert_eq!(built.model, "echo");
}

#[tokio::test]
async fn provider_actor_rejects_unknown_provider() {
    let factory = Arc::new(MockFactory::err(ProviderError::UnknownProvider(
        "ghost".into(),
    )));
    let (handle, _provider_actor, _config_actor) = spawn_actor(factory);

    let err = handle.build("ghost".into(), "x".into()).await.unwrap_err();

    assert_eq!(err, ProviderError::UnknownProvider("ghost".into()));
}

#[tokio::test]
async fn provider_actor_validates_key() {
    let factory = Arc::new(MockFactory::validate_ok(vec!["model-a".into()]));
    let (handle, _provider_actor, _config_actor) = spawn_actor(factory);

    let models = handle
        .validate_key("openai".into(), "sk-test".into())
        .await
        .unwrap();

    assert_eq!(models, vec!["model-a"]);
}

#[tokio::test]
async fn provider_actor_lists_models() {
    let factory = Arc::new(MockFactory::validate_ok(vec!["m1".into(), "m2".into()]));
    let (handle, _provider_actor, _config_actor) = spawn_actor(factory);

    let models = handle.list_models("openai".into()).await.unwrap();

    assert_eq!(models, vec!["m1", "m2"]);
}

#[tokio::test]
async fn provider_actor_list_models_fails_without_key() {
    let factory = Arc::new(MockFactory {
        build_result: std::sync::Mutex::new(None),
        validate_result: std::sync::Mutex::new(Some(Ok(Vec::new()))),
        credentials: Some(("http://localhost".into(), "".into())),
    });
    let (handle, _provider_actor, _config_actor) = spawn_actor(factory);

    let err = handle.list_models("openai".into()).await.unwrap_err();

    assert!(err.to_string().contains("API key"));
}
