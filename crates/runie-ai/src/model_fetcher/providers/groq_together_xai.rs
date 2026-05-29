use crate::ModelInfo;

pub fn groq() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "deepseek-r1-distill-llama-70b".to_string(), name: "DeepSeek R1 Distill Llama 70B".to_string() },
        ModelInfo { id: "gemma2-9b-it".to_string(), name: "Gemma 2 9B".to_string() },
        ModelInfo { id: "groq/compound".to_string(), name: "Compound".to_string() },
        ModelInfo { id: "groq/compound-mini".to_string(), name: "Compound Mini".to_string() },
        ModelInfo { id: "llama-3.1-8b-instant".to_string(), name: "Llama 3.1 8B Instant".to_string() },
        ModelInfo { id: "llama-3.3-70b-versatile".to_string(), name: "Llama 3.3 70B Versatile".to_string() },
        ModelInfo { id: "llama3-70b-8192".to_string(), name: "Llama 3 70B".to_string() },
        ModelInfo { id: "llama3-8b-8192".to_string(), name: "Llama 3 8B".to_string() },
        ModelInfo { id: "meta-llama/llama-4-maverick-17b-128e-instruct".to_string(), name: "Llama 4 Maverick 17B".to_string() },
        ModelInfo { id: "meta-llama/llama-4-scout-17b-16e-instruct".to_string(), name: "Llama 4 Scout 17B".to_string() },
        ModelInfo { id: "mistral-saba-24b".to_string(), name: "Mistral Saba 24B".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2-instruct".to_string(), name: "Kimi K2 Instruct".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2-instruct-0905".to_string(), name: "Kimi K2 Instruct 0905".to_string() },
        ModelInfo { id: "openai/gpt-oss-120b".to_string(), name: "GPT OSS 120B".to_string() },
        ModelInfo { id: "openai/gpt-oss-20b".to_string(), name: "GPT OSS 20B".to_string() },
        ModelInfo { id: "openai/gpt-oss-safeguard-20b".to_string(), name: "Safety GPT OSS 20B".to_string() },
        ModelInfo { id: "qwen-qwq-32b".to_string(), name: "Qwen QwQ 32B".to_string() },
        ModelInfo { id: "qwen/qwen3-32b".to_string(), name: "Qwen3 32B".to_string() },
    ]
}

pub fn together() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "MiniMaxAI/MiniMax-M2.5".to_string(), name: "MiniMax-M2.5".to_string() },
        ModelInfo { id: "MiniMaxAI/MiniMax-M2.7".to_string(), name: "MiniMax-M2.7".to_string() },
        ModelInfo { id: "Qwen/Qwen3-235B-A22B-Instruct-2507-tput".to_string(), name: "Qwen3 235B A22B Instruct 2507 FP8".to_string() },
        ModelInfo { id: "Qwen/Qwen3-Coder-480B-A35B-Instruct-FP8".to_string(), name: "Qwen3 Coder 480B A35B Instruct".to_string() },
        ModelInfo { id: "Qwen/Qwen3-Coder-Next-FP8".to_string(), name: "Qwen3 Coder Next FP8".to_string() },
        ModelInfo { id: "Qwen/Qwen3.5-397B-A17B".to_string(), name: "Qwen3.5 397B A17B".to_string() },
        ModelInfo { id: "Qwen/Qwen3.6-Plus".to_string(), name: "Qwen3.6 Plus".to_string() },
        ModelInfo { id: "Qwen/Qwen3.7-Max".to_string(), name: "Qwen3.7 Max".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V3".to_string(), name: "DeepSeek V3".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V3-1".to_string(), name: "DeepSeek V3.1".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V4-Pro".to_string(), name: "DeepSeek V4 Pro".to_string() },
        ModelInfo { id: "essentialai/Rnj-1-Instruct".to_string(), name: "Rnj-1 Instruct".to_string() },
        ModelInfo { id: "google/gemma-4-31B-it".to_string(), name: "Gemma 4 31B Instruct".to_string() },
        ModelInfo { id: "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(), name: "Llama 3.3 70B".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2.5".to_string(), name: "Kimi K2.5".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2.6".to_string(), name: "Kimi K2.6".to_string() },
        ModelInfo { id: "openai/gpt-oss-120b".to_string(), name: "GPT OSS 120B".to_string() },
        ModelInfo { id: "zai-org/GLM-5.1".to_string(), name: "GLM-5.1".to_string() },
    ]
}

pub fn xai() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "grok-3".to_string(), name: "Grok 3".to_string() },
        ModelInfo { id: "grok-3-fast".to_string(), name: "Grok 3 Fast".to_string() },
        ModelInfo { id: "grok-4.20-0309-non-reasoning".to_string(), name: "Grok 4.20 (Non-Reasoning)".to_string() },
        ModelInfo { id: "grok-4.20-0309-reasoning".to_string(), name: "Grok 4.20 (Reasoning)".to_string() },
        ModelInfo { id: "grok-4.3".to_string(), name: "Grok 4.3".to_string() },
        ModelInfo { id: "grok-build-0.1".to_string(), name: "Grok Build 0.1".to_string() },
        ModelInfo { id: "grok-code-fast-1".to_string(), name: "Grok Code Fast 1".to_string() },
    ]
}
