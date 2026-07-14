//! Model routing strategies for multi-deployment providers.

use crate::config::ModelRoutingStrategy;
use crate::provider::ModelMeta;

/// Select a model based on the routing strategy.
///
/// For `SimpleShuffle`, returns the first model (round-robin is handled
/// externally via deployment tracking). `LatencyBased` and `CostBased`
/// select the optimal model from the available list.
pub fn route_model<'a>(
    strategy: &'a ModelRoutingStrategy,
    models: &'a [ModelMeta],
    request_size_hint: Option<usize>,
) -> Option<&'a ModelMeta> {
    match strategy {
        ModelRoutingStrategy::SimpleShuffle => models.first(),
        ModelRoutingStrategy::LatencyBased => {
            // Latency-based: prefer models with lower latency estimates.
            // Currently uses context window size as a proxy (smaller = faster).
            models
                .iter()
                .min_by_key(|m| m.context_window.unwrap_or(usize::MAX))
        }
        ModelRoutingStrategy::CostBased => {
            // Cost-based: prefer models with lower combined cost.
            let request_size = request_size_hint.unwrap_or(1000);
            // Assume ~50% prompt, ~50% completion for cost estimation.
            let prompt_tokens = request_size / 2;
            let completion_tokens = request_size / 2;
            models.iter().min_by_key(|m| {
                let prompt_cost = m.cost_prompt.unwrap_or(0.0) * prompt_tokens as f64;
                let completion_cost = m.cost_completion.unwrap_or(0.0) * completion_tokens as f64;
                (prompt_cost + completion_cost) as u64
            })
        }
    }
}

/// Select a fallback model when context window is exceeded.
pub fn select_context_fallback<'a>(
    fallback_list: &'a [String],
    models: &'a [ModelMeta],
) -> Option<&'a ModelMeta> {
    for name in fallback_list {
        if let Some(model) = models.iter().find(|m| &m.name == name) {
            return Some(model);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model(
        name: &str,
        context_window: usize,
        cost_prompt: f64,
        cost_completion: f64,
    ) -> ModelMeta {
        ModelMeta {
            name: name.to_string(),
            cost_prompt: Some(cost_prompt),
            cost_completion: Some(cost_completion),
            supports_thinking: false,
            supports_vision: false,
            tokenizer: None,
            context_window: Some(context_window),
            streaming: true,
            supports_tools: true,
            supports_reasoning: false,
            supports_system: true,
            max_output_tokens: 4096,
            cache_control: false,
        }
    }

    #[test]
    fn simple_shuffle_uses_first_model() {
        let models = vec![
            make_model("model-a", 128000, 0.0, 0.0),
            make_model("model-b", 64000, 0.0, 0.0),
        ];
        let result = route_model(&ModelRoutingStrategy::SimpleShuffle, &models, None);
        assert_eq!(result.map(|m| m.name.as_str()), Some("model-a"));
    }

    #[test]
    fn latency_based_uses_smallest_context() {
        let models = vec![
            make_model("model-a", 128000, 0.0, 0.0),
            make_model("model-b", 32000, 0.0, 0.0),
            make_model("model-c", 64000, 0.0, 0.0),
        ];
        let result = route_model(&ModelRoutingStrategy::LatencyBased, &models, None);
        assert_eq!(result.map(|m| m.name.as_str()), Some("model-b"));
    }

    #[test]
    fn cost_based_uses_cheapest_combined() {
        let models = vec![
            make_model("model-a", 128000, 0.01, 0.03), // $0.04/1k tokens
            make_model("model-b", 64000, 0.005, 0.01), // $0.015/1k tokens - cheapest
            make_model("model-c", 32000, 0.02, 0.02),  // $0.04/1k tokens
        ];
        let result = route_model(&ModelRoutingStrategy::CostBased, &models, Some(1000));
        assert_eq!(result.map(|m| m.name.as_str()), Some("model-b"));
    }

    #[test]
    fn context_fallback_selects_from_list() {
        let models = vec![
            make_model("model-a", 128000, 0.0, 0.0),
            make_model("model-b", 64000, 0.0, 0.0),
            make_model("model-c", 32000, 0.0, 0.0),
        ];
        let fallback = vec!["model-c".to_string(), "model-b".to_string()];
        let result = select_context_fallback(&fallback, &models);
        assert_eq!(result.map(|m| m.name.as_str()), Some("model-c"));
    }

    #[test]
    fn context_fallback_falls_back_to_second() {
        let models = vec![
            make_model("model-a", 128000, 0.0, 0.0),
            make_model("model-b", 64000, 0.0, 0.0),
        ];
        let fallback = vec!["model-x".to_string(), "model-b".to_string()];
        let result = select_context_fallback(&fallback, &models);
        assert_eq!(result.map(|m| m.name.as_str()), Some("model-b"));
    }

    #[test]
    fn context_fallback_returns_none_when_empty() {
        let models = vec![make_model("model-a", 128000, 0.0, 0.0)];
        let result = select_context_fallback(&[], &models);
        assert!(result.is_none());
    }
}
