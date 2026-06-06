//! Tests for runie-provider

use runie_core::provider::{Message, Provider};
use crate::{MockProvider, model::{ModelId, ModelRegistry, builtin_providers, ProviderMeta}};

#[test]
fn test_mock_provider_generates_chunks() {
    let provider = MockProvider;
    let messages = vec![Message::User { content: "Hello World".to_string() }];
    let chunks = provider.generate(messages);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].content, "Hello ");
    assert_eq!(chunks[1].content, "World ");
}

#[test]
fn test_mock_provider_empty_input() {
    let provider = MockProvider;
    let messages = vec![];
    let chunks = provider.generate(messages);

    assert!(chunks.is_empty());
}

#[test]
fn test_mock_provider_single_word() {
    let provider = MockProvider;
    let messages = vec![Message::User { content: "Hello".to_string() }];
    let chunks = provider.generate(messages);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello ");
}

#[test]
fn test_mock_provider_triggers_list_files() {
    let provider = MockProvider;
    let messages = vec![Message::User { content: "list files".to_string() }];
    let chunks = provider.generate(messages);

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("TOOL:list_dir"));
}

#[test]
fn test_mock_provider_triggers_read_file() {
    let provider = MockProvider;
    let messages = vec![Message::User { content: "read the readme".to_string() }];
    let chunks = provider.generate(messages);

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("TOOL:read_file"));
}

#[test]
fn test_mock_provider_triggers_write_file() {
    let provider = MockProvider;
    let messages = vec![Message::User { content: "write something".to_string() }];
    let chunks = provider.generate(messages);

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("TOOL:write_file"));
}

#[test]
fn test_mock_provider_triggers_bash() {
    let provider = MockProvider;
    let messages = vec![Message::User { content: "run command".to_string() }];
    let chunks = provider.generate(messages);

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].content.contains("TOOL:bash"));
}

#[test]
fn test_mock_provider_follows_up_after_tool_result() {
    let provider = MockProvider;
    let messages = vec![
        Message::User { content: "list files".to_string() },
        Message::Assistant { content: "TOOL:list_dir:.".to_string() },
        Message::ToolResult { content: "file1.txt (file)".to_string() },
    ];
    let chunks = provider.generate(messages);

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
    assert!(registry.find("mock/echo").is_some());
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
