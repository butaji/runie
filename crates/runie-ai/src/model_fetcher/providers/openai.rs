use crate::ModelInfo;

fn gpt4_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string() },
        ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string() },
        ModelInfo { id: "gpt-4.1".to_string(), name: "GPT-4.1".to_string() },
        ModelInfo { id: "gpt-4.1-mini".to_string(), name: "GPT-4.1 mini".to_string() },
        ModelInfo { id: "gpt-4.1-nano".to_string(), name: "GPT-4.1 nano".to_string() },
    ]
}

fn gpt4o_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string() },
        ModelInfo { id: "gpt-4o-2024-05-13".to_string(), name: "GPT-4o (2024-05-13)".to_string() },
        ModelInfo { id: "gpt-4o-2024-08-06".to_string(), name: "GPT-4o (2024-08-06)".to_string() },
        ModelInfo { id: "gpt-4o-2024-11-20".to_string(), name: "GPT-4o (2024-11-20)".to_string() },
        ModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o mini".to_string() },
    ]
}

fn gpt5_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "gpt-5".to_string(), name: "GPT-5".to_string() },
        ModelInfo { id: "gpt-5-chat-latest".to_string(), name: "GPT-5 Chat Latest".to_string() },
        ModelInfo { id: "gpt-5-codex".to_string(), name: "GPT-5-Codex".to_string() },
        ModelInfo { id: "gpt-5-mini".to_string(), name: "GPT-5 Mini".to_string() },
        ModelInfo { id: "gpt-5-nano".to_string(), name: "GPT-5 Nano".to_string() },
        ModelInfo { id: "gpt-5-pro".to_string(), name: "GPT-5 Pro".to_string() },
        ModelInfo { id: "gpt-5.1".to_string(), name: "GPT-5.1".to_string() },
        ModelInfo { id: "gpt-5.1-chat-latest".to_string(), name: "GPT-5.1 Chat".to_string() },
        ModelInfo { id: "gpt-5.1-codex".to_string(), name: "GPT-5.1 Codex".to_string() },
        ModelInfo { id: "gpt-5.1-codex-max".to_string(), name: "GPT-5.1 Codex Max".to_string() },
        ModelInfo { id: "gpt-5.1-codex-mini".to_string(), name: "GPT-5.1 Codex mini".to_string() },
        ModelInfo { id: "gpt-5.2".to_string(), name: "GPT-5.2".to_string() },
        ModelInfo { id: "gpt-5.2-chat-latest".to_string(), name: "GPT-5.2 Chat".to_string() },
        ModelInfo { id: "gpt-5.2-codex".to_string(), name: "GPT-5.2 Codex".to_string() },
        ModelInfo { id: "gpt-5.2-pro".to_string(), name: "GPT-5.2 Pro".to_string() },
        ModelInfo { id: "gpt-5.3-chat-latest".to_string(), name: "GPT-5.3 Chat (latest)".to_string() },
        ModelInfo { id: "gpt-5.3-codex".to_string(), name: "GPT-5.3 Codex".to_string() },
        ModelInfo { id: "gpt-5.3-codex-spark".to_string(), name: "GPT-5.3 Codex Spark".to_string() },
        ModelInfo { id: "gpt-5.4".to_string(), name: "GPT-5.4".to_string() },
        ModelInfo { id: "gpt-5.4-mini".to_string(), name: "GPT-5.4 mini".to_string() },
        ModelInfo { id: "gpt-5.4-nano".to_string(), name: "GPT-5.4 nano".to_string() },
        ModelInfo { id: "gpt-5.4-pro".to_string(), name: "GPT-5.4 Pro".to_string() },
        ModelInfo { id: "gpt-5.5".to_string(), name: "GPT-5.5".to_string() },
        ModelInfo { id: "gpt-5.5-pro".to_string(), name: "GPT-5.5 Pro".to_string() },
    ]
}

fn o_series_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "o1".to_string(), name: "o1".to_string() },
        ModelInfo { id: "o1-pro".to_string(), name: "o1-pro".to_string() },
        ModelInfo { id: "o3".to_string(), name: "o3".to_string() },
        ModelInfo { id: "o3-deep-research".to_string(), name: "o3-deep-research".to_string() },
        ModelInfo { id: "o3-mini".to_string(), name: "o3-mini".to_string() },
        ModelInfo { id: "o3-pro".to_string(), name: "o3-pro".to_string() },
        ModelInfo { id: "o4-mini".to_string(), name: "o4-mini".to_string() },
        ModelInfo { id: "o4-mini-deep-research".to_string(), name: "o4-mini-deep-research".to_string() },
    ]
}

pub fn openai() -> Vec<ModelInfo> {
    let mut models = Vec::new();
    models.extend(gpt4_models());
    models.extend(gpt4o_models());
    models.extend(gpt5_models());
    models.extend(o_series_models());
    models
}
