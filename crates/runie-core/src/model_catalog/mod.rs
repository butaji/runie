//! Model catalog for the model selector dialog.

use crate::model::ModelSelectorItem;
use derive_builder::Builder;

pub mod configured;
pub use configured::configured_models_catalog;

/// Model capability flags — drives runtime adaptation and UI filtering.
#[derive(Clone, Debug, Default, PartialEq, Builder)]
pub struct ModelCapabilities {
    pub streaming: bool,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub max_context_tokens: usize,
    pub max_output_tokens: usize,
    pub cache_control: bool,
}

impl ModelCapabilities {
    pub fn streaming() -> Self {
        Self {
            streaming: true,
            ..Default::default()
        }
    }

    pub fn with_vision(mut self) -> Self {
        self.supports_vision = true;
        self
    }

    pub fn with_tools(mut self) -> Self {
        self.supports_tools = true;
        self
    }

    pub fn with_reasoning(mut self) -> Self {
        self.supports_reasoning = true;
        self
    }

    pub fn with_context(mut self, max_context: usize) -> Self {
        self.max_context_tokens = max_context;
        self
    }

    pub fn with_output_limit(mut self, max_output: usize) -> Self {
        self.max_output_tokens = max_output;
        self
    }

    pub fn with_cache_control(mut self) -> Self {
        self.cache_control = true;
        self
    }
}

/// Information about a model for the model selector dialog.
#[derive(Clone, Debug, PartialEq, Builder)]
#[builder(setter(strip_option))]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub display_name: String,
    pub cost_prompt: Option<f64>,
    pub cost_completion: Option<f64>,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub tokenizer: Option<String>,
    pub context_window: Option<usize>,
    pub capabilities: ModelCapabilities,
}

impl ModelInfo {
    pub fn new(provider: impl Into<String>, name: impl Into<String>) -> Self {
        let provider = provider.into();
        let name = name.into();
        Self {
            display_name: name.clone(),
            name,
            provider,
            cost_prompt: None,
            cost_completion: None,
            supports_thinking: false,
            supports_vision: false,
            tokenizer: None,
            context_window: None,
            capabilities: ModelCapabilities::default(),
        }
    }

    pub fn with_cost(mut self, prompt: f64, completion: f64) -> Self {
        self.cost_prompt = Some(prompt);
        self.cost_completion = Some(completion);
        self
    }

    pub fn with_thinking(mut self) -> Self {
        self.supports_thinking = true;
        self
    }

    pub fn with_vision(mut self) -> Self {
        self.supports_vision = true;
        self
    }

    pub fn full(&self) -> String {
        format!("{}/{}", self.provider, self.name)
    }
}

/// Static catalog of known models for the model selector.
/// Derived from the provider registry so provider and model metadata stay in sync.
pub fn model_catalog() -> Vec<ModelInfo> {
    let mut models = Vec::new();
    for provider in crate::provider::known_providers() {
        for model in &provider.models {
            let capabilities = ModelCapabilities {
                streaming: model.streaming,
                supports_vision: model.supports_vision,
                supports_tools: model.supports_tools,
                supports_reasoning: model.supports_reasoning,
                max_context_tokens: model.context_window.unwrap_or(0),
                max_output_tokens: model.max_output_tokens,
                cache_control: model.cache_control,
            };
            models.push(
                ModelInfoBuilder::default()
                    .provider(provider.key.clone())
                    .name(model.name.clone())
                    .display_name(model.name.clone())
                    .cost_prompt(model.cost_prompt.unwrap_or_default())
                    .cost_completion(model.cost_completion.unwrap_or_default())
                    .supports_thinking(model.supports_thinking)
                    .supports_vision(model.supports_vision)
                    .tokenizer(model.tokenizer.clone().unwrap_or_default())
                    .context_window(model.context_window.unwrap_or_default())
                    .capabilities(capabilities)
                    .build()
                    .unwrap(),
            );
        }
    }
    models
}

/// Filter and score models by name, provider, or display name using fuzzy matching.
/// Returns indices sorted by match score (highest first).
pub fn filter_models(models: &[ModelInfo], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..models.len()).collect();
    }

    let mut scored: Vec<(usize, i32)> = models
        .iter()
        .enumerate()
        .filter_map(|(idx, m)| {
            // Try matching on full name (provider/model), name, and provider separately
            let full = m.full();
            let name = &m.name;
            let provider = &m.provider;
            let display = &m.display_name;

            // Score the best match across all fields
            let best_score = [
                fuzzy_score(query, &full),
                fuzzy_score(query, name),
                fuzzy_score(query, provider),
                fuzzy_score(query, display),
            ]
            .into_iter()
            .flatten()
            .max();

            best_score.map(|score| (idx, score))
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(idx, _)| idx).collect()
}

/// Score a fuzzy match between `query` and `candidate` using `sublime_fuzzy`.
fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    sublime_fuzzy::best_match(query, candidate).map(|m| m.score() as i32)
}

/// Build grouped model selector items for the snapshot.
/// Returns (provider_header, display_name, cost_str, is_selected, is_current) tuples.
pub fn build_model_selector_items(
    models: &[ModelInfo],
    recent: &[String],
    filter: &str,
    current_provider: &str,
    current_model: &str,
) -> Vec<ModelSelectorItem> {
    let indices = model_indices(models, filter);
    let mut items = build_recent_items(models, recent, filter, current_provider, current_model);
    let main_items = build_main_items(
        models,
        &indices,
        recent,
        filter,
        current_provider,
        current_model,
    );
    items.extend(main_items);
    items
}

fn model_indices(models: &[ModelInfo], filter: &str) -> Vec<usize> {
    if filter.is_empty() {
        (0..models.len()).collect()
    } else {
        filter_models(models, filter)
    }
}

fn build_recent_items(
    models: &[ModelInfo],
    recent: &[String],
    filter: &str,
    current_provider: &str,
    current_model: &str,
) -> Vec<ModelSelectorItem> {
    if !filter.is_empty() || recent.is_empty() {
        return Vec::new();
    }
    recent
        .iter()
        .rev()
        .take(5)
        .filter_map(|r| models.iter().position(|m| m.full() == *r))
        .map(|idx| {
            let m = &models[idx];
            model_item("Recent", m, current_provider, current_model, false)
        })
        .collect()
}

fn build_main_items(
    models: &[ModelInfo],
    indices: &[usize],
    recent: &[String],
    filter: &str,
    current_provider: &str,
    current_model: &str,
) -> Vec<ModelSelectorItem> {
    let mut items = Vec::new();
    let mut last_provider = String::new();
    for &idx in indices {
        let m = &models[idx];
        if filter.is_empty() && recent.contains(&m.full()) {
            continue;
        }
        let header = provider_header(&mut last_provider, &m.provider);
        items.push(model_item(
            &header,
            m,
            current_provider,
            current_model,
            false,
        ));
    }
    items
}

fn provider_header(last_provider: &mut String, provider: &str) -> String {
    if provider != *last_provider {
        *last_provider = provider.to_owned();
        provider.to_owned()
    } else {
        String::new()
    }
}

fn model_item(
    header: &str,
    m: &ModelInfo,
    current_provider: &str,
    current_model: &str,
    selected: bool,
) -> ModelSelectorItem {
    let cost = format_cost(m.cost_prompt, m.cost_completion);
    let is_current = m.provider == current_provider && m.name == current_model;
    (header.to_owned(), m.full(), cost, selected, is_current)
}

fn format_cost(prompt: Option<f64>, completion: Option<f64>) -> String {
    match (prompt, completion) {
        (Some(p), Some(c)) => format!("${:.2}/${:.2}", p, c),
        (Some(p), None) => format!("${:.2}/?", p),
        (None, Some(c)) => format!("?/${:.2}", c),
        (None, None) => String::new(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::ModelCapabilitiesBuilder;

    #[test]
    fn model_catalog_is_not_empty() {
        assert!(!model_catalog().is_empty());
    }

    #[test]
    fn model_catalog_contains_all_provider_default_models() {
        let catalog = model_catalog();
        for provider in crate::provider::known_providers() {
            for model in &provider.models {
                assert!(
                    catalog
                        .iter()
                        .any(|m| m.provider == provider.key && m.name == model.name),
                    "model catalog missing {}/{}",
                    provider.key,
                    model.name
                );
            }
        }
    }

    #[test]
    fn registry_model_has_consistent_provider() {
        for provider in crate::provider::known_providers() {
            for model in &provider.models {
                let caps = ModelCapabilitiesBuilder::default()
                    .streaming(model.streaming)
                    .supports_vision(model.supports_vision)
                    .supports_tools(model.supports_tools)
                    .supports_reasoning(model.supports_reasoning)
                    .max_context_tokens(model.context_window.unwrap_or(0))
                    .max_output_tokens(model.max_output_tokens)
                    .cache_control(model.cache_control)
                    .build()
                    .unwrap();
                let info = ModelInfoBuilder::default()
                    .provider(provider.key.clone())
                    .name(model.name.clone())
                    .display_name(model.name.clone())
                    .cost_prompt(model.cost_prompt.unwrap_or_default())
                    .cost_completion(model.cost_completion.unwrap_or_default())
                    .supports_thinking(model.supports_thinking)
                    .supports_vision(model.supports_vision)
                    .tokenizer(model.tokenizer.clone().unwrap_or_default())
                    .context_window(model.context_window.unwrap_or_default())
                    .capabilities(caps)
                    .build()
                    .unwrap();
                assert_eq!(info.provider, provider.key);
                assert_eq!(info.full(), format!("{}/{}", provider.key, model.name));
            }
        }
    }

    #[test]
    fn model_catalog_preserves_costs_and_flags() {
        let catalog = model_catalog();
        let gpt4o = catalog
            .iter()
            .find(|m| m.provider == "openai" && m.name == "gpt-4o")
            .expect("gpt-4o should exist");
        assert_eq!(gpt4o.cost_prompt, Some(5.0));
        assert_eq!(gpt4o.cost_completion, Some(15.0));
        assert!(gpt4o.supports_vision);
        assert!(!gpt4o.supports_thinking);

        let o1 = catalog
            .iter()
            .find(|m| m.provider == "openai" && m.name == "o1")
            .expect("o1 should exist");
        assert!(o1.supports_thinking);
        assert!(o1.supports_vision);
    }

    #[test]
    fn model_capabilities_derives_from_registry() {
        let catalog = model_catalog();
        // Default streaming=true, tools=true
        let gpt4o = catalog
            .iter()
            .find(|m| m.provider == "openai" && m.name == "gpt-4o")
            .unwrap();
        assert!(gpt4o.capabilities.streaming);
        assert!(gpt4o.capabilities.supports_tools);
        assert!(gpt4o.capabilities.supports_vision);
        assert!(!gpt4o.capabilities.supports_reasoning);
    }

    #[test]
    fn model_capabilities_builder() {
        let caps = ModelCapabilities::streaming()
            .with_vision()
            .with_tools()
            .with_reasoning()
            .with_context(200_000)
            .with_output_limit(40_000)
            .with_cache_control();
        assert!(caps.streaming);
        assert!(caps.supports_vision);
        assert!(caps.supports_tools);
        assert!(caps.supports_reasoning);
        assert_eq!(caps.max_context_tokens, 200_000);
        assert_eq!(caps.max_output_tokens, 40_000);
        assert!(caps.cache_control);
    }

    #[test]
    fn model_capabilities_derive_builder() {
        // Exercise the derive_builder generated API for ModelCapabilities.
        // derive_builder generates StructNameBuilder::default()...build(), not StructName::builder().
        let caps = ModelCapabilitiesBuilder::default()
            .streaming(true)
            .supports_vision(true)
            .supports_tools(true)
            .supports_reasoning(true)
            .max_context_tokens(128_000)
            .max_output_tokens(8_192)
            .cache_control(true)
            .build()
            .unwrap();
        assert!(caps.streaming);
        assert!(caps.supports_vision);
        assert!(caps.supports_tools);
        assert!(caps.supports_reasoning);
        assert_eq!(caps.max_context_tokens, 128_000);
        assert_eq!(caps.max_output_tokens, 8_192);
        assert!(caps.cache_control);
    }

    #[test]
    fn model_catalog_groups_by_provider() {
        let catalog = model_catalog();
        let mut provider_positions: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (i, m) in catalog.iter().enumerate() {
            provider_positions
                .entry(m.provider.clone())
                .or_default()
                .push(i);
        }
        for (provider, positions) in provider_positions {
            let min = *positions.iter().min().unwrap();
            let max = *positions.iter().max().unwrap();
            assert_eq!(
                max - min + 1,
                positions.len(),
                "provider {} models should be contiguous",
                provider
            );
        }
    }

    #[test]
    fn model_catalog_lookup_resolves_context_and_pricing() {
        // Test that context limit and pricing resolve by model id
        let catalog = model_catalog();

        // gpt-4o: known model with full metadata
        let gpt4o = catalog
            .iter()
            .find(|m| m.full() == "openai/gpt-4o")
            .expect("gpt-4o should exist");
        assert_eq!(gpt4o.context_window, Some(128_000));
        assert_eq!(gpt4o.cost_prompt, Some(5.0));
        assert_eq!(gpt4o.cost_completion, Some(15.0));

        // claude-sonnet-4-6: known model with full metadata
        let claude = catalog
            .iter()
            .find(|m| m.full() == "anthropic/claude-sonnet-4-6")
            .expect("claude-sonnet-4-6 should exist");
        assert_eq!(claude.context_window, Some(200_000));
        assert!(claude.cost_prompt.is_some());
        assert!(claude.cost_completion.is_some());

        // Minimax-M3: known model
        let minimax = catalog
            .iter()
            .find(|m| m.full() == "minimax/MiniMax-M3")
            .expect("MiniMax-M3 should exist");
        assert_eq!(minimax.context_window, Some(256_000));
        assert!(minimax.capabilities.streaming);
        assert!(minimax.capabilities.supports_tools);
    }

    // ── Fuzzy matching tests ─────────────────────────────────────────────────

    #[test]
    fn filter_models_empty_query_returns_all() {
        let catalog = model_catalog();
        let indices = filter_models(&catalog, "");
        assert_eq!(indices.len(), catalog.len());
        // Should be in original order
        assert_eq!(indices[0], 0);
    }

    #[test]
    fn filter_models_exact_match() {
        let catalog = model_catalog();
        let indices = filter_models(&catalog, "gpt-4o");
        assert!(!indices.is_empty());
        // First result should be gpt-4o
        let first = &catalog[indices[0]];
        assert_eq!(first.name, "gpt-4o");
        assert_eq!(first.provider, "openai");
    }

    #[test]
    fn filter_models_fuzzy_typo() {
        let catalog = model_catalog();
        // "gpt4" should still find "gpt-4o" via fuzzy matching
        let indices = filter_models(&catalog, "gpt4");
        assert!(!indices.is_empty());
        // Should find gpt-4o even with typo
        let has_gpt4o = indices.iter().any(|&i| catalog[i].name == "gpt-4o");
        assert!(has_gpt4o, "fuzzy match should find gpt-4o for query 'gpt4'");
    }

    #[test]
    fn filter_models_sorted_by_score() {
        let catalog = model_catalog();
        // Query that matches multiple models should sort by score
        let indices = filter_models(&catalog, "claude");
        assert!(!indices.is_empty());
        // First result should be a claude model
        let first = &catalog[indices[0]];
        assert!(first.name.contains("claude") || first.provider == "anthropic");
    }

    #[test]
    fn filter_models_by_provider() {
        let catalog = model_catalog();
        let indices = filter_models(&catalog, "openai");
        assert!(!indices.is_empty());
        // First result should be from openai (highest score)
        assert_eq!(catalog[indices[0]].provider, "openai");
    }
}
