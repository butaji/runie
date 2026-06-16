//! Model trait resolver — maps abstract `ModelTrait` requests to concrete models.
//!
//! The Orchestrator requests models by trait (e.g. `Reasoning`, `Vision`). This
//! module resolves those requests against the configured model profiles.
//!
//! **Auto-derivation:** if a profile has no explicit traits, they are derived
//! from its `ModelCapabilities` (streaming → Fast, reasoning → Reasoning,
//! vision → Vision, context_window > 200k → LongContext, else → General).

use crate::model_catalog::{ModelCapabilities, ModelInfo};
use crate::orchestrator::ModelTrait;
use serde::{Deserialize, Serialize};
use std::fmt;

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
    pub fn new(provider: impl Into<String>, model: impl Into<String>, traits: Vec<ModelTrait>) -> Self {
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

// ─────────────────────────────────────────────────────────────────────────────
// Resolution error
// ─────────────────────────────────────────────────────────────────────────────

/// Error returned when trait resolution fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolverError {
    /// No model profile matches the requested trait.
    NoMatch { trait_: ModelTrait },
    /// No models are configured at all.
    NoModelsConfigured,
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolverError::NoMatch { trait_ } => {
                write!(f, "no model configured with trait '{}'", trait_.label())
            }
            ResolverError::NoModelsConfigured => {
                write!(f, "no models configured")
            }
        }
    }
}

impl std::error::Error for ResolverError {}

// ─────────────────────────────────────────────────────────────────────────────
// Model resolver
// ─────────────────────────────────────────────────────────────────────────────

/// Resolves `ModelTrait` requests to concrete model profiles.
///
/// Resolution strategy:
/// 1. **Exact match**: profile whose only declared trait matches the request.
/// 2. **Partial match**: profile that has the requested trait; tie-break by
///    number of matching traits (more = better); further tie-break by `priority`.
/// 3. **Fallback**: `General` is accepted by any model.
pub struct ModelResolver {
    profiles: Vec<ModelProfile>,
    /// Optional priority ordering for tie-breaking (earlier = preferred).
    priority: Vec<String>,
}

impl ModelResolver {
    /// Build a resolver from a list of profiles.
    pub fn new(profiles: Vec<ModelProfile>) -> Self {
        Self {
            profiles,
            priority: Vec::new(),
        }
    }

    /// Build a resolver from `ModelInfo` entries (auto-derive traits).
    pub fn from_catalog(catalog: &[ModelInfo]) -> Self {
        let profiles = catalog.iter().map(ModelProfile::from_info).collect();
        Self {
            profiles,
            priority: Vec::new(),
        }
    }

    /// Set a priority list. Models listed earlier are preferred when scores tie.
    /// Model keys are `"provider/model"` format.
    #[allow(dead_code)]
    pub fn with_priority(mut self, priority: Vec<String>) -> Self {
        self.priority = priority;
        self
    }

    /// Resolve a trait request to the best matching profile.
    ///
    /// Returns `Err(ResolverError)` if no profile matches.
    pub fn resolve(&self, trait_: ModelTrait) -> Result<&ModelProfile, ResolverError> {
        if self.profiles.is_empty() {
            return Err(ResolverError::NoModelsConfigured);
        }

        let mut candidates: Vec<&ModelProfile> = self
            .profiles
            .iter()
            .filter(|p| p.has_trait(trait_))
            .collect();

        if candidates.is_empty() {
            return Err(ResolverError::NoMatch { trait_ });
        }

        // Sort by score: lower tier/score wins; priority and input order tie-break.
        candidates.sort_by(|a, b| {
            let score_a = self.score(a, trait_);
            let score_b = self.score(b, trait_);
            score_a
                .cmp(&score_b)
                .then_with(|| self.priority_index(a).cmp(&self.priority_index(b)))
                // Natural key order: earlier-inserted profiles come first when
                // scores and priority are equal.
                .then_with(|| a.key().cmp(&b.key()))
        });

        Ok(candidates[0])
    }

    /// Resolve all traits in a plan (each task's model_trait) to profiles.
    /// Returns one profile per trait, deduplicating by key.
    pub fn resolve_all<'a>(
        &'a self,
        traits: &[ModelTrait],
    ) -> Vec<Result<&'a ModelProfile, ResolverError>> {
        traits.iter().map(|&t| self.resolve(t)).collect()
    }

    /// Number of profiles in the resolver.
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Whether the resolver has no profiles.
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    /// All registered profiles.
    pub fn profiles(&self) -> &[ModelProfile] {
        &self.profiles
    }

    /// Score is a `(tier, spec_score)` tuple. The tier determines priority; spec_score
    /// breaks ties within a tier.
    ///
    /// Tiers (lower = higher priority):
    ///   0 = exact single-trait match (requested is the only trait)
    ///   1 = partial match (requested + other traits, i.e. more specialized)
    ///   2 = partial match (requested + General only, i.e. less specialized)
    ///
    /// For General requests: tiers 0 and 2 are swapped so that single-trait General
    /// (tier 0 = exact) beats [General, X] (tier 2) — but tier 0 still beats tier 1.
    pub(crate) fn score(&self, profile: &ModelProfile, requested: ModelTrait) -> (u8, isize) {
        let has_requested = profile.has_trait(requested);

        // Non-General traits beyond the requested one.
        let extra_non_general: usize = profile
            .traits
            .iter()
            .filter(|&&t| t != ModelTrait::General && t != requested)
            .count();

        // Whether the profile only has General alongside the requested trait.
        let general_only_extra = profile.has_trait(ModelTrait::General)
            && profile.traits.len() == 2;

        if profile.traits.len() == 1 && has_requested {
            // Exact single-trait match → tier 0.
            (0, extra_non_general as isize)
        } else if has_requested {
            if requested == ModelTrait::General {
                // [General, X]: tier 2 for General requests (least specialized).
                // [X, General]: tier 2 when X is not the requested non-General.
                if general_only_extra {
                    (2, extra_non_general as isize)
                } else {
                    // [General, X, Y] — still tier 1 (more specialized than [General,X])
                    (1, extra_non_general as isize)
                }
            } else {
                // Partial match on a non-General request.
                (1, extra_non_general as isize)
            }
        } else {
            // No match → tier 3 (never returned by resolve anyway)
            (3, extra_non_general as isize)
        }
    }

    fn priority_index(&self, profile: &ModelProfile) -> usize {
        self.priority
            .iter()
            .position(|p| p == &profile.key())
            .unwrap_or(usize::MAX)
    }
}

impl Default for ModelResolver {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver(profiles: Vec<ModelProfile>) -> ModelResolver {
        ModelResolver::new(profiles)
    }

    fn p(provider: &str, model: &str, traits: Vec<ModelTrait>) -> ModelProfile {
        ModelProfile::new(provider, model, traits)
    }

    // ── Exact trait wins ──────────────────────────────────────────────────

    #[test]
    fn exact_trait_wins() {
        let r = resolver(vec![
            p("openai", "gpt-4o", vec![ModelTrait::General, ModelTrait::Vision]),
            p("anthropic", "o3-mini", vec![ModelTrait::Reasoning]),
        ]);
        assert_eq!(r.resolve(ModelTrait::Reasoning).unwrap().model, "o3-mini");
    }

    #[test]
    fn general_trait_falls_back_to_any() {
        let r = resolver(vec![
            p("openai", "gpt-4o", vec![ModelTrait::General]),
        ]);
        assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "gpt-4o");
    }

    #[test]
    fn general_falls_back_to_any_with_multiple() {
        let r = resolver(vec![
            p("openai", "gpt-4o", vec![ModelTrait::General]),
            p("anthropic", "claude-3", vec![ModelTrait::Vision, ModelTrait::General]),
        ]);
        // gpt-4o: tier 0 exact [General]; claude: tier 2 [General,Vision] requesting General.
        // Tier 0 < tier 2 → gpt-4o wins.
        assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "gpt-4o");
    }

    // ── Partial match ──────────────────────────────────────────────────────

    #[test]
    fn most_matching_traits_wins_on_partial_match() {
        let r = resolver(vec![
            p("fast", "coder", vec![ModelTrait::General]),
            p("capable", "coder-plus", vec![ModelTrait::Reasoning]),
        ]);
        // Both have General; coder-plus has Reasoning (extra non-General trait).
        // When requesting General: coder=tier 0 (exact), coder-plus=tier 2 (partial).
        // Tier 0 wins. This test verifies exact General beats partial General.
        assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "coder");

        // But when requesting Reasoning: both match, coder=tier 2, coder-plus=tier 1.
        // Tier 1 wins → coder-plus wins (more specialized for this request).
        assert_eq!(r.resolve(ModelTrait::Reasoning).unwrap().model, "coder-plus");
    }

    // ── Priority breaks ties ─────────────────────────────────────────────

    #[test]
    fn priority_breaks_ties() {
        let r = resolver(vec![
            p("openai", "first", vec![ModelTrait::General]),
            p("anthropic", "second", vec![ModelTrait::General]),
        ])
        .with_priority(vec!["openai/first".into(), "anthropic/second".into()]);

        assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "first");
    }

    #[test]
    fn priority_with_reverse_order() {
        let r = resolver(vec![
            p("openai", "first", vec![ModelTrait::General]),
            p("anthropic", "second", vec![ModelTrait::General]),
        ])
        .with_priority(vec!["anthropic/second".into(), "openai/first".into()]);

        assert_eq!(r.resolve(ModelTrait::General).unwrap().model, "second");
    }

    // ── No match ──────────────────────────────────────────────────────────

    #[test]
    fn no_match_returns_error() {
        let r = resolver(vec![p("fast", "tiny", vec![ModelTrait::Fast])]);
        let err = r.resolve(ModelTrait::Vision).unwrap_err();
        assert!(matches!(err, ResolverError::NoMatch { trait_: ModelTrait::Vision }));
    }

    #[test]
    fn empty_resolver_returns_no_models() {
        let r: ModelResolver = resolver(Vec::new());
        let err = r.resolve(ModelTrait::General).unwrap_err();
        assert!(matches!(err, ResolverError::NoModelsConfigured));
    }

    // ── ModelProfile helpers ────────────────────────────────────────────

    #[test]
    fn profile_key() {
        let p = ModelProfile::new("openai", "gpt-4o", vec![ModelTrait::General]);
        assert_eq!(p.key(), "openai/gpt-4o");
    }

    #[test]
    fn profile_has_trait() {
        let p = ModelProfile::new("anthropic", "claude-3", vec![ModelTrait::Vision, ModelTrait::General]);
        assert!(p.has_trait(ModelTrait::Vision));
        assert!(p.has_trait(ModelTrait::General));
        assert!(!p.has_trait(ModelTrait::Reasoning));
    }

    #[test]
    fn model_profile_from_info() {
        let info = ModelInfo {
            name: "gpt-4o".into(),
            provider: "openai".into(),
            display_name: "GPT-4o".into(),
            cost_prompt: None,
            cost_completion: None,
            supports_thinking: false,
            supports_vision: true,
            tokenizer: None,
            context_window: Some(128_000),
            capabilities: ModelCapabilities {
                streaming: true,
                supports_vision: true,
                supports_tools: true,
                supports_reasoning: false,
                max_context_tokens: 128_000,
                max_output_tokens: 16_384,
                cache_control: false,
            },
        };
        let profile = ModelProfile::from_info(&info);
        assert_eq!(profile.provider, "openai");
        assert_eq!(profile.model, "gpt-4o");
        assert!(profile.has_trait(ModelTrait::Vision));
        assert!(profile.has_trait(ModelTrait::Fast)); // supports_tools && !reasoning
        assert!(profile.has_trait(ModelTrait::General));
        assert!(!profile.has_trait(ModelTrait::Reasoning));
        assert!(!profile.has_trait(ModelTrait::LongContext)); // context_window <= 200k
    }

    #[test]
    fn reasoning_model_derives_correct_traits() {
        let info = ModelInfo {
            name: "o3-mini".into(),
            provider: "anthropic".into(),
            display_name: "o3-mini".into(),
            cost_prompt: None,
            cost_completion: None,
            supports_thinking: true,
            supports_vision: false,
            tokenizer: None,
            context_window: Some(200_000),
            capabilities: ModelCapabilities {
                streaming: false,
                supports_vision: false,
                supports_tools: true,
                supports_reasoning: true,
                max_context_tokens: 200_000,
                max_output_tokens: 100_000,
                cache_control: true,
            },
        };
        let profile = ModelProfile::from_info(&info);
        assert!(profile.has_trait(ModelTrait::Reasoning));
        assert!(profile.has_trait(ModelTrait::LongContext)); // max_context_tokens > 200k
        assert!(profile.has_trait(ModelTrait::General));
        assert!(!profile.has_trait(ModelTrait::Fast)); // has reasoning → not fast
        assert!(!profile.has_trait(ModelTrait::Vision));
    }

    // ── Resolve all ─────────────────────────────────────────────────────

    #[test]
    fn resolve_all_deduplicates() {
        let r = resolver(vec![
            p("anthropic", "claude", vec![ModelTrait::General, ModelTrait::Vision]),
        ]);
        let results = r.resolve_all(&[ModelTrait::General, ModelTrait::Vision]);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert_eq!(results[0].as_ref().unwrap().model, "claude");
        assert_eq!(results[1].as_ref().unwrap().model, "claude");
    }

    // ── Error display ────────────────────────────────────────────────────

    #[test]
    fn resolver_error_display() {
        let err = ResolverError::NoMatch { trait_: ModelTrait::Vision };
        assert!(err.to_string().contains("vision"));
        assert!(err.to_string().contains("no model"));

        let err2 = ResolverError::NoModelsConfigured;
        assert!(err2.to_string().contains("no models"));
    }
}
