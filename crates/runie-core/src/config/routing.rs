//! Model routing and deployment selection strategies.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A model identifier in `"provider/model"` form.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
pub struct ModelId(pub String);

impl ModelId {
    /// Create a new `ModelId` from a provider/model string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ModelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Routing strategy for selecting among multiple model deployments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Default)]
pub enum RoutingStrategy {
    /// Round-robin or random shuffle across available deployments.
    #[default]
    SimpleShuffle,
    /// Route to the lowest-latency deployment based on recent measurements.
    LatencyBased,
    /// Route to the cheapest deployment that meets requirements.
    CostBased,
    /// Route based on current usage/capacity load.
    UsageBased,
}

/// Configuration for a single model's routing behaviour.
///
/// Add this field to `ModelProvider` or a top-level `models` section
/// in `Config` to enable per-model strategy overrides.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ModelRoutingConfig {
    /// Strategy to use when multiple deployments are available.
    #[serde(default)]
    pub strategy: RoutingStrategy,

    /// Ordered list of fallback model IDs to try when the primary model
    /// cannot handle a request (e.g., context window exceeded).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_window_fallbacks: Vec<ModelId>,
}

/// Resolves the best deployment for a request using the configured strategy.
#[derive(Debug, Clone, Default)]
pub struct RouterStrategy;

impl RouterStrategy {
    /// Select the best deployment index from a list of candidates.
    ///
    /// `candidates` are deployment identifiers ordered by priority.
    /// The strategy chooses one based on its configuration.
    pub fn select(candidates: &[String], strategy: &RoutingStrategy) -> Option<usize> {
        if candidates.is_empty() {
            return None;
        }
        match strategy {
            RoutingStrategy::SimpleShuffle => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0);
                let idx = (nanos as usize) % candidates.len();
                Some(idx)
            }
            RoutingStrategy::LatencyBased | RoutingStrategy::CostBased | RoutingStrategy::UsageBased => {
                // Placeholder: all three currently fall back to the first candidate.
                // Latency, cost, and usage tracking require runtime state that
                // is outside the scope of this minimal config-only implementation.
                Some(0)
            }
        }
    }

    /// Iterate through context-window fallback models, returning the first
    /// one that exists in `available`.
    pub fn select_fallback(fallbacks: &[ModelId], available: &[ModelId]) -> Option<ModelId> {
        fallbacks.iter().find(|f| available.contains(f)).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_shuffle_returns_valid_index() {
        let candidates = vec!["openai/gpt-4o".to_string(), "openai/gpt-4o-mini".to_string()];
        let idx = RouterStrategy::select(&candidates, &RoutingStrategy::SimpleShuffle);
        assert!(idx.is_some());
        assert!(idx.unwrap() < candidates.len());
    }

    #[test]
    fn simple_shuffle_empty_returns_none() {
        let candidates: Vec<String> = vec![];
        let idx = RouterStrategy::select(&candidates, &RoutingStrategy::SimpleShuffle);
        assert!(idx.is_none());
    }

    #[test]
    fn latency_based_defaults_to_first() {
        let candidates = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(
            RouterStrategy::select(&candidates, &RoutingStrategy::LatencyBased),
            Some(0)
        );
    }

    #[test]
    fn cost_based_defaults_to_first() {
        let candidates = vec!["a".to_string(), "b".to_string()];
        assert_eq!(
            RouterStrategy::select(&candidates, &RoutingStrategy::CostBased),
            Some(0)
        );
    }

    #[test]
    fn usage_based_defaults_to_first() {
        let candidates = vec!["a".to_string(), "b".to_string()];
        assert_eq!(
            RouterStrategy::select(&candidates, &RoutingStrategy::UsageBased),
            Some(0)
        );
    }

    #[test]
    fn model_id_display() {
        let id = ModelId::new("anthropic/claude-3-5-sonnet");
        assert_eq!(format!("{}", id), "anthropic/claude-3-5-sonnet");
    }

    #[test]
    fn model_id_from_str() {
        let id: ModelId = "openai/gpt-4o".into();
        assert_eq!(id.as_str(), "openai/gpt-4o");
    }

    #[test]
    fn model_id_default() {
        let id = ModelId::default();
        assert_eq!(id.as_str(), "");
    }

    #[test]
    fn routing_strategy_default() {
        assert_eq!(RoutingStrategy::default(), RoutingStrategy::SimpleShuffle);
    }

    #[test]
    fn model_routing_config_default() {
        let cfg = ModelRoutingConfig::default();
        assert_eq!(cfg.strategy, RoutingStrategy::SimpleShuffle);
        assert!(cfg.context_window_fallbacks.is_empty());
    }

    #[test]
    fn select_fallback_finds_first_match() {
        let fallbacks = vec![ModelId::new("openai/gpt-4o"), ModelId::new("anthropic/claude-3-5-sonnet")];
        let available = vec![ModelId::new("anthropic/claude-3-5-sonnet"), ModelId::new("openai/gpt-4o-mini")];
        assert_eq!(
            RouterStrategy::select_fallback(&fallbacks, &available),
            Some(ModelId::new("anthropic/claude-3-5-sonnet"))
        );
    }

    #[test]
    fn select_fallback_returns_none_when_no_match() {
        let fallbacks = vec![ModelId::new("openai/gpt-4o")];
        let available = vec![ModelId::new("anthropic/claude-3-5-sonnet")];
        assert!(RouterStrategy::select_fallback(&fallbacks, &available).is_none());
    }

    #[test]
    fn select_fallback_empty_fallbacks() {
        let fallbacks: Vec<ModelId> = vec![];
        let available = vec![ModelId::new("openai/gpt-4o")];
        assert!(RouterStrategy::select_fallback(&fallbacks, &available).is_none());
    }

    #[test]
    fn routing_strategy_serialize() {
        let json = serde_json::to_string(&RoutingStrategy::SimpleShuffle).unwrap();
        assert_eq!(json, r#"{"type":"simple_shuffle"}"#);

        let json = serde_json::to_string(&RoutingStrategy::LatencyBased).unwrap();
        assert_eq!(json, r#"{"type":"latency_based"}"#);

        let json = serde_json::to_string(&RoutingStrategy::CostBased).unwrap();
        assert_eq!(json, r#"{"type":"cost_based"}"#);

        let json = serde_json::to_string(&RoutingStrategy::UsageBased).unwrap();
        assert_eq!(json, r#"{"type":"usage_based"}"#);
    }

    #[test]
    fn routing_strategy_deserialize() {
        let s: RoutingStrategy = serde_json::from_str(r#"{"type":"simple_shuffle"}"#).unwrap();
        assert_eq!(s, RoutingStrategy::SimpleShuffle);

        let l: RoutingStrategy = serde_json::from_str(r#"{"type":"latency_based"}"#).unwrap();
        assert_eq!(l, RoutingStrategy::LatencyBased);

        let c: RoutingStrategy = serde_json::from_str(r#"{"type":"cost_based"}"#).unwrap();
        assert_eq!(c, RoutingStrategy::CostBased);

        let u: RoutingStrategy = serde_json::from_str(r#"{"type":"usage_based"}"#).unwrap();
        assert_eq!(u, RoutingStrategy::UsageBased);
    }

    #[test]
    fn model_routing_config_roundtrip() {
        let cfg = ModelRoutingConfig {
            strategy: RoutingStrategy::CostBased,
            context_window_fallbacks: vec![
                ModelId::new("openai/gpt-4o-mini"),
                ModelId::new("anthropic/claude-3-haiku"),
            ],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let roundtrip: ModelRoutingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, cfg);
    }
}
