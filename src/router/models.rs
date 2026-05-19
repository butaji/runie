use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: usize,
    pub input_cost: f64,  // per million tokens
    pub output_cost: f64, // per million tokens
    pub capabilities: ModelCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub function_calling: bool,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub health: HealthLevel,
    pub latency_ms: u64,
    pub spent: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthLevel {
    Healthy,    // ●●●●●
    Good,       // ●●●●○
    Degraded,   // ●●●○○
    Critical,   // ●●○○○
}

impl HealthLevel {
    pub fn dots(&self) -> &'static str {
        match self {
            HealthLevel::Healthy => "●●●●●",
            HealthLevel::Good => "●●●●○",
            HealthLevel::Degraded => "●●●○○",
            HealthLevel::Critical => "●●○○○",
        }
    }
}

pub struct ModelDatabase {
    pub models: HashMap<String, Model>,
    pub statuses: HashMap<String, ModelStatus>,
}

impl ModelDatabase {
    pub fn new() -> Self {
        let models = Self::default_models();
        let statuses = models.keys()
            .map(|k| (k.clone(), ModelStatus {
                health: HealthLevel::Healthy,
                latency_ms: 50 + (k.len() as u64 * 10),
                spent: 0.0,
                is_active: k.contains("claude") || k.contains("gpt"),
            }))
            .collect();

        Self { models, statuses }
    }

    fn default_models() -> HashMap<String, Model> {
        let mut models = HashMap::new();
        
        models.insert("anthropic/claude-sonnet-4".to_string(), Model {
            id: "anthropic/claude-sonnet-4".to_string(),
            name: "Claude Sonnet 4".to_string(),
            provider: "anthropic".to_string(),
            context_length: 200_000,
            input_cost: 3.00,
            output_cost: 15.00,
            capabilities: ModelCapabilities {
                vision: true,
                function_calling: true,
                streaming: true,
            },
        });

        models.insert("openai/gpt-4o".to_string(), Model {
            id: "openai/gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            provider: "openai".to_string(),
            context_length: 128_000,
            input_cost: 2.50,
            output_cost: 10.00,
            capabilities: ModelCapabilities {
                vision: true,
                function_calling: true,
                streaming: true,
            },
        });

        models.insert("ollama/llama3.3".to_string(), Model {
            id: "ollama/llama3.3".to_string(),
            name: "Llama 3.3".to_string(),
            provider: "ollama".to_string(),
            context_length: 8_000,
            input_cost: 0.0,
            output_cost: 0.0,
            capabilities: ModelCapabilities {
                vision: false,
                function_calling: false,
                streaming: true,
            },
        });

        models.insert("google/gemini-1.5-pro".to_string(), Model {
            id: "google/gemini-1.5-pro".to_string(),
            name: "Gemini 1.5 Pro".to_string(),
            provider: "google".to_string(),
            context_length: 1_000_000,
            input_cost: 1.25,
            output_cost: 5.00,
            capabilities: ModelCapabilities {
                vision: true,
                function_calling: true,
                streaming: true,
            },
        });

        models.insert("deepseek/deepseek-v3".to_string(), Model {
            id: "deepseek/deepseek-v3".to_string(),
            name: "DeepSeek V3".to_string(),
            provider: "deepseek".to_string(),
            context_length: 64_000,
            input_cost: 0.27,
            output_cost: 1.10,
            capabilities: ModelCapabilities {
                vision: false,
                function_calling: true,
                streaming: true,
            },
        });

        models
    }

    pub async fn fetch_from_models_dev(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Try to fetch from models.dev
        let response = reqwest::get("https://models.dev/api.json").await?;
        
        if response.status().is_success() {
            let _data: serde_json::Value = response.json().await?;
            // Parse and update models from API
            // For now, we'll just use our defaults
            eprintln!("Successfully fetched models.dev API");
        }
        
        Ok(())
    }

    pub fn track_spend(&mut self, model_id: &str, amount: f64) {
        if let Some(status) = self.statuses.get_mut(model_id) {
            status.spent += amount;
        }
    }

    pub fn total_spent(&self) -> f64 {
        self.statuses.values().map(|s| s.spent).sum()
    }

    pub fn active_models(&self) -> Vec<(&String, &Model, &ModelStatus)> {
        self.models.iter()
            .filter(|(id, _)| self.statuses.get(*id).map(|s| s.is_active).unwrap_or(false))
            .filter_map(|(id, model)| {
                self.statuses.get(id).map(|status| (id, model, status))
            })
            .collect()
    }
}

impl Default for ModelDatabase {
    fn default() -> Self {
        Self::new()
    }
}
