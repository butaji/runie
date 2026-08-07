//! Pure Pi-compatible model catalog and scoped-model selection semantics.
//!
//! Refreshing the catalog and committing a selection belong to an owning
//! actor. This module only reduces immutable catalog inputs, so it is safe to
//! use from actors, YAML replay, and the TUI's declarative selector.

use crate::types::Model;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedModel {
    pub model: Model,
    pub thinking_level: Option<crate::types::ThinkingLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCatalog {
    pub available: Vec<Model>,
    pub scoped: Vec<ScopedModel>,
}

impl ModelCatalog {
    pub fn new(available: Vec<Model>, scoped: Vec<ScopedModel>) -> Self {
        Self { available, scoped }
    }

    /// Pi's selector searches provider/id/name, preserving catalog order.
    pub fn search(&self, query: &str, scoped_only: bool) -> Vec<Model> {
        let query = query.trim().to_ascii_lowercase();
        let models: Vec<&Model> = if scoped_only {
            self.scoped
                .iter()
                .filter(|item| self.is_available(&item.model))
                .map(|item| &item.model)
                .collect()
        } else {
            self.available.iter().collect()
        };
        models
            .into_iter()
            .filter(|model| {
                query.is_empty()
                    || model.id.to_ascii_lowercase().contains(&query)
                    || model.name.to_ascii_lowercase().contains(&query)
                    || model.provider.to_ascii_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    /// Cycle through the effective scoped set, or all available models when
    /// no scope is configured. A missing current model starts at the first
    /// model, matching Pi's initial-selection behavior.
    pub fn cycle(&self, current: Option<&Model>, direction: CycleDirection) -> Option<ScopedModel> {
        let candidates: Vec<ScopedModel> = if self.scoped.is_empty() {
            self.available
                .iter()
                .cloned()
                .map(|model| ScopedModel {
                    model,
                    thinking_level: None,
                })
                .collect()
        } else {
            self.scoped
                .iter()
                .filter(|item| self.is_available(&item.model))
                .cloned()
                .collect()
        };
        if candidates.len() <= 1 {
            return None;
        }
        let current_index = current
            .and_then(|model| {
                candidates
                    .iter()
                    .position(|item| same_model(&item.model, model))
            })
            .unwrap_or(0);
        let next = match direction {
            CycleDirection::Forward => (current_index + 1) % candidates.len(),
            CycleDirection::Backward => {
                current_index.checked_sub(1).unwrap_or(candidates.len() - 1)
            }
        };
        Some(candidates[next].clone())
    }

    fn is_available(&self, model: &Model) -> bool {
        self.available
            .iter()
            .any(|candidate| same_model(candidate, model))
    }
}

fn same_model(left: &Model, right: &Model) -> bool {
    left.provider == right.provider && left.id == right.id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(provider: &str, id: &str, name: &str) -> Model {
        Model {
            provider: provider.into(),
            id: id.into(),
            name: name.into(),
            ..Model::default()
        }
    }

    #[test]
    fn search_is_case_insensitive_and_preserves_catalog_order() {
        let catalog = ModelCatalog::new(
            vec![
                model("openai", "gpt-5", "GPT Five"),
                model("xai", "grok", "Grok"),
            ],
            vec![],
        );
        assert_eq!(
            catalog
                .search("GPT", false)
                .into_iter()
                .map(|model| model.id)
                .collect::<Vec<_>>(),
            vec!["gpt-5"]
        );
    }

    #[test]
    fn cycle_wraps_and_drops_unavailable_scoped_models() {
        let gpt = model("openai", "gpt-5", "GPT Five");
        let grok = model("xai", "grok", "Grok");
        let catalog = ModelCatalog::new(
            vec![gpt.clone(), grok.clone()],
            vec![
                ScopedModel {
                    model: gpt.clone(),
                    thinking_level: Some(crate::types::ThinkingLevel::High),
                },
                ScopedModel {
                    model: model("missing", "gone", "Gone"),
                    thinking_level: None,
                },
                ScopedModel {
                    model: grok.clone(),
                    thinking_level: None,
                },
            ],
        );
        assert_eq!(
            catalog
                .cycle(Some(&gpt), CycleDirection::Forward)
                .unwrap()
                .model,
            grok
        );
        assert_eq!(
            catalog
                .cycle(Some(&gpt), CycleDirection::Backward)
                .unwrap()
                .model,
            grok
        );
    }
}
