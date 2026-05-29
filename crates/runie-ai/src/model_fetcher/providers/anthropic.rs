use crate::ModelInfo;

pub fn anthropic() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "claude-3-5-haiku-20241022".to_string(), name: "Claude Haiku 3.5".to_string() },
        ModelInfo { id: "claude-3-5-haiku-latest".to_string(), name: "Claude Haiku 3.5 (latest)".to_string() },
        ModelInfo { id: "claude-3-5-sonnet-20240620".to_string(), name: "Claude Sonnet 3.5".to_string() },
        ModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), name: "Claude Sonnet 3.5 v2".to_string() },
        ModelInfo { id: "claude-3-7-sonnet-20250219".to_string(), name: "Claude Sonnet 3.7".to_string() },
        ModelInfo { id: "claude-3-haiku-20240307".to_string(), name: "Claude Haiku 3".to_string() },
        ModelInfo { id: "claude-3-opus-20240229".to_string(), name: "Claude Opus 3".to_string() },
        ModelInfo { id: "claude-3-sonnet-20240229".to_string(), name: "Claude Sonnet 3".to_string() },
        ModelInfo { id: "claude-haiku-4-5".to_string(), name: "Claude Haiku 4.5 (latest)".to_string() },
        ModelInfo { id: "claude-haiku-4-5-20251001".to_string(), name: "Claude Haiku 4.5".to_string() },
        ModelInfo { id: "claude-opus-4-0".to_string(), name: "Claude Opus 4 (latest)".to_string() },
        ModelInfo { id: "claude-opus-4-1".to_string(), name: "Claude Opus 4.1 (latest)".to_string() },
        ModelInfo { id: "claude-opus-4-1-20250805".to_string(), name: "Claude Opus 4.1".to_string() },
        ModelInfo { id: "claude-opus-4-20250514".to_string(), name: "Claude Opus 4".to_string() },
        ModelInfo { id: "claude-opus-4-5".to_string(), name: "Claude Opus 4.5 (latest)".to_string() },
        ModelInfo { id: "claude-opus-4-5-20251101".to_string(), name: "Claude Opus 4.5".to_string() },
        ModelInfo { id: "claude-opus-4-6".to_string(), name: "Claude Opus 4.6".to_string() },
        ModelInfo { id: "claude-opus-4-7".to_string(), name: "Claude Opus 4.7".to_string() },
        ModelInfo { id: "claude-sonnet-4-0".to_string(), name: "Claude Sonnet 4 (latest)".to_string() },
        ModelInfo { id: "claude-sonnet-4-20250514".to_string(), name: "Claude Sonnet 4".to_string() },
        ModelInfo { id: "claude-sonnet-4-5".to_string(), name: "Claude Sonnet 4.5 (latest)".to_string() },
        ModelInfo { id: "claude-sonnet-4-5-20250929".to_string(), name: "Claude Sonnet 4.5".to_string() },
        ModelInfo { id: "claude-sonnet-4-6".to_string(), name: "Claude Sonnet 4.6".to_string() },
    ]
}
