use async_trait::async_trait;
use crate::{Router, RouterError, RoutingContext, ProviderMetadata};
use std::collections::HashMap;

/// Routes to the most capable provider for the task.
pub struct CapabilityRouter {
    providers: HashMap<String, ProviderMetadata>,
}

impl CapabilityRouter {
    pub fn new(providers: HashMap<String, ProviderMetadata>) -> Self {
        Self { providers }
    }
}

#[async_trait]
impl Router for CapabilityRouter {
    async fn select_provider(
        &self,
        context: &RoutingContext,
        available: &[String],
    ) -> Result<String, RouterError> {
        let mut candidates: Vec<_> = available.iter()
            .filter_map(|name| self.providers.get(name))
            .filter(|p| p.max_tokens >= context.estimated_tokens)
            .collect();

        if candidates.is_empty() {
            return Err(RouterError::NoSuitableProvider);
        }

        // Score by capability match + reliability
        candidates.sort_by(|a, b| {
            let score_a = a.capabilities.len() as f32 * a.reliability_score;
            let score_b = b.capabilities.len() as f32 * b.reliability_score;
            score_b.total_cmp(&score_a)
        });

        Ok(candidates[0].name.clone())
    }

    async fn should_handoff(
        &self,
        current: &str,
        context: &RoutingContext,
    ) -> Result<Option<String>, RouterError> {
        // Handoff if current provider lacks required capabilities
        if let Some(current_meta) = self.providers.get(current) {
            let missing: Vec<_> = context.required_capabilities.iter()
                .filter(|cap| !current_meta.capabilities.contains(cap))
                .collect();
            
            if !missing.is_empty() {
                // Find provider with all capabilities
                for (_, meta) in &self.providers {
                    if meta.name != current && 
                       context.required_capabilities.iter().all(|cap| meta.capabilities.contains(cap)) {
                        return Ok(Some(meta.name.clone()));
                    }
                }
            }
        }
        Ok(None)
    }
}
