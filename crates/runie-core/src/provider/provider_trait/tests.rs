//! Tests for the provider trait and types.

use super::*;
use crate::model_catalog::ModelCapabilitiesBuilder;

// ─── Layer 1: typed error variant displays ─────────────────────────────────

#[test]
fn missing_api_key_display_names_provider_and_env_var() {
    let err = ProviderError::MissingApiKey("MINIMAX_API_KEY".into());
    let msg = err.to_string();
    assert!(msg.contains("minimax"), "{msg}");
    assert!(msg.contains("MINIMAX_API_KEY"), "{msg}");
    assert!(msg.contains("[model_providers.minimax]"), "{msg}");
}

#[test]
fn typed_error_display_rate_limit() {
    let err = ProviderError::RateLimit {
        retry_after_secs: Some(60),
    };
    let msg = err.to_string();
    assert!(msg.contains("Rate limited"), "{msg}");
    assert!(err.is_retryable());
    assert!(!err.is_fatal());
}

#[test]
fn typed_error_display_network() {
    let err = ProviderError::Network("connection refused".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Network error"), "{msg}");
    assert!(err.is_retryable());
    assert!(!err.is_fatal());
}

#[test]
fn typed_error_display_timeout() {
    let err = ProviderError::Timeout;
    let msg = err.to_string();
    assert!(msg.contains("timed out"), "{msg}");
    assert!(err.is_retryable());
    assert!(!err.is_fatal());
}

#[test]
fn typed_error_display_server() {
    let err = ProviderError::Server(502, "Bad Gateway".to_string());
    let msg = err.to_string();
    assert!(msg.contains("502"), "{msg}");
    assert!(err.is_retryable());
    assert!(!err.is_fatal());
}

#[test]
fn typed_error_display_auth() {
    let err = ProviderError::Auth(401);
    let msg = err.to_string();
    assert!(msg.contains("Authentication failed"), "{msg}");
    assert!(msg.contains("401"), "{msg}");
    assert!(!err.is_retryable());
    assert!(err.is_fatal());
}

#[test]
fn typed_error_display_context_length() {
    let err = ProviderError::ContextLength(128_000);
    let msg = err.to_string();
    assert!(msg.contains("Context length exceeded"), "{msg}");
    assert!(msg.contains("128000"), "{msg}");
    assert!(!err.is_retryable());
    assert!(err.is_fatal());
}

// ─── Layer 1: is_retryable determinism ─────────────────────────────────────

#[test]
fn retryable_is_true_for_transient_errors() {
    let transient = [
        ProviderError::RateLimit {
            retry_after_secs: None,
        },
        ProviderError::Network("connection refused".into()),
        ProviderError::Timeout,
        ProviderError::Server(500, Default::default()),
        ProviderError::Server(503, "Service Unavailable".into()),
    ];
    for err in transient {
        assert!(err.is_retryable(), "expected {err:?} to be retryable");
    }
}

#[test]
fn retryable_is_false_for_fatal_errors() {
    let fatal = [
        ProviderError::Auth(401),
        ProviderError::Auth(403),
        ProviderError::ContextLength(100_000),
        ProviderError::UnknownProvider("foo".into()),
        ProviderError::MissingApiKey("OPENAI_API_KEY".into()),
        ProviderError::ConfigNotLoaded,
    ];
    for err in fatal {
        assert!(!err.is_retryable(), "expected {err:?} to NOT be retryable");
        assert!(err.is_fatal(), "expected {err:?} to be fatal");
    }
}

#[test]
fn clone_preserves_variant_and_data() {
    let err = ProviderError::Server(503, "Service Unavailable".into());
    let cloned = err.clone();
    assert!(matches!(cloned, ProviderError::Server(503, msg) if msg == "Service Unavailable"));

    let auth_err = ProviderError::Auth(401);
    assert!(matches!(auth_err.clone(), ProviderError::Auth(401)));

    let rate_err = ProviderError::RateLimit {
        retry_after_secs: Some(30),
    };
    assert!(matches!(
        rate_err.clone(),
        ProviderError::RateLimit {
            retry_after_secs: Some(30)
        }
    ));
}

// ─── Layer 1: existing error display messages are preserved ────────────────

#[test]
fn central_error_displays_preserve_messages() {
    let cases = [
        (
            ProviderError::UnknownProvider("my-model".into()),
            "Unknown provider: my-model",
        ),
        (
            ProviderError::MissingApiKey("OPENAI_API_KEY".into()),
            "Missing API key",
        ),
        (ProviderError::ConfigNotLoaded, "Configuration not loaded"),
    ];
    for (err, prefix) in cases {
        let msg = err.to_string();
        assert!(
            msg.starts_with(prefix),
            "expected message to start with '{prefix}', got: {msg}"
        );
    }
}

// Layer 1: provider errors are still identifiable by variant
#[test]
fn provider_error_source_round_trips() {
    let anyhow_err = anyhow::anyhow!("network error: connection refused");
    let err: ProviderError = anyhow_err.into();
    let msg = err.to_string();
    // The underlying error message is preserved in the display
    assert!(
        msg.contains("network error"),
        "expected 'network error' in: {msg}"
    );
    assert!(
        msg.contains("connection refused"),
        "expected 'connection refused' in: {msg}"
    );
    // The variant is still Source
    assert!(
        matches!(err, ProviderError::Source(_)),
        "expected Source variant, got: {err:?}"
    );
}

// ─── Layer 1: ProviderMetadata tests ────────────────────────────────────────

#[test]
fn provider_metadata_default_values() {
    let meta = ProviderMetadata::default();
    assert!(meta.model_info.is_none());
    assert!(!meta.streaming);
    assert!(!meta.supports_tools);
    assert_eq!(
        meta.retry_config.max_attempts,
        DEFAULT_RETRY_CONFIG.max_attempts
    );
}

#[test]
fn provider_metadata_with_model_info() {
    let info = ModelInfo::new("openai", "gpt-4o");
    let meta = ProviderMetadata::new().with_model_info(info);
    assert!(meta.model_info.is_some());
    assert_eq!(meta.model_info.as_ref().unwrap().name, "gpt-4o");
    assert_eq!(meta.model_info.as_ref().unwrap().provider, "openai");
}

#[test]
fn provider_metadata_with_custom_retry_config() {
    let custom_config = RetryConfig::new(10, Duration::from_secs(1), Duration::from_secs(60), 3.0);
    let meta = ProviderMetadata::new().with_retry_config(custom_config.clone());
    assert_eq!(meta.retry_config.max_attempts, 10);
    assert_eq!(meta.retry_config.multiplier, 3.0);
}

#[test]
fn provider_metadata_streaming_flag() {
    let meta = ProviderMetadata::new().with_streaming(true);
    assert!(meta.streaming);

    let meta = ProviderMetadata::new().with_streaming(false);
    assert!(!meta.streaming);
}

#[test]
fn provider_metadata_supports_tools_flag() {
    let meta = ProviderMetadata::new().with_supports_tools(true);
    assert!(meta.supports_tools);

    let meta = ProviderMetadata::new().with_supports_tools(false);
    assert!(!meta.supports_tools);
}

// ─── Layer 1: RetryConfig tests ────────────────────────────────────────────

#[test]
fn retry_config_default_values() {
    let config = RetryConfig::default();
    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.initial_delay, Duration::from_millis(100));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert_eq!(config.multiplier, 2.0);
}

#[test]
fn retry_config_no_retry() {
    let config = RetryConfig::no_retry();
    assert_eq!(config.max_attempts, 1);
    assert_eq!(config.initial_delay, Duration::from_secs(0));
    assert_eq!(config.max_delay, Duration::from_secs(0));
    assert_eq!(config.multiplier, 1.0);
}

#[test]
fn retry_config_custom_values() {
    let config = RetryConfig::new(3, Duration::from_secs(1), Duration::from_secs(10), 1.5);
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_delay, Duration::from_secs(1));
    assert_eq!(config.max_delay, Duration::from_secs(10));
    assert_eq!(config.multiplier, 1.5);
}

#[test]
fn retry_config_clone_preserves_values() {
    let config = RetryConfig::new(7, Duration::from_secs(2), Duration::from_secs(120), 4.0);
    let cloned = config.clone();
    assert_eq!(cloned.max_attempts, config.max_attempts);
    assert_eq!(cloned.initial_delay, config.initial_delay);
    assert_eq!(cloned.max_delay, config.max_delay);
    assert_eq!(cloned.multiplier, config.multiplier);
}

#[test]
fn retry_config_partial_eq() {
    let config1 = RetryConfig::new(5, Duration::from_secs(1), Duration::from_secs(30), 2.0);
    let config2 = RetryConfig::new(5, Duration::from_secs(1), Duration::from_secs(30), 2.0);
    let config3 = RetryConfig::new(6, Duration::from_secs(1), Duration::from_secs(30), 2.0);
    assert_eq!(config1, config2);
    assert_ne!(config1, config3);
}

#[test]
fn retry_config_derive_builder() {
    // Exercise the derive_builder generated API for RetryConfig.
    // derive_builder generates StructNameBuilder (not StructName::builder()).
    let config = RetryConfigBuilder::default()
        .max_attempts(10)
        .initial_delay(Duration::from_millis(500))
        .max_delay(Duration::from_secs(60))
        .multiplier(2.0)
        .build()
        .unwrap();
    assert_eq!(config.max_attempts, 10);
    assert_eq!(config.initial_delay, Duration::from_millis(500));
    assert_eq!(config.max_delay, Duration::from_secs(60));
    assert_eq!(config.multiplier, 2.0);
}

#[test]
fn provider_metadata_derive_builder() {
    // Exercise the derive_builder generated API for ProviderMetadata.
    // ModelCapabilities requires all its fields when built via derive_builder.
    // ProviderMetadata fields model_info and retry_config are provided explicitly.
    let caps = ModelCapabilitiesBuilder::default()
        .streaming(true)
        .supports_vision(true)
        .supports_tools(true)
        .supports_reasoning(false)
        .max_context_tokens(128_000)
        .max_output_tokens(8_192)
        .cache_control(false)
        .build()
        .unwrap();
    let retry = RetryConfigBuilder::default()
        .max_attempts(5)
        .initial_delay(Duration::from_millis(100))
        .max_delay(Duration::from_secs(30))
        .multiplier(2.0)
        .build()
        .unwrap();
    let meta = ProviderMetadataBuilder::default()
        .capabilities(caps)
        .retry_config(retry)
        .streaming(true)
        .supports_tools(true)
        .build()
        .unwrap();
    assert!(meta.streaming);
    assert!(meta.supports_tools);
    assert!(meta.capabilities.streaming);
    assert!(meta.capabilities.supports_vision);
    assert!(meta.capabilities.supports_tools);
}
