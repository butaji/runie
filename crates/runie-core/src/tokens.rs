//! Token estimation and tracking.
//!
//! `estimate_tokens` provides the legacy chars/4 approximation. Use
//! `estimate_tokens_with_tokenizer` when a model-specific tokenizer is known.

/// Tokenizer selection for token estimation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Tokenizer {
    /// A tiktoken encoding name, e.g. `"cl100k_base"` or `"o200k_base"`.
    Tiktoken(String),
    /// The legacy chars/4 approximation.
    #[default]
    Approximate,
}

impl Tokenizer {
    /// Convenience constructor for a named tiktoken tokenizer.
    pub fn tiktoken(name: impl Into<String>) -> Self {
        Tokenizer::Tiktoken(name.into())
    }
}

/// Approximate token count: one token per four characters, rounding up.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Estimate token count for the active provider/model, falling back to the
/// chars/4 approximation for unknown models or tokenizer initialization errors.
pub fn estimate_tokens_for_model(text: &str, provider: &str, model: &str) -> usize {
    let tracker = token_tracker_for(provider, model);
    estimate_tokens_with_tokenizer(text, tracker.tokenizer.clone())
}

/// Estimate token count using the requested tokenizer, falling back to the
/// chars/4 approximation for unknown tokenizer names or initialization errors.
pub fn estimate_tokens_with_tokenizer(text: &str, tokenizer: Tokenizer) -> usize {
    match tokenizer {
        Tokenizer::Tiktoken(name) => match name.as_str() {
            "cl100k_base" => tiktoken_count(text, tiktoken_rs::cl100k_base),
            "o200k_base" => tiktoken_count(text, tiktoken_rs::o200k_base),
            _ => estimate_tokens(text),
        },
        Tokenizer::Approximate => estimate_tokens(text),
    }
}

fn tiktoken_count<F>(text: &str, init: F) -> usize
where
    F: FnOnce() -> Result<tiktoken_rs::CoreBPE, anyhow::Error>,
{
    init()
        .map(|bpe| bpe.encode_with_special_tokens(text).len())
        .unwrap_or_else(|_| estimate_tokens(text))
}

/// Tracks input/output token totals for a session and the current turn.
#[derive(Debug, Clone, Default)]
pub struct TokenTracker {
    input_total: usize,
    output_total: usize,
    turn_input: usize,
    turn_output: usize,
    input_cost_per_1m: f64,
    output_cost_per_1m: f64,
    tokenizer: Tokenizer,
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

    pub fn with_tokenizer(mut self, tokenizer: Tokenizer) -> Self {
        self.tokenizer = tokenizer;
        self
    }

    pub fn add_input(&mut self, tokens: usize) {
        self.input_total += tokens;
        self.turn_input += tokens;
    }

    pub fn add_output(&mut self, tokens: usize) {
        self.output_total += tokens;
        self.turn_output += tokens;
    }

    /// Estimate input tokens with the configured tokenizer and add them.
    pub fn track_input(&mut self, text: &str) {
        let tokens = estimate_tokens_with_tokenizer(text, self.tokenizer.clone());
        self.add_input(tokens);
    }

    /// Estimate output tokens with the configured tokenizer and add them.
    pub fn track_output(&mut self, text: &str) {
        let tokens = estimate_tokens_with_tokenizer(text, self.tokenizer.clone());
        self.add_output(tokens);
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
        estimate_tokens_with_tokenizer(text, self.tokenizer.clone())
    }

    /// Estimate output tokens without updating totals.
    pub fn estimate_output(&self, text: &str) -> usize {
        estimate_tokens_with_tokenizer(text, self.tokenizer.clone())
    }

    /// The tokenizer used for estimation.
    pub fn tokenizer(&self) -> &Tokenizer {
        &self.tokenizer
    }
}

/// Build a `TokenTracker` configured from the provider/model registry.
/// Falls back to the chars/4 approximation and zero costs for unknown models.
pub fn token_tracker_for(provider: &str, model: &str) -> TokenTracker {
    crate::provider::find_provider(provider)
        .and_then(|p| p.models.iter().find(|m| m.name == model))
        .map(|meta| {
            let mut tracker = TokenTracker::with_costs(
                meta.cost_prompt.unwrap_or(0.0),
                meta.cost_completion.unwrap_or(0.0),
            );
            if let Some(name) = meta.tokenizer {
                tracker = tracker.with_tokenizer(Tokenizer::tiktoken(name));
            }
            tracker
        })
        .unwrap_or_default()
}
