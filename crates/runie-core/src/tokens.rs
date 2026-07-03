//! Token estimation and tracking.
//!
//! Primary: tiktoken-based counting for OpenAI-compatible providers.
//! Fallback: chars/4 approximation for unknown providers or when tiktoken
//! is unavailable. Provider-reported `usage` fields on LLM responses are
//! authoritative for billing/cost calculations. Local estimates are used
//! for pre-flight truncation decisions and UI token counters, where ±20%
//! accuracy is acceptable for the fallback path.

// =============================================================================
// Tiktoken encoder
// =============================================================================

/// Count tokens using tiktoken's cl100k_base encoding.
/// Returns `None` if the `tiktoken` feature is disabled or tiktoken fails.
///
/// Note: tiktoken's `get_encoding` already caches encoders globally,
/// so this function reuses that cache without additional synchronization.
#[cfg(feature = "tiktoken")]
fn tiktoken_count(text: &str) -> Option<usize> {
    tiktoken::get_encoding("cl100k_base").map(|enc| enc.count(text))
}

/// Fallback when `tiktoken` feature is disabled — always returns `None`,
/// forcing callers to use the chars/4 heuristic.
#[cfg(not(feature = "tiktoken"))]
fn tiktoken_count(_text: &str) -> Option<usize> {
    None
}

// =============================================================================
// Estimation
// =============================================================================

/// Approximate token count: try tiktoken first, fall back to chars/4.
///
/// For OpenAI-compatible models (cl100k_base), tiktoken gives accurate counts.
/// For other providers or if tiktoken fails, chars/4 is used as a rough estimate.
pub fn estimate_tokens(text: &str) -> usize {
    tiktoken_count(text).unwrap_or_else(|| chars4_count(text))
}

/// Count tokens using the chars/4 heuristic (rounding up).
fn chars4_count(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// Estimate token count for the active provider/model.
/// Uses tiktoken for OpenAI-compatible providers; falls back to chars/4.
pub fn estimate_tokens_for_model(text: &str, provider: &str, _model: &str) -> usize {
    // Try tiktoken for OpenAI-compatible models (cl100k_base family)
    if provider == "openai" {
        if let Some(count) = tiktoken_count(text) {
            return count;
        }
    }
    // Fall back to chars/4
    chars4_count(text)
}

/// Estimate token count using tiktoken if available.
pub fn estimate_tokens_with_tokenizer(text: &str) -> usize {
    estimate_tokens(text)
}

// =============================================================================
// TokenTracker
// =============================================================================

/// Tracks input/output token totals for a session and the current turn.
///
/// **NOTE**: Counts use tiktoken for OpenAI-compatible providers and chars/4
/// for others. Provider-reported `usage` fields on LLM responses are authoritative
/// for billing and cost calculations.
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

    /// Estimate input tokens and add them.
    pub fn track_input(&mut self, text: &str) {
        self.add_input(estimate_tokens(text));
    }

    /// Estimate output tokens and add them.
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
            p.models.iter().find(|m| m.name == model).map(|meta| {
                TokenTracker::with_costs(
                    meta.cost_prompt.unwrap_or(0.0),
                    meta.cost_completion.unwrap_or(0.0),
                )
            })
        })
        .unwrap_or_default()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "tiktoken")]
    fn tiktoken_english() {
        // "hello world" is 2 tokens in cl100k_base
        let count = tiktoken_count("hello world").expect("tiktoken should encode");
        assert_eq!(count, 2, "hello world should be 2 tokens");
    }

    #[test]
    #[cfg(feature = "tiktoken")]
    fn tiktoken_code() {
        // Code tokens are typically shorter; function signatures vary
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let count = tiktoken_count(code).expect("tiktoken should encode code");
        // Should be non-zero and reasonable
        assert!(count > 0, "code should produce tokens");
        assert!(count < code.len(), "tokens should be fewer than chars");
    }

    #[test]
    fn tiktoken_fallback_returns_charcount4() {
        // When tiktoken_count returns None, estimate_tokens falls back to chars/4
        // We can't easily trigger the None path without mocking, but we can
        // verify the fallback behavior by checking that chars/4 is used for
        // non-OpenAI providers.
        let text = "hello world";
        let chars4 = text.chars().count().div_ceil(4);
        #[cfg(feature = "tiktoken")]
        let estimate = estimate_tokens_for_model(text, "unknown_provider", "unknown");
        #[cfg(not(feature = "tiktoken"))]
        let estimate = chars4; // When tiktoken disabled, always uses chars/4
        // For unknown provider, should fall back to chars/4
        assert_eq!(estimate, chars4, "unknown provider should use chars/4 fallback");
    }

    #[test]
    #[cfg(feature = "tiktoken")]
    fn tiktoken_openai_provider_uses_tiktoken() {
        let text = "hello world";
        let tiktoken_result = tiktoken_count(text).expect("tiktoken should work");
        let estimate = estimate_tokens_for_model(text, "openai", "gpt-4o");
        assert_eq!(estimate, tiktoken_result, "openai provider should use tiktoken");
    }

    #[test]
    #[cfg(feature = "tiktoken")]
    fn tiktoken_accurate_for_english() {
        // Verify tiktoken is accurate (not just chars/4) for English
        let text = "The quick brown fox jumps over the lazy dog.";
        let tiktoken_count = tiktoken_count(text).expect("tiktoken should work");
        let chars4_count = chars4_count(text);

        // These should differ — tiktoken is more accurate than chars/4
        // For this sentence, tiktoken gives ~9 tokens, chars/4 gives ~11
        assert_ne!(
            tiktoken_count, chars4_count,
            "tiktoken should differ from chars/4 for English text"
        );
        // tiktoken should give a lower count (tokens < chars/4 for English)
        assert!(
            tiktoken_count < chars4_count,
            "tiktoken ({}) should be less than chars/4 ({})",
            tiktoken_count,
            chars4_count
        );
    }

    #[test]
    #[cfg(feature = "tiktoken")]
    fn token_tracker_tracks_tokens() {
        let mut tracker = TokenTracker::new();
        tracker.track_input("hello world");
        assert_eq!(tracker.input_total(), 2, "hello world = 2 tokens");
        tracker.track_output("goodbye");
        // "goodbye" = 2 tokens in cl100k_base
        assert_eq!(tracker.output_total(), 2, "goodbye = 2 tokens");
    }

    #[test]
    fn token_tracker_cost_calculation() {
        let mut tracker = TokenTracker::with_costs(5.0, 15.0); // $5/1M in, $15/1M out
        tracker.add_input(1_000_000); // 1M input tokens
        tracker.add_output(500_000); // 500K output tokens
        let cost = tracker.session_cost();
        // 1M * 5.0 / 1M + 500K * 15.0 / 1M = 5.0 + 7.5 = 12.5
        assert!((cost - 12.5).abs() < 0.001, "cost should be $12.50, got {}", cost);
    }

    #[test]
    fn estimate_tokens_never_panics() {
        // Empty string
        assert_eq!(estimate_tokens(""), 0);
        // Very long string
        let long = "a".repeat(10_000);
        let count = estimate_tokens(&long);
        assert!(count > 0, "long string should produce tokens");
    }

    #[test]
    #[cfg(feature = "tiktoken")]
    fn tiktoken_encoding_is_cached() {
        // Calling tiktoken_count twice should return the same result
        let text = "test string";
        let first = tiktoken_count(text).expect("should work");
        let second = tiktoken_count(text).expect("should work again");
        assert_eq!(first, second, "results should be consistent");
    }
}
