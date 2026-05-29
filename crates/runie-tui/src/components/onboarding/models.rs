// ============================================================================
// Model Definitions per Provider
// ============================================================================

use super::ModelOption;

// ─── Model Lists ─────────────────────────────────────────────────────────────

pub fn get_openai_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption {
            name: "GPT-4o".to_string(),
            id: "gpt-4o".to_string(),
            description: "Most capable, multimodal flagship model".to_string(),
        },
        ModelOption {
            name: "GPT-4o Mini".to_string(),
            id: "gpt-4o-mini".to_string(),
            description: "Fast, affordable small model".to_string(),
        },
        ModelOption {
            name: "O1 Mini".to_string(),
            id: "o1-mini".to_string(),
            description: "Reasoning model optimized for code".to_string(),
        },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_anthropic_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption {
            name: "Claude Sonnet 4".to_string(),
            id: "claude-sonnet-4".to_string(),
            description: "Balanced performance and intelligence".to_string(),
        },
        ModelOption {
            name: "Claude Haiku".to_string(),
            id: "claude-haiku".to_string(),
            description: "Fast, lightweight for simple tasks".to_string(),
        },
        ModelOption {
            name: "Claude Opus".to_string(),
            id: "claude-opus".to_string(),
            description: "Most capable model for complex tasks".to_string(),
        },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_google_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption {
            name: "Gemini Pro".to_string(),
            id: "gemini-pro".to_string(),
            description: "Balanced multimodal model".to_string(),
        },
        ModelOption {
            name: "Gemini Flash".to_string(),
            id: "gemini-flash".to_string(),
            description: "Fast, efficient for high-volume tasks".to_string(),
        },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_cohere_models() -> Vec<ModelOption> {
    let mut models = vec![
        ModelOption {
            name: "Command R".to_string(),
            id: "command-r".to_string(),
            description: "High quality RAG-optimized model".to_string(),
        },
        ModelOption {
            name: "Command R Plus".to_string(),
            id: "command-r-plus".to_string(),
            description: "Most capable model for complex tasks".to_string(),
        },
    ];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_mistral_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Mistral Large".to_string(),
        id: "mistral-large-latest".to_string(),
        description: "Flagship model for complex reasoning".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_deepseek_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "DeepSeek Chat".to_string(),
        id: "deepseek-chat".to_string(),
        description: "Efficient conversational model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_groq_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llama 3.1 8B Instant".to_string(),
        id: "llama-3.1-8b-instant".to_string(),
        description: "Ultra-fast inference at low cost".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_openrouter_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "GPT-4o".to_string(),
        id: "openai/gpt-4o".to_string(),
        description: "Most capable multimodal model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_huggingface_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llama 2 70B".to_string(),
        id: "meta-llama/Llama-2-70b-chat-hf".to_string(),
        description: "70B parameter open model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_xai_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Grok Beta".to_string(),
        id: "grok-beta".to_string(),
        description: "Real-time knowledge and reasoning".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_azure_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "GPT-4o".to_string(),
        id: "gpt-4o".to_string(),
        description: "Most capable multimodal model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_moonshot_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Moonshot V1 8K".to_string(),
        id: "moonshot-v1-8k".to_string(),
        description: "Long context conversational model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_perplexity_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llama 3.1 Sonar Large".to_string(),
        id: "llama-3.1-sonar-large-128k-online".to_string(),
        description: "Online search-augmented model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_ollama_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llama 3.2".to_string(),
        id: "llama3.2".to_string(),
        description: "Latest open-source model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_hyperbolic_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llama 3.1 70B".to_string(),
        id: "meta-llama/Meta-Llama-3.1-70B-Instruct".to_string(),
        description: "High-quality open-source model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_together_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llama 3.2 3B Turbo".to_string(),
        id: "meta-llama/Llama-3.2-3B-Instruct-Turbo".to_string(),
        description: "Fast, efficient instruction model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_zai_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Default".to_string(),
        id: "default-model".to_string(),
        description: "Default model for Zai".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_minimax_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "ABAB 6.5".to_string(),
        id: "abab6.5-chat".to_string(),
        description: "MiniMax chat model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_mira_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Default".to_string(),
        id: "default-model".to_string(),
        description: "Default model for Mira".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_galadriel_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Default".to_string(),
        id: "default-model".to_string(),
        description: "Default model for Galadriel".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

pub fn get_llamafile_models() -> Vec<ModelOption> {
    let mut models = vec![ModelOption {
        name: "Llamafile".to_string(),
        id: "llamafile".to_string(),
        description: "Local llamafile model".to_string(),
    }];
    models.sort_by(|a, b| a.name.cmp(&b.name));
    models
}

// ─── Aggregated Lists ────────────────────────────────────────────────────────

/// Returns all model names from all providers, sorted alphabetically
pub fn get_all_model_names() -> Vec<String> {
    let all_models: Vec<String> = [
        get_openai_models(),
        get_anthropic_models(),
        get_google_models(),
        get_cohere_models(),
        get_mistral_models(),
        get_deepseek_models(),
        get_groq_models(),
        get_openrouter_models(),
        get_huggingface_models(),
        get_xai_models(),
        get_azure_models(),
        get_moonshot_models(),
        get_perplexity_models(),
        get_ollama_models(),
        get_hyperbolic_models(),
        get_together_models(),
        get_zai_models(),
        get_minimax_models(),
        get_mira_models(),
        get_galadriel_models(),
        get_llamafile_models(),
    ]
    .iter()
    .flat_map(|models| models.iter().map(|m| m.name.clone()))
    .collect();

    let mut sorted = all_models;
    sorted.sort();
    sorted.dedup();
    sorted
}
