//! Tests for runie-provider

use crate::{
    model::{builtin_providers, ModelId, ModelRegistry, ProviderMeta},
    DynProvider, MockProvider, MockStreamingProvider,
};
use futures::StreamExt;
use runie_core::provider::{Message, Provider, ProviderError};

#[tokio::test]
async fn test_mock_provider_generates_chunks() {
    let provider = MockProvider::default();
    let messages = vec![Message::User {
        content: "Hello World".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].content, "Hello ");
    assert_eq!(chunks[1].content, "World ");
}

#[tokio::test]
async fn test_mock_provider_empty_input() {
    let provider = MockProvider::default();
    let messages = vec![];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_mock_provider_single_word() {
    let provider = MockProvider::default();
    let messages = vec![Message::User {
        content: "Hello".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello ");
}

#[tokio::test]
async fn test_mock_provider_triggers_list_files() {
    let provider = MockProvider::default();
    let messages = vec![Message::User {
        content: "list files".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert_eq!(chunks.len(), 2);
    assert!(chunks[1].content.contains("TOOL:list_dir"));
}

#[tokio::test]
async fn test_mock_provider_triggers_read_file() {
    let provider = MockProvider::default();
    let messages = vec![Message::User {
        content: "read the readme".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert_eq!(chunks.len(), 2);
    assert!(chunks[1].content.contains("TOOL:read_file"));
}

#[tokio::test]
async fn test_mock_provider_triggers_write_file() {
    let provider = MockProvider::default();
    let messages = vec![Message::User {
        content: "write something".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert_eq!(chunks.len(), 2);
    assert!(chunks[1].content.contains("TOOL:write_file"));
}

#[tokio::test]
async fn test_mock_provider_triggers_bash() {
    let provider = MockProvider::default();
    let messages = vec![Message::User {
        content: "run command".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert_eq!(chunks.len(), 2);
    assert!(chunks[1].content.contains("TOOL:bash"));
}

#[tokio::test]
async fn test_mock_provider_follows_up_after_tool_result() {
    let provider = MockProvider::default();
    let messages = vec![
        Message::User {
            content: "list files".to_string(),
        },
        Message::Assistant {
            content: "TOOL:list_dir:.".to_string(),
        },
        Message::ToolResult {
            content: "file1.txt (file)".to_string(),
        },
    ];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("Done"));
}

#[test]
fn test_model_id_full() {
    let model = ModelId::new("openai", "gpt-4o");
    assert_eq!(model.full(), "openai/gpt-4o");
}

#[test]
fn test_model_registry_default_not_empty() {
    let registry = ModelRegistry::default();
    let models = registry.list();
    assert!(!models.is_empty());
}

#[test]
fn test_model_registry_find_exists() {
    let registry = ModelRegistry::default();
    assert!(registry.find("openai/gpt-4o").is_some());
    assert!(registry.find("anthropic/claude-sonnet-4-6").is_some());
    // mock/echo is only present when dev flags enable the mock provider.
    if runie_core::provider_registry::is_mock_enabled() {
        assert!(registry.find("mock/echo").is_some());
    }
}

#[test]
fn test_model_registry_find_missing() {
    let registry = ModelRegistry::default();
    assert!(registry.find("openai/gpt-99").is_none());
}

#[test]
fn test_model_registry_by_provider() {
    let registry = ModelRegistry::default();
    let openai_models = registry.by_provider("openai");
    assert!(!openai_models.is_empty());
    for m in &openai_models {
        assert_eq!(m.provider, "openai");
    }
}

#[test]
fn test_model_registry_register() {
    let mut registry = ModelRegistry::default();
    registry.register(ModelId::new("custom", "my-model"));
    assert!(registry.find("custom/my-model").is_some());
}

#[test]
fn test_builtin_providers_has_anthropic() {
    let providers = builtin_providers();
    assert!(providers.iter().any(|p| p.key == "anthropic"));
    assert!(providers.iter().any(|p| p.key == "openai"));
    assert!(providers.iter().any(|p| p.key == "ollama"));
}

#[test]
fn test_provider_meta_env_var() {
    let meta = ProviderMeta::new("anthropic", "ANTHROPIC_API_KEY");
    assert_eq!(meta.env_var, "ANTHROPIC_API_KEY");
    assert!(meta.base_url_hint.is_none());
}

#[test]
fn test_provider_meta_with_url() {
    let meta = ProviderMeta::with_url("ollama", "OLLAMA_HOST", "http://localhost:11434/v1");
    assert_eq!(meta.base_url_hint, Some("http://localhost:11434/v1"));
}

#[test]
fn test_registry_has_ollama_models() {
    let registry = ModelRegistry::default();
    let ollama = registry.by_provider("ollama");
    assert!(!ollama.is_empty());
    assert!(ollama.iter().any(|m| m.name == "llama3.1"));
}

#[test]
fn test_registry_has_openrouter_models() {
    let registry = ModelRegistry::default();
    let or = registry.by_provider("openrouter");
    assert!(!or.is_empty());
}

#[test]
fn test_registry_has_bedrock_models() {
    let registry = ModelRegistry::default();
    let bedrock = registry.by_provider("amazon-bedrock");
    assert!(!bedrock.is_empty());
}

#[tokio::test]
async fn mock_provider_default_no_delay() {
    let p = MockProvider::default();
    let start = std::time::Instant::now();
    let mut chunks = Vec::new();
    let mut stream = p.generate(vec![Message::User {
        content: "test".to_string(),
    }]);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert!(start.elapsed() < std::time::Duration::from_millis(50));
    assert!(!chunks.is_empty());
}

#[test]
fn mock_provider_with_delay_configured() {
    let p = MockProvider::with_delay(500, 3000);
    assert_eq!(p.delay_ms(), Some((500, 3000)));
}

// =============================================================================
// DynProvider tests (Layer 1 — state/logic)
// =============================================================================

#[test]
fn test_dyn_provider_unknown_returns_err() {
    // `DynProvider::new` must not silently fall back to Mock for unknown keys.
    let result = DynProvider::new("bogus-provider-xyz", "gpt-4o");
    match result {
        Err(ProviderError::UnknownProvider(key)) => {
            assert_eq!(key, "bogus-provider-xyz");
        }
        other => panic!("expected UnknownProvider error, got: {:?}", other),
    }
}

#[test]
fn test_dyn_provider_missing_api_key_returns_err() {
    // Without RUNIE_MOCK, an unknown provider with no API key returns MissingApiKey.
    // We use a known-but-unconfigured provider to avoid the UnknownProvider path.
    let result = DynProvider::new("openai", "gpt-4o-mini");
    match result {
        Err(ProviderError::MissingApiKey(var)) => {
            assert_eq!(var, "OPENAI_API_KEY");
        }
        other => panic!("expected MissingApiKey error, got: {:?}", other),
    }
}

#[test]
fn test_dyn_provider_known_with_key_succeeds() {
    // With a mocked API key, DynProvider::new succeeds.
    std::env::set_var("OPENAI_API_KEY", "test-key-123");
    let result = DynProvider::new("openai", "gpt-4o-mini");
    std::env::remove_var("OPENAI_API_KEY");

    let provider = result.expect("DynProvider should succeed with API key");
    assert_eq!(provider.key(), "openai");
    assert_eq!(provider.model(), "gpt-4o-mini");
}

#[test]
fn test_dyn_provider_key_and_model_accessors() {
    std::env::set_var("OPENAI_API_KEY", "sk-test");
    let provider = DynProvider::new("openai", "gpt-4o").unwrap();
    std::env::remove_var("OPENAI_API_KEY");

    assert_eq!(provider.key(), "openai");
    assert_eq!(provider.model(), "gpt-4o");
}

#[test]
fn test_provider_trait_is_dyn_compatible() {
    // Compile-time assertion: concrete providers can be stored behind dyn Provider.
    let _: Box<dyn Provider> = Box::new(MockProvider::default());
    // Also verify OpenAiProvider (via DynProvider construction since we can't
    // construct it without an API key — use a known mock key).
    std::env::set_var("OPENAI_API_KEY", "sk-dyn-test");
    let dp = DynProvider::new("openai", "gpt-4o").unwrap();
    std::env::remove_var("OPENAI_API_KEY");
    let _: Box<dyn Provider> = Box::new(dp);
}

#[test]
fn test_build_provider_with_warning_returns_err_for_unknown() {
    let result = crate::build_provider_with_warning("not-a-provider", "gpt-4");
    assert!(
        result.is_err(),
        "build_provider_with_warning should return error for unknown provider"
    );
    assert!(matches!(
        result.unwrap_err(),
        ProviderError::UnknownProvider(_)
    ));
}

#[test]
fn test_build_provider_panics_for_unknown() {
    // `build_provider` panics when the key is unknown (callers must validate).
    let result = std::panic::catch_unwind(|| crate::build_provider("totally-invalid-key", "model"));
    assert!(
        result.is_err(),
        "build_provider should panic for unknown provider"
    );
}

#[test]
fn test_is_known_provider() {
    assert!(crate::is_known("openai"));
    assert!(crate::is_known("anthropic"));
    assert!(!crate::is_known("not-a-real-provider"));
}

// =============================================================================
// MockStreamingProvider tests
// =============================================================================

#[tokio::test]
async fn test_mock_streaming_provider_basic() {
    let provider = MockStreamingProvider::new();
    let messages = vec![Message::User {
        content: "Hello".to_string(),
    }];
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert!(!chunks.is_empty());
    for chunk in &chunks {
        assert!(!chunk.content.is_empty());
    }
}

#[tokio::test]
async fn test_mock_streaming_provider_at_rate() {
    let provider = MockStreamingProvider::with_rate(100.0);
    let messages = vec![Message::User {
        content: "test".to_string(),
    }];
    let start = std::time::Instant::now();
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    let elapsed = start.elapsed();
    assert!(!chunks.is_empty());
    assert!(elapsed < std::time::Duration::from_secs(5));
}

#[tokio::test]
async fn test_mock_streaming_provider_no_delay() {
    let mut provider = MockStreamingProvider::new();
    provider.delay_ms = 0;
    let messages = vec![Message::User {
        content: "Hi".to_string(),
    }];
    let start = std::time::Instant::now();
    let mut chunks = Vec::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        chunks.push(r.unwrap());
    }
    assert!(start.elapsed() < std::time::Duration::from_millis(50));
    assert!(!chunks.is_empty());
}

#[tokio::test]
async fn test_validate_api_key_times_out_on_hanging_server() {
    use std::time::Duration;

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    std::thread::spawn(move || {
        let (mut _stream, _) = listener.accept().unwrap();
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
        "should return quickly due to timeout, took {:?}",
        elapsed
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
async fn test_mock_streaming_provider_accumulates_content() {
    let provider = MockStreamingProvider::new();
    let messages = vec![Message::User {
        content: "test".to_string(),
    }];
    let mut full_content = String::new();
    let mut stream = provider.generate(messages);
    while let Some(r) = stream.next().await {
        full_content.push_str(&r.unwrap().content);
    }
    assert!(full_content.contains("test"));
    assert!(full_content.len() > 10);
}
