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

    /// Fetch models from models.dev API and merge into the database.
    /// Preserves user overrides and API-key-gated models.
    pub async fn fetch_from_models_dev(&mut self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let url = std::env::var("ANVIL_MODELS_DEV_URL")
            .unwrap_or_else(|_| "https://models.dev/api.json".to_string());

        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            return Err(format!("models.dev API returned {}", response.status()).into());
        }

        let data: serde_json::Value = response.json().await?;
        if !data.is_object() {
            return Err("models.dev response is not a JSON object".into());
        }

        // Discover which API keys are present so we can flag available models
        let has_anthropic = std::env::var("ANTHROPIC_API_KEY").is_ok();
        let has_openai = std::env::var("OPENAI_API_KEY").is_ok();
        let has_google = std::env::var("GOOGLE_API_KEY").is_ok();
        let has_deepseek = std::env::var("DEEPSEEK_API_KEY").is_ok();

        let mut added = 0;

        if let Some(providers) = data.as_object() {
            for (provider_name, provider_data) in providers {
                let Some(models_obj) = provider_data.get("models").and_then(|m| m.as_object()) else {
                    continue;
                };

                for (model_key, model_data) in models_obj {
                    let Some(model_obj) = model_data.as_object() else { continue; };

                    // Build canonical ID: provider/model-id
                    let model_id = format!("{}/{}", provider_name, model_key);

                    // Skip if already in DB (user overrides take precedence)
                    if self.models.contains_key(&model_id) {
                        continue;
                    }

                    let name = model_obj.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(model_key)
                        .to_string();

                    let context_length = model_obj.get("limit")
                        .and_then(|l| l.get("context"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    let input_cost = model_obj.get("cost")
                        .and_then(|c| c.get("input"))
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    let output_cost = model_obj.get("cost")
                        .and_then(|c| c.get("output"))
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    let vision = model_obj.get("modalities")
                        .and_then(|m| m.get("input"))
                        .and_then(|arr| arr.as_array())
                        .map(|arr| arr.iter().any(|v| v.as_str() == Some("image")))
                        .unwrap_or(false);

                    let function_calling = model_obj.get("tool_call")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // Determine availability based on known API keys
                    let is_available = match provider_name.as_str() {
                        "anthropic" => has_anthropic,
                        "openai" => has_openai,
                        "google" => has_google,
                        "deepseek" => has_deepseek,
                        "ollama" => true, // local, no key needed
                        _ => false,
                    };

                    let model = Model {
                        id: model_id.clone(),
                        name,
                        provider: provider_name.clone(),
                        context_length,
                        input_cost,
                        output_cost,
                        capabilities: ModelCapabilities {
                            vision,
                            function_calling,
                            streaming: true,
                        },
                    };

                    self.models.insert(model_id.clone(), model);
                    self.statuses.insert(model_id, ModelStatus {
                        health: if is_available { HealthLevel::Healthy } else { HealthLevel::Critical },
                        latency_ms: if is_available { 100 } else { u64::MAX },
                        spent: 0.0,
                        is_active: false,
                    });

                    added += 1;
                }
            }
        }

        // Cache to disk
        Self::cache_models(&self.models)?;

        eprintln!("[anvil] models.dev: {} new models merged ({} total)", added, self.models.len());
        Ok(added)
    }

    /// Load cached models from disk, merging with defaults.
    pub fn load_cached() -> Self {
        let cache_path = Self::cache_path();
        if cache_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cache_path) {
                if let Ok(cached) = serde_json::from_str::<HashMap<String, Model>>(&content) {
                    let mut db = Self::new();
                    for (id, model) in cached {
                        db.models.entry(id.clone()).or_insert(model);
                    }
                    eprintln!("[anvil] Loaded {} cached models from {:?}", db.models.len(), cache_path);
                    return db;
                }
            }
        }
        Self::new()
    }

    fn cache_path() -> std::path::PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".anvil/cache/models.dev.json"))
            .unwrap_or_else(|| std::path::PathBuf::from(".anvil/cache/models.dev.json"))
    }

    fn cache_models(models: &HashMap<String, Model>) -> anyhow::Result<()> {
        let path = Self::cache_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(models)?;
        std::fs::write(&path, json)?;
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
