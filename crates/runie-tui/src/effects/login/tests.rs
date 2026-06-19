use super::validation_base_url;
use runie_core::config::{Config, ModelProvider};

#[test]
fn validation_base_url_uses_saved_custom_url() {
    let mut config = Config::default();
    config.model_providers.insert(
        "openai".into(),
        ModelProvider {
            provider_type: None,
            base_url: "http://proxy.local/v1".into(),
            api_key: "test-key".into(),
            models: Vec::new(),
        },
    );

    assert_eq!(
        validation_base_url("openai", &config),
        Some("http://proxy.local/v1".into())
    );
}

#[test]
fn validation_base_url_returns_none_when_no_saved_url() {
    let config = Config::default();

    assert_eq!(validation_base_url("openai", &config), None);
}
