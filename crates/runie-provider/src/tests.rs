//! Tests for runie-provider

use crate::{DynProvider, MockProvider, MockStreamingProvider};
use futures::StreamExt;
use runie_core::config::Config;
use runie_core::llm_event::LLMEvent;
use runie_core::message::ChatMessage;
use runie_core::provider::{Provider, ProviderError};
use std::io::{Read, Write};
use std::sync::Mutex;

/// Guards environment variable mutations during parallel test execution.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Helper to run a test closure with the env lock held.
fn with_env_lock<F, T>(var: &str, value: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = ENV_LOCK.lock().unwrap();
    std::env::set_var(var, value);
    let result = f();
    std::env::remove_var(var);
    result
}

/// Collect all text deltas from a stream.
async fn collect_text(
    stream: impl futures::Stream<Item = anyhow::Result<LLMEvent>> + Unpin,
) -> Vec<String> {
    let mut texts = Vec::new();
    futures::pin_mut!(stream);
    while let Some(result) = stream.next().await {
        if let Ok(LLMEvent::TextDelta(t)) = result {
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
    let p = MockProvider::with_delay(500, 3000);
    assert_eq!(p.delay_ms(), Some((500, 3000)));
}

#[tokio::test]
async fn test_dyn_provider_mock_delay_adds_streaming_delay() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("RUNIE_MOCK");
    std::env::set_var("RUNIE_MOCK_DELAY", "1");

    let provider = DynProvider::new_with_config("mock", "echo", &Config::default()).expect("mock should build with RUNIE_MOCK_DELAY");
    let start = std::time::Instant::now();
    let texts = collect_text(provider.generate(vec![ChatMessage::user("hello world".to_string())])).await;

    std::env::remove_var("RUNIE_MOCK_DELAY");
    assert!(!texts.is_empty());
    assert!(
        start.elapsed() >= std::time::Duration::from_millis(50),
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
// DynProvider tests
// ============================================================================

#[test]
fn test_dyn_provider_unknown_returns_err() {
    let result = DynProvider::new_with_config("bogus-provider-xyz", "gpt-4o", &Config::default());
    match result {
        Err(ProviderError::UnknownProvider(key)) => assert_eq!(key, "bogus-provider-xyz"),
        other => panic!("expected UnknownProvider error, got: {:?}", other),
    }
}

#[test]
fn test_dyn_provider_missing_api_key_returns_err() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("OPENAI_API_KEY");
    let result = DynProvider::new_with_config("openai", "gpt-4o-mini", &Config::default());
    match result {
        Err(ProviderError::MissingApiKey(var)) => assert_eq!(var, "OPENAI_API_KEY"),
        other => panic!("expected MissingApiKey error, got: {:?}", other),
    }
}

#[test]
fn test_dyn_provider_known_with_key_succeeds() {
    with_env_lock("OPENAI_API_KEY", "test-key-123", || {
        let result = DynProvider::new_with_config("openai", "gpt-4o-mini", &Config::default());
        let provider = result.expect("DynProvider should succeed with API key");
        assert_eq!(provider.key(), "openai");
        assert_eq!(provider.model(), "gpt-4o-mini");
    });
}

#[test]
fn test_dyn_provider_key_and_model_accessors() {
    with_env_lock("OPENAI_API_KEY", "sk-test", || {
        let provider = DynProvider::new_with_config("openai", "gpt-4o", &Config::default()).unwrap();
        assert_eq!(provider.key(), "openai");
        assert_eq!(provider.model(), "gpt-4o");
    });
}

#[test]
fn test_provider_trait_is_dyn_compatible() {
    let _: Box<dyn Provider> = Box::new(MockProvider::default());
    with_env_lock("OPENAI_API_KEY", "sk-dyn-test", || {
        let dp = DynProvider::new_with_config("openai", "gpt-4o", &Config::default()).unwrap();
        let _: Box<dyn Provider> = Box::new(dp);
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

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    std::thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        std::thread::sleep(Duration::from_secs(30));
    });

    let start = std::time::Instant::now();
    let result = crate::validate_api_key_with_timeout(
        &format!("http://127.0.0.1:{}/v1", port),
        "sk-test",
        Duration::from_millis(250),
    )
    .await;

    let elapsed = start.elapsed();
    assert!(result.is_err(), "hanging server should produce an error");
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

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let auth = request
            .lines()
            .find(|l| l.to_lowercase().starts_with("authorization:"))
            .unwrap_or("")
            .to_string();
        let _ = tx.send(auth);
        let body = r#"{"data":[{"id":"MiniMax-M3"},{"id":"MiniMax-M2.7"}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });

    let models = crate::validate_api_key_with_timeout(
        &format!("http://127.0.0.1:{}/v1", port),
        "  sk-test\n ",
        Duration::from_secs(2),
    )
    .await
    .expect("valid MiniMax-style response should succeed");

    assert_eq!(models, vec!["MiniMax-M3", "MiniMax-M2.7"]);
    let auth = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("server sent auth header");
    assert!(
        auth.contains("Bearer sk-test"),
        "key should be trimmed in request header: {}",
        auth
    );
}

#[test]
fn test_dyn_provider_trims_whitespace_api_key_from_env() {
    with_env_lock("OPENAI_API_KEY", "  sk-with-space\n ", || {
        let provider = DynProvider::new_with_config("openai", "gpt-4o-mini", &Config::default()).expect("trimmed key should build");
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
