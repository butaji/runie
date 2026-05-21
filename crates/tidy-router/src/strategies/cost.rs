use async_trait::async_trait;
use crate::{Router, RouterError, RoutingContext, ProviderMetadata};
use std::collections::HashMap;

/// Routes to the cheapest provider that meets requirements.
pub struct CostRouter {
    providers: HashMap<String, ProviderMetadata>,
}

impl CostRouter {
    pub fn new(providers: HashMap<String, ProviderMetadata>) -> Self {
        Self { providers }
    }
}

#[async_trait]
impl Router for CostRouter {
    async fn select_provider(
        &self,
        context: &RoutingContext,
        available: &[String],
    ) -> Result<String, RouterError> {
        let mut candidates: Vec<_> = available.iter()
            .filter_map(|name| self.providers.get(name))
            .filter(|p| p.max_tokens >= context.estimated_tokens)
            .filter(|p| {
                context.required_capabilities.iter().all(|cap| p.capabilities.contains(cap))
            })
            .collect();

        if candidates.is_empty() {
            return Err(RouterError::NoSuitableProvider);
        }

        // Sort by estimated cost
        candidates.sort_by(|a, b| {
            let cost_a = a.cost_per_1k_input * context.estimated_tokens as f64 / 1000.0;
            let cost_b = b.cost_per_1k_input * context.estimated_tokens as f64 / 1000.0;
            cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(candidates[0].name.clone())
    }

    async fn should_handoff(
        &self,
        _current: &str,
        _context: &RoutingContext,
    ) -> Result<Option<String>, RouterError> {
        // Cost router doesn't proactively handoff
        Ok(None)
    }
}
