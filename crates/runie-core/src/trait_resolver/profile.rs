//! Model profile — maps a concrete `(provider, model)` pair to a set of traits.

use crate::model_catalog::{ModelCapabilities, ModelInfo};
use crate::orchestrator::ModelTrait;
use serde::{Deserialize, Serialize};

/// A model profile maps a concrete `(provider, model)` pair to a set of traits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Provider key (e.g. "openai").
    pub provider: String,
    /// Model name (e.g. "gpt-4o").
    pub model: String,
    /// Declared traits. Empty means auto-derived from `ModelCapabilities`.
    pub traits: Vec<ModelTrait>,
}

impl ModelProfile {
    /// Build a profile from a `ModelInfo` with auto-derived traits.
    pub fn from_info(info: &ModelInfo) -> Self {
        Self {
            provider: info.provider.clone(),
            model: info.name.clone(),
            traits: Self::derive_traits(&info.capabilities),
        }
    }

    /// Build a profile with explicit traits.
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        traits: Vec<ModelTrait>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            traits,
        }
    }

    /// Derive traits from model capabilities.
    ///
    /// Priority: reasoning > vision > long-context > fast > general
    fn derive_traits(caps: &ModelCapabilities) -> Vec<ModelTrait> {
        let mut traits = Vec::new();

        if caps.supports_reasoning {
            traits.push(ModelTrait::Reasoning);
        }
        if caps.supports_vision {
            traits.push(ModelTrait::Vision);
        }
        // LongContext derives from the model's effective context window cap.
        if caps.max_context_tokens >= 200_000 {
            traits.push(ModelTrait::LongContext);
        }
        // A fast model is one that supports tools and is not reasoning-focused.
        if caps.supports_tools && !caps.supports_reasoning {
            traits.push(ModelTrait::Fast);
        }
        // Every model supports general tasks.
        traits.push(ModelTrait::General);

        traits
    }

    /// Unique key: `"provider/model"`.
    pub fn key(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }

    /// Whether this profile declares the given trait.
    pub fn has_trait(&self, trait_: ModelTrait) -> bool {
        self.traits.contains(&trait_)
    }

    /// Number of declared traits.
    pub fn trait_count(&self) -> usize {
        self.traits.len()
    }
}
