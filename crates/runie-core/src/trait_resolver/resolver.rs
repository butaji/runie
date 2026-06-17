//! Model resolver — resolves `ModelTrait` requests to concrete model profiles.

use crate::model_catalog::ModelInfo;
use crate::orchestrator::ModelTrait;

use super::{ModelProfile, ResolverError};

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
        let general_only_extra =
            profile.has_trait(ModelTrait::General) && profile.traits.len() == 2;

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
