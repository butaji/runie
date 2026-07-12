#![allow(clippy::all)]
//! Tests for runie-provider

use proptest::prelude::*;

use crate::{
    build_provider_with_config, BuiltProviderFactory, MockProvider, MockProviderBuilder,
    MockStreamingProvider, Provider, ProviderError,
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
async fn test_mock_provider_echoes_input() {
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("Hello World".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert_eq!(texts.concat(), "Hello World\n");
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

    assert_eq!(texts.concat(), "Hello\n");
}

#[tokio::test]
async fn test_mock_provider_triggers_list_files() {
    // Use explicit fixture for reliable testing
    let provider = MockProviderBuilder::new().list_dir().build();
    let messages = vec![ChatMessage::user("list files".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 1);
    assert!(texts[0].contains("list the files"));
}

#[tokio::test]
async fn test_mock_provider_triggers_read_file() {
    // Use explicit fixture for reliable testing
    let provider = MockProviderBuilder::new().read_file().build();
    let messages = vec![ChatMessage::user("read the readme".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 1);
    assert!(texts[0].contains("read that file"));
}

#[tokio::test]
async fn test_mock_provider_triggers_write_file() {
    // Use explicit fixture for reliable testing
    let provider = MockProviderBuilder::new().write_file().build();
    let messages = vec![ChatMessage::user("write something".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 1);
    assert!(texts[0].contains("create that file"));
}

#[tokio::test]
async fn test_mock_provider_triggers_bash() {
    // Use explicit fixture for reliable testing
    let provider = MockProviderBuilder::new().bash().build();
    let messages = vec![ChatMessage::user("run command".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert!(texts.len() >= 1);
    assert!(texts[0].contains("run that command"));
}

#[tokio::test]
async fn test_mock_provider_triggers_done() {
    // "Done" response is triggered after a ToolResult, not from simple "hello" input.
    // Verify the simple input is echoed back exactly.
    let provider = MockProvider::default();
    let messages = vec![ChatMessage::user("hello".to_string())];
    let texts = collect_text(provider.generate(messages)).await;

    assert_eq!(texts.concat(), "hello\n");
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
    // Use a generous timeout to avoid flakiness on busy systems
    assert!(start.elapsed() < std::time::Duration::from_millis(200));
    assert!(!texts.is_empty());
}

fn run_mock_echo_sync(input: &str) -> String {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    rt.block_on(async {
        let provider = MockProviderBuilder::new().build();
        let messages = vec![ChatMessage::user(input.to_string())];
        collect_text(provider.generate(messages)).await.concat()
    })
}

fn echo_safe_string() -> impl Strategy<Value = String> {
    // Alphabet chosen so generated strings never contain fixture keywords
    // ("read", "write", "edit", "list", "files", "run", "cmd", etc.).
    let chars = prop_oneof![
        Just('a'),
        Just('b'),
        Just('A'),
        Just('B'),
        Just('0'),
        Just('1'),
        Just('9'),
        Just(' '),
        Just('\n'),
        Just('\t'),
        Just('!'),
        Just('?'),
        Just(','),
        Just('.'),
        Just('-'),
        Just('+'),
        Just('é'),
        Just('中'),
    ];
    prop::collection::vec(chars, 0..200).prop_map(|v| v.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn mock_provider_echoes_arbitrary_safe_input(input in echo_safe_string()) {
        let output = run_mock_echo_sync(&input);
        // Empty input returns empty, otherwise adds trailing newline
        let expected = if input.is_empty() {
            String::new()
        } else {
            format!("{}\n", input)
        };
        prop_assert_eq!(output, expected);
    }
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
    // Isolate from the developer's real auth.json so the resolver finds no key.
    std::env::set_var("RUNIE_AUTH_FILE", "/dev/null/nonexistent_runie_auth.json");
    let result = build_provider_with_config("openai", "gpt-4o-mini", &Config::default());
    std::env::remove_var("RUNIE_AUTH_FILE");
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
        let provider = build_provider_with_config("openai", "gpt-4o", &Config::default()).unwrap();
        assert_eq!(provider.key(), "openai");
        assert_eq!(provider.model(), "gpt-4o");
    });
}

#[test]
fn test_built_provider_accepts_prefixed_model_name() {
    // Regression: configs that use provider-prefixed model names ("openai/gpt-4o")
    // must build successfully. The original (prefixed) model name is preserved on
    // BuiltProvider so status displays and recent-model history remain consistent.
    with_env_lock("OPENAI_API_KEY", "sk-test", || {
        let provider = build_provider_with_config("openai", "openai/gpt-4o", &Config::default())
            .expect("should build with prefixed model");
        assert_eq!(provider.key(), "openai");
        assert_eq!(provider.model(), "openai/gpt-4o");
    });
}

#[test]
fn test_built_provider_accepts_minimax_prefixed_model_name() {
    // Regression: "minimax/MiniMax-M3" previously failed because validation and
    // provider construction did not strip the provider prefix before registry lookup.
    with_env_lock("MINIMAX_API_KEY", "sk-minimax-test", || {
        let provider =
            build_provider_with_config("minimax", "minimax/MiniMax-M3", &Config::default())
                .expect("should build with minimax prefix");
        assert_eq!(provider.key(), "minimax");
        assert_eq!(provider.model(), "minimax/MiniMax-M3");
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"id": "MiniMax-M3"},
                {"id": "MiniMax-M2.7"}
            ]
        })))
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
    let (config_handle, _config_actor, _join) =
        ConfigActor::spawn_default(bus.clone()).await.unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(
        bus,
        config_handle,
        std::sync::Arc::new(BuiltProviderFactory::new()),
    )
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
async fn provider_actor_builds_mock_provider_with_fixture_model() {
    let _guard = runie_testing::ENV_LOCK.lock().unwrap();
    std::env::set_var("RUNIE_MOCK", "1");

    let bus = EventBus::<Event>::new(1);
    let (config_handle, _config_actor, _join) =
        ConfigActor::spawn_default(bus.clone()).await.unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(
        bus,
        config_handle,
        std::sync::Arc::new(BuiltProviderFactory::new()),
    )
    .await
    .unwrap();

    let built = provider_handle
        .build("mock".into(), "list_dir".into())
        .await
        .expect("mock provider should build with fixture model");

    assert_eq!(built.key, "mock");
    assert_eq!(built.model, "list_dir");

    std::env::remove_var("RUNIE_MOCK");
}

#[tokio::test]
async fn provider_actor_rejects_unknown_provider_real_factory() {
    let bus = EventBus::<Event>::new(1);
    let (config_handle, _config_actor, _join) =
        ConfigActor::spawn_default(bus.clone()).await.unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(
        bus,
        config_handle,
        std::sync::Arc::new(BuiltProviderFactory::new()),
    )
    .await
    .unwrap();

    let err = provider_handle
        .build("ghost-provider".into(), "x".into())
        .await
        .unwrap_err();

    assert!(matches!(err, ProviderError::UnknownProvider(ref k) if k == "ghost-provider"));
}

#[tokio::test]
async fn prefixed_model_name_is_stripped_in_api_request() {
    use futures::StreamExt;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let _guard = runie_testing::ENV_LOCK.lock().unwrap();
    std::env::set_var("MINIMAX_API_KEY", "sk-minimax-test");

    // Raw-TCP mock: capture the request body (for the model-field assertion) and
    // return a *valid* 200 SSE response (Content-Type: text/event-stream). A raw
    // server is used because wiremock cannot reliably emit text/event-stream, and
    // reqwest-eventsource now correctly rejects non-SSE content-types.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_for = captured.clone();
    tokio::spawn(async move {
        let Ok((mut sock, _)) = listener.accept().await else {
            return;
        };
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];
        let header_end;
        loop {
            match sock.read(&mut tmp).await {
                Ok(0) | Err(_) => return,
                Ok(k) => {
                    buf.extend_from_slice(&tmp[..k]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_end = pos + 4;
                        break;
                    }
                    if buf.len() > 64 * 1024 {
                        return;
                    }
                }
            }
        }
        let header_str = String::from_utf8_lossy(&buf[..header_end]);
        let content_len: usize = header_str
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                if k.eq_ignore_ascii_case("content-length") {
                    v.trim().parse().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let mut body = buf[header_end..].to_vec();
        while body.len() < content_len {
            match sock.read(&mut tmp).await {
                Ok(0) | Err(_) => break,
                Ok(k) => body.extend_from_slice(&tmp[..k]),
            }
        }
        body.truncate(content_len);
        *captured_for.lock().unwrap() = body;

        let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
             data: [DONE]\n\n";
        let _ = sock.write_all(resp.as_bytes()).await;
        let _ = sock.flush().await;
    });

    let mut config = Config::default();
    config.provider = Some("minimax".to_string());
    config.models.default = Some("minimax/MiniMax-M3".to_string());
    config.model_providers.insert(
        "minimax".to_string(),
        runie_core::config::ModelProvider {
            provider_type: Some("openai-compatible".to_string()),
            base_url: format!("{base_url}/v1"),
            models: vec!["MiniMax-M3".to_string()],
        },
    );

    let provider = build_provider_with_config("minimax", "minimax/MiniMax-M3", &config)
        .expect("should build with prefixed model");

    // Drive the stream to ensure the HTTP request is made. We don't assert on
    // events here; the goal is to verify the outgoing request body.
    let _events: Vec<_> = provider
        .generate(vec![ChatMessage::user("hello".to_string())])
        .collect()
        .await;

    let body_bytes = captured.lock().unwrap().clone();
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("request body is valid JSON");
    assert_eq!(
        body.get("model").and_then(|v| v.as_str()),
        Some("MiniMax-M3"),
        "API request must use bare model name, got model field: {:?}",
        body.get("model")
    );

    std::env::remove_var("MINIMAX_API_KEY");
}

#[tokio::test]
async fn provider_actor_validates_key_against_mock_server() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", "Bearer sk-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"id": "model-a"},
                {"id": "model-b"}
            ]
        })))
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
        ConfigActor::spawn(bus.clone(), Some(config_path), None)
            .await
            .unwrap();
    let (provider_handle, _provider_actor, _join) = ProviderActor::spawn(
        bus,
        config_handle,
        std::sync::Arc::new(BuiltProviderFactory::new()),
    )
    .await
    .unwrap();

    let models = provider_handle
        .validate_key("test".into(), "sk-test".into())
        .await
        .expect("validation should parse mock server response");

    assert_eq!(models, vec!["model-a", "model-b"]);
}

#[test]
fn sanitize_provider_error_extracts_json_message() {
    use crate::sanitize_provider_error;
    let body = r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid bearer token"}}"#;
    assert_eq!(
        sanitize_provider_error(reqwest::StatusCode::UNAUTHORIZED, body),
        "Invalid bearer token"
    );
}

#[test]
fn sanitize_provider_error_falls_back_to_status_message() {
    use crate::sanitize_provider_error;
    assert_eq!(
        sanitize_provider_error(reqwest::StatusCode::UNAUTHORIZED, "not json"),
        "Invalid API key (unauthorized)."
    );
    assert_eq!(
        sanitize_provider_error(reqwest::StatusCode::TOO_MANY_REQUESTS, "not json"),
        "Rate limited. Please wait a moment and try again."
    );
    assert_eq!(
        sanitize_provider_error(reqwest::StatusCode::SERVICE_UNAVAILABLE, "not json"),
        "Provider server error. Please try again later."
    );
}
