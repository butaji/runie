use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub estimated_cost: f64,
}

impl TokenUsage {
    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
        self.estimated_cost += other.estimated_cost;
    }

    pub fn estimate_cost(prompt_tokens: usize, completion_tokens: usize, model: &str) -> f64 {
        match model {
            "gpt-4o" | "gpt-4o-2024-08-06" | "o1-preview" | "o1-mini" => {
                (prompt_tokens as f64 / 1000.0 * 0.005) +
                (completion_tokens as f64 / 1000.0 * 0.015)
            }
            "gpt-4" | "gpt-4-turbo" | "gpt-4-turbo-2024-04-09" => {
                (prompt_tokens as f64 / 1000.0 * 0.03) +
                (completion_tokens as f64 / 1000.0 * 0.06)
            }
            "gpt-3.5-turbo" => {
                (prompt_tokens as f64 / 1000.0 * 0.0005) +
                (completion_tokens as f64 / 1000.0 * 0.0015)
            }
            "claude-3-5-sonnet" | "claude-3-5-sonnet-20241022" | "claude-sonnet-4-20250514" => {
                (prompt_tokens as f64 / 1000.0 * 0.003) +
                (completion_tokens as f64 / 1000.0 * 0.015)
            }
            "claude-3-opus" | "claude-opus-4-20250514" => {
                (prompt_tokens as f64 / 1000.0 * 0.015) +
                (completion_tokens as f64 / 1000.0 * 0.075)
            }
            "claude-3-haiku" | "claude-haiku-4-20250514" => {
                (prompt_tokens as f64 / 1000.0 * 0.0003) +
                (completion_tokens as f64 / 1000.0 * 0.00125)
            }
            _ => 0.0,
        }
    }

    pub fn estimate_from_text(prompt_text: &str, completion_text: &str) -> (usize, usize) {
        let prompt_tokens = (prompt_text.len() / 4).max(1);
        let completion_tokens = (completion_text.len() / 4).max(1);
        (prompt_tokens, completion_tokens)
    }
}
