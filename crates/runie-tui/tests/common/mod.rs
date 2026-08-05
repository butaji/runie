//! Shared test helpers for runie-tui integration tests.

use runie_core::types::Model;

pub fn test_model() -> Model {
    Model {
        id: "test-model".into(),
        name: "test".into(),
        api: "test".into(),
        provider: "test".into(),
        base_url: String::new(),
        reasoning: false,
        context_window: 0,
        max_tokens: 0,
        ..Default::default()
    }
}
