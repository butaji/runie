#![allow(clippy::all)]
//! Tests for runie-provider

use crate::{
    build_provider_from_boxed, build_provider_with_config, BuiltProviderFactory, MockProvider,
    MockStreamingProvider, Provider, ProviderError, BuiltProvider,
};
use futures::StreamExt;
use runie_core::actors::{
    provider::RactorProviderActor as ProviderActor, RactorConfigActor as ConfigActor,
};
use runie_core::bus::EventBus;
use runie_core::config::Config;
use runie_core::event::Event;
use runie_core::message::ChatMessage;
use runie_core::provider_event::ProviderEvent;
/// Helper to run a test closure with protected env vars.
fn with_env_lock<F, T>(var: &str, value: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = runie_testing::ENV_LOCK.lock().unwrap();
    std::env::set_var(var, value);
    let result = f();
    std::env::remove_var(var);
    result
}

/// Collect all text deltas from a stream.
async fn collect_text(
    stream: impl futures::Stream<Item = anyhow::Result<ProviderEvent>> + Unpin,
) -> Vec<String> {
    let mut texts = Vec::new();
    futures::pin_mut!(stream);
    while let Some(result) = stream.next().await {
        if let Ok(ProviderEvent::TextDelta(t)) = result {
            texts.push(t);
        }
    }
    texts
}

// ============================================================================
// MockProvider tests
// ============================================================================

#[tokio::test]
async fn test_mock_provider_generates_chunks() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("Hello World".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 2);
    assert_eq!(texts[0], "Hello ");
    assert_eq!(texts[1], "World ");
}

#[tokio::test]
async fn test_mock_provider_empty_input() {
    let provider = MockProvider::default();
    let messages = vec![];
    let texts = collect_text(provider.generate(messages)).await;
    assert!(texts.is_empty());
}

#[tokio::test]
async fn test_mock_provider_single_word() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("Hello".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(!texts.is_empty());
    assert_eq!(texts[0], "Hello ");
}

#[tokio::test]
async fn test_mock_provider_triggers_list_files() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("list files".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 2);
    assert!(texts[1].contains("TOOL:list_dir"));
}

#[tokio::test]
async fn test_mock_provider_triggers_read_file() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("read the readme".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 2);
    assert!(texts[1].contains("TOOL:read_file"));
}

#[tokio::test]
async fn test_mock_provider_triggers_write_file() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("write something".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 2);
    assert!(texts[1].contains("TOOL:write_file"));
}

#[tokio::test]
async fn test_mock_provider_triggers_bash() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("run command".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 2);
    assert!(texts[1].contains("TOOL:bash"));
}

#[tokio::test]
async fn test_mock_provider_triggers_done() {
    // "Done" response is triggered after a ToolResult, not from simple "hello" input.
    // Verify the simple input returns word chunks.
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("hello".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(!texts.is_empty());
    assert!(texts[0].contains("hello "));
}

#[tokio::test]
async fn test_mock_provider_follows_up_after_tool_result() {
    let provider = MockProvider::default();
    let messages = vec![
        ChatMessage::user("list files".to_string()),
        ChatMessage::assistant("TOOL:list_dir:.".to_string()),
        ChatMessage::tool("file1.txt (file)".to_string()),
    ];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(!texts.is_empty());
    assert!(texts[0].contains("Done"));
}

#[tokio::test]
async fn mock_provider_default_no_delay() {
    let p = MockProvider::default();
    let start = std::time::Instant::now();
    let texts = collect_text(p.generate(vec![ChatMessage::user("test".to_string())])).await;
    assert!(start.elapsed() < std::time::Duration::from_millis(50));
    assert!(!texts.is_empty());
}

#[test]
fn mock_provider_with_delay_configured() {
    let p = MockProvider::with_delay(5, 15);
    assert_eq!(p.delay_ms(), Some((5, 15)));
}

#[tokio::test]
async fn test_built_provider_mock_delay_adds_streaming_delay() {
    // Use env_lock for async test context
    let _guard = runie_testing::ENV_LOCK.lock().unwrap();
    std::env::remove_var("RUNIE_MOCK");
    std::env::set_var("RUNIE_MOCK_DELAY", "1");

    let provider = build_provider_with_config("mock", "echo", &Config::default())
        .expect("mock should build with RUNIE_MOCK_DELAY");
    let start = std::time::Instant::now();
    let texts =
        collect_text(provider.generate(vec![ChatMessage::user("hello world".to_string())])).await;

    std::env::remove_var("RUNIE_MOCK_DELAY");
    assert!(!texts.is_empty());
    // Delay is now 5-10ms, so we check for >= 1ms to verify delay was applied
    assert!(
        start.elapsed() >= std::time::Duration::from_millis(1),
        "RUNIE_MOCK_DELAY should introduce a streaming delay, elapsed: {:?}",
        start.elapsed()
    );
}

#[test]
fn mock_provider_delay_is_deterministic() {
    let p1 = MockProvider::with_seed(10, 100, 123);
    let p2 = MockProvider::with_seed(10, 100, 123);
    let mut delays1 = Vec::new();
    let mut delays2 = Vec::new();
    for _ in 0..5 {
        delays1.push(p1.random_delay());
        delays2.push(p2.random_delay());
    }
    assert_eq!(delays1, delays2);
}

// ============================================================================
// BuiltProvider tests
// ============================================================================

#[test]
fn test_built_provider_unknown_returns_err() {
    let result = build_provider_with_config("bogus-provider-xyz", "gpt-4o", &Config::default());
    match result {
        Err(ProviderError::UnknownProvider(key)) => assert_eq!(key, "bogus-provider-xyz"),
        other => panic!("expected UnknownProvider error, got: {:?}", other),
    }
}

#[test]
fn test_built_provider_missing_api_key_returns_err() {
    // Use env_lock for test isolation
    let _guard = runie_testing::ENV_LOCK.lock().unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let result = build_provider_with_config("openai", "gpt-4o-mini", &Config::default());
    match result {
        Err(ProviderError::MissingApiKey(ref err)) => assert_eq!(err.env_var, "OPENAI_API_KEY"),
        other => panic!("expected MissingApiKey error, got: {:?}", other),
    }
}

#[test]
fn test_built_provider_known_with_key_succeeds() {
    with_env_lock("OPENAI_API_KEY", "test-key-123", || {
        let result = build_provider_with_config("openai", "gpt-4o-mini", &Config::default());
        let provider = result.expect("BuiltProvider should succeed with API key");
        assert_eq!(provider.key(), "openai");
        assert_eq!(provider.model(), "gpt-4o-mini");
    });
}

#[test]
fn test_built_provider_key_and_model_accessors() {
    with_env_lock("OPENAI_API_KEY", "sk-test", || {
        let provider =
            build_provider_with_config("openai", "gpt-4o", &Config::default()).unwrap();
        assert_eq!(provider.key(), "openai");
        assert_eq!(provider.model(), "gpt-4o");
    });
}

#[test]
fn test_provider_trait_is_dyn_compatible() {
    let _: Box<dyn Provider> = Box::new(MockProvider::default());
    with_env_lock("OPENAI_API_KEY", "sk-dyn-test", || {
        let bp = build_provider_with_config("openai", "gpt-4o", &Config::default()).unwrap();
        let _: Box<dyn Provider> = Box::new(bp);
    });
}

#[test]
fn test_is_known_provider() {
    assert!(crate::is_known("openai"));
    assert!(crate::is_known("anthropic"));
    assert!(!crate::is_known("not-a-real-provider"));
}

// ============================================================================
// MockStreamingProvider tests
// ============================================================================

#[tokio::test]
async fn test_mock_streaming_provider_basic() {
    let provider = MockStreamingProvider::new();
    let messages = vec![ChatMessage::user("Hello".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(!texts.is_empty());
    for text in &texts {
        assert!(!text.is_empty());
    }
}

#[tokio::test]
async fn test_mock_streaming_provider_at_rate() {
    let provider = MockStreamingProvider::with_rate(100.0);
    let messages = vec![ChatMessage::user("test".to_string())];
    let start = std::time::Instant::now();
    let texts = collect_text(provider.generate(messages)).await;
    let elapsed = start.elapsed();
    assert!(!texts.is_empty());
    assert!(elapsed < std::time::Duration::from_secs(5));
}

#[tokio::test]
async fn test_mock_streaming_provider_no_delay() {
    let mut provider = MockStreamingProvider::new();
    provider.delay_ms = 0;
    let messages = vec![ChatMessage::user("Hi".to_string())];
    let start = std::time::Instant::now();
    let texts = collect_text(provider.generate(messages)).await;
    assert!(start.elapsed() < std::time::Duration::from_millis(50));
    assert!(!texts.is_empty());
}

#[tokio::test]
async fn test_validate_api_key_times_out_on_hanging_server() {
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Start a wiremock server that delays response beyond the validation timeout.
    let mock_server = MockServer::start().await;

    // Match GET /v1/models and delay response by 5 seconds (exceeds 250ms timeout).
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&mock_server)
        .await;

    let start = std::time::Instant::now();
    let result = crate::validate_api_key_with_timeout(
        &format!("{}/v1", mock_server.uri()),
        "sk-test",
        Duration::from_millis(250),
    )
    .await;

    let elapsed = start.elapsed();
    assert!(
        result.is_err(),
        "server delayed beyond timeout should produce an error: {:?}",
        result
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "should return quickly due to timeout"
    );
}

#[tokio::test]
async fn test_validate_api_key_rejects_unreachable_port() {
    use std::time::Duration;

    let result = crate::validate_api_key_with_timeout(
        "http://127.0.0.1:1/v1",
        "sk-test",
        Duration::from_secs(1),
    )
    .await;

    assert!(result.is_err(), "unreachable port should produce an error");
}

#[tokio::test]
async fn test_validate_api_key_parses_minimax_models_and_trims_key() {
    use std::time::Duration;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Verify the Authorization header is sent with the trimmed key.
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer sk-test"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "MiniMax-M3"},
                    {"id": "MiniMax-M2.7"}
                ]
            })),
        )
        .mount(&mock_server)
        .await;

    let models = crate::validate_api_key_with_timeout(
        &format!("{}/v1", mock_server.uri()),
        "  sk-test\n ",
        Duration::from_secs(2),
    )
    .await
    .expect("valid MiniMax-style response should succeed");

    assert_eq!(models, vec!["MiniMax-M3", "MiniMax-M2.7"]);
}

#[test]
fn test_built_provider_trims_whitespace_api_key_from_env() {
    with_env_lock("OPENAI_API_KEY", "  sk-with-space\n ", || {
        let provider = build_provider_with_config("openai", "gpt-4o-mini", &Config::default())
            .expect("trimmed key should build");
        assert_eq!(provider.key(), "openai");
    });
}

#[tokio::test]
async fn test_mock_streaming_provider_accumulates_content() {
    let provider = MockStreamingProvider::new();
    let messages = vec![ChatMessage::user("test".to_string())];
    let texts = collect_text(provider.generate(messages)).await;
    let full_content: String = texts.into_iter().collect();

    assert!(full_content.contains("test"));
    assert!(full_content.len() > 10);
}

// ============================================================================
// ProviderActor integration tests using the real BuiltProviderFactory
// ============================================================================

#[tokio::test]
async fn provider_actor_builds_mock_provider_with_runie_mock() {
    // Use env_lock for test isolation
    let _guard = runie_testing::ENV_LOCK.lock().unwrap();
    std::env::set_var("RUNIE_MOCK", "1");

    let bus = EventBus::<Event>::new(1);
    let (config_handle, _config_actor, _join) = ConfigActor::spawn_default(bus.clone()).await.unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(bus, config_handle, std::sync::Arc::new(BuiltProviderFactory::new()))
            .await
            .unwrap();

    let built = provider_handle
        .build("mock".into(), "echo".into())
        .await
        .expect("mock provider should build with RUNIE_MOCK");

    assert_eq!(built.key, "mock");
    assert_eq!(built.model, "echo");

    std::env::remove_var("RUNIE_MOCK");
}

#[tokio::test]
async fn provider_actor_rejects_unknown_provider_real_factory() {
    let bus = EventBus::<Event>::new(1);
    let (config_handle, _config_actor, _join) = ConfigActor::spawn_default(bus.clone()).await.unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(bus, config_handle, std::sync::Arc::new(BuiltProviderFactory::new()))
            .await
            .unwrap();

    let err = provider_handle
        .build("ghost-provider".into(), "x".into())
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::UnknownProvider(ref k) if k == "ghost-provider"));
}

#[tokio::test]
async fn provider_actor_validates_key_against_mock_server() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer sk-test"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": "model-a"},
                    {"id": "model-b"}
                ]
            })),
        )
        .mount(&mock_server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let config = format!(
        r#"[model_providers.test]
base_url = "{base}/v1"
api_key = "sk-test"
"#,
        base = mock_server.uri()
    );
    std::fs::write(&config_path, config).unwrap();

    let bus = EventBus::<Event>::new(1);
    let (config_handle, _config_actor, _) =
        ConfigActor::spawn(bus.clone(), Some(config_path), None).await.unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(bus, config_handle, std::sync::Arc::new(BuiltProviderFactory::new()))
            .await
            .unwrap();

    let models = provider_handle
        .validate_key("test".into(), "sk-test".into())
        .await
        .expect("validation should parse mock server response");

    assert_eq!(models, vec!["model-a", "model-b"]);
}
