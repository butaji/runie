//! Token estimation and tracking.
//!
//! All estimation uses the chars/4 approximation. Provider-reported `usage`
//! fields on LLM responses are the authoritative source for billing/cost
//! calculations. Local estimates are used for pre-flight truncation decisions
//! and UI token counters, where ±20% accuracy is acceptable.

// =============================================================================
// Estimation
// =============================================================================

/// Approximate token count: one token per four characters, rounding up.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Estimate token count for the active provider/model.
/// Always uses the chars/4 approximation.
pub fn estimate_tokens_for_model(text: &str, _provider: &str, _model: &str) -> usize {
    estimate_tokens(text)
}

/// Estimate token count using the chars/4 approximation.
pub fn estimate_tokens_with_tokenizer(text: &str) -> usize {
    estimate_tokens(text)
}

// =============================================================================
// TokenTracker
// =============================================================================

/// Tracks input/output token totals for a session and the current turn.
///
/// **NOTE**: All token counts are estimates using the chars/4 heuristic.
/// Provider-reported `usage` fields on LLM responses are authoritative for
/// billing and cost calculations.
#[derive(Debug, Clone, Default)]
pub struct TokenTracker {
    input_total: usize,
    output_total: usize,
    turn_input: usize,
    turn_output: usize,
    input_cost_per_1m: f64,
    output_cost_per_1m: f64,
}

impl TokenTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_costs(input_cost: f64, output_cost: f64) -> Self {
        Self {
            input_cost_per_1m: input_cost,
            output_cost_per_1m: output_cost,
            ..Self::default()
        }
    }

    pub fn add_input(&mut self, tokens: usize) {
        self.input_total += tokens;
        self.turn_input += tokens;
    }

    pub fn add_output(&mut self, tokens: usize) {
        self.output_total += tokens;
        self.turn_output += tokens;
    }

    /// Estimate input tokens with the chars/4 heuristic and add them.
    pub fn track_input(&mut self, text: &str) {
        self.add_input(estimate_tokens(text));
    }

    /// Estimate output tokens with the chars/4 heuristic and add them.
    pub fn track_output(&mut self, text: &str) {
        self.add_output(estimate_tokens(text));
    }

    pub fn end_turn(&mut self) {
        self.turn_input = 0;
        self.turn_output = 0;
    }

    pub fn session_total(&self) -> usize {
        self.input_total + self.output_total
    }

    pub fn turn_total(&self) -> usize {
        self.turn_input + self.turn_output
    }

    pub fn input_total(&self) -> usize {
        self.input_total
    }

    pub fn output_total(&self) -> usize {
        self.output_total
    }

    pub fn session_cost(&self) -> f64 {
        let input_cost = self.input_total as f64 * self.input_cost_per_1m / 1_000_000.0;
        let output_cost = self.output_total as f64 * self.output_cost_per_1m / 1_000_000.0;
        input_cost + output_cost
    }

    /// Estimate input tokens without updating totals.
    pub fn estimate_input(&self, text: &str) -> usize {
        estimate_tokens(text)
    }

    /// Estimate output tokens without updating totals.
    pub fn estimate_output(&self, text: &str) -> usize {
        estimate_tokens(text)
    }
}

/// Build a `TokenTracker` from the provider/model registry.
pub fn token_tracker_for(provider: &str, model: &str) -> TokenTracker {
    crate::provider::find_provider(provider)
        .and_then(|p| {
            p.models
                .iter()
                .find(|m| m.name == model)
                .map(|meta| {
                    TokenTracker::with_costs(
                        meta.cost_prompt.unwrap_or(0.0),
                        meta.cost_completion.unwrap_or(0.0),
                    )
                })
        })
        .unwrap_or_default()
}
