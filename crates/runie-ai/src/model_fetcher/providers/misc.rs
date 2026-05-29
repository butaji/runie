use crate::ModelInfo;

pub fn mistral() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "codestral-latest".to_string(), name: "Codestral (latest)".to_string() },
        ModelInfo { id: "devstral-2512".to_string(), name: "Devstral 2".to_string() },
        ModelInfo { id: "devstral-medium-2507".to_string(), name: "Devstral Medium".to_string() },
        ModelInfo { id: "devstral-medium-latest".to_string(), name: "Devstral 2 (latest)".to_string() },
        ModelInfo { id: "devstral-small-2505".to_string(), name: "Devstral Small 2505".to_string() },
        ModelInfo { id: "devstral-small-2507".to_string(), name: "Devstral Small".to_string() },
        ModelInfo { id: "labs-devstral-small-2512".to_string(), name: "Devstral Small 2".to_string() },
        ModelInfo { id: "magistral-medium-latest".to_string(), name: "Magistral Medium (latest)".to_string() },
        ModelInfo { id: "magistral-small".to_string(), name: "Magistral Small".to_string() },
        ModelInfo { id: "ministral-3b-latest".to_string(), name: "Ministral 3B (latest)".to_string() },
        ModelInfo { id: "ministral-8b-latest".to_string(), name: "Ministral 8B (latest)".to_string() },
        ModelInfo { id: "mistral-large-2411".to_string(), name: "Mistral Large 2.1".to_string() },
        ModelInfo { id: "mistral-large-2512".to_string(), name: "Mistral Large 3".to_string() },
        ModelInfo { id: "mistral-large-latest".to_string(), name: "Mistral Large (latest)".to_string() },
        ModelInfo { id: "mistral-medium-2505".to_string(), name: "Mistral Medium 3".to_string() },
        ModelInfo { id: "mistral-medium-2508".to_string(), name: "Mistral Medium 3.1".to_string() },
        ModelInfo { id: "mistral-medium-2604".to_string(), name: "Mistral Medium 3.5".to_string() },
        ModelInfo { id: "mistral-medium-3.5".to_string(), name: "Mistral Medium 3.5".to_string() },
        ModelInfo { id: "mistral-medium-latest".to_string(), name: "Mistral Medium (latest)".to_string() },
        ModelInfo { id: "mistral-nemo".to_string(), name: "Mistral Nemo".to_string() },
        ModelInfo { id: "mistral-small-2506".to_string(), name: "Mistral Small 3.2".to_string() },
        ModelInfo { id: "mistral-small-2603".to_string(), name: "Mistral Small 4".to_string() },
        ModelInfo { id: "mistral-small-latest".to_string(), name: "Mistral Small (latest)".to_string() },
        ModelInfo { id: "open-mistral-7b".to_string(), name: "Mistral 7B".to_string() },
        ModelInfo { id: "open-mixtral-8x22b".to_string(), name: "Mixtral 8x22B".to_string() },
        ModelInfo { id: "open-mixtral-8x7b".to_string(), name: "Mixtral 8x7B".to_string() },
        ModelInfo { id: "pixtral-12b".to_string(), name: "Pixtral 12B".to_string() },
        ModelInfo { id: "pixtral-large-latest".to_string(), name: "Pixtral Large (latest)".to_string() },
    ]
}

pub fn deepseek() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "deepseek-v4-flash".to_string(), name: "DeepSeek V4 Flash".to_string() },
        ModelInfo { id: "deepseek-v4-pro".to_string(), name: "DeepSeek V4 Pro".to_string() },
    ]
}

pub fn minimax() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "MiniMax-M2.7".to_string(), name: "MiniMax-M2.7".to_string() },
        ModelInfo { id: "MiniMax-M2.7-highspeed".to_string(), name: "MiniMax-M2.7-highspeed".to_string() },
    ]
}

pub fn huggingface() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "MiniMaxAI/MiniMax-M2.1".to_string(), name: "MiniMax-M2.1".to_string() },
        ModelInfo { id: "MiniMaxAI/MiniMax-M2.5".to_string(), name: "MiniMax-M2.5".to_string() },
        ModelInfo { id: "MiniMaxAI/MiniMax-M2.7".to_string(), name: "MiniMax-M2.7".to_string() },
        ModelInfo { id: "Qwen/Qwen3-235B-A22B-Thinking-2507".to_string(), name: "Qwen3-235B-A22B-Thinking-2507".to_string() },
        ModelInfo { id: "Qwen/Qwen3-Coder-480B-A35B-Instruct".to_string(), name: "Qwen3-Coder-480B-A35B-Instruct".to_string() },
        ModelInfo { id: "Qwen/Qwen3-Coder-Next".to_string(), name: "Qwen3-Coder-Next".to_string() },
        ModelInfo { id: "Qwen/Qwen3-Next-80B-A3B-Instruct".to_string(), name: "Qwen3-Next-80B-A3B-Instruct".to_string() },
        ModelInfo { id: "Qwen/Qwen3-Next-80B-A3B-Thinking".to_string(), name: "Qwen3-Next-80B-A3B-Thinking".to_string() },
        ModelInfo { id: "Qwen/Qwen3.5-397B-A17B".to_string(), name: "Qwen3.5-397B-A17B".to_string() },
        ModelInfo { id: "XiaomiMiMo/MiMo-V2-Flash".to_string(), name: "MiMo-V2-Flash".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-R1-0528".to_string(), name: "DeepSeek-R1-0528".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V3.2".to_string(), name: "DeepSeek-V3.2".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V4-Pro".to_string(), name: "DeepSeek V4 Pro".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2-Instruct".to_string(), name: "Kimi-K2-Instruct".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2-Instruct-0905".to_string(), name: "Kimi-K2-Instruct-0905".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2-Thinking".to_string(), name: "Kimi-K2-Thinking".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2.5".to_string(), name: "Kimi-K2.5".to_string() },
        ModelInfo { id: "moonshotai/Kimi-K2.6".to_string(), name: "Kimi-K2.6".to_string() },
        ModelInfo { id: "zai-org/GLM-4.7".to_string(), name: "GLM-4.7".to_string() },
        ModelInfo { id: "zai-org/GLM-4.7-Flash".to_string(), name: "GLM-4.7-Flash".to_string() },
        ModelInfo { id: "zai-org/GLM-5".to_string(), name: "GLM-5".to_string() },
        ModelInfo { id: "zai-org/GLM-5.1".to_string(), name: "GLM-5.1".to_string() },
    ]
}

pub fn zai() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "glm-4.5-air".to_string(), name: "GLM-4.5-Air".to_string() },
        ModelInfo { id: "glm-4.7".to_string(), name: "GLM-4.7".to_string() },
        ModelInfo { id: "glm-5-turbo".to_string(), name: "GLM-5-Turbo".to_string() },
        ModelInfo { id: "glm-5.1".to_string(), name: "GLM-5.1".to_string() },
        ModelInfo { id: "glm-5v-turbo".to_string(), name: "GLM-5V-Turbo".to_string() },
    ]
}

pub fn google() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "gemini-1.5-pro".to_string(), name: "Gemini 1.5 Pro".to_string() },
        ModelInfo { id: "gemini-1.5-flash".to_string(), name: "Gemini 1.5 Flash".to_string() },
        ModelInfo { id: "gemini-1.5-flash-8b".to_string(), name: "Gemini 1.5 Flash-8B".to_string() },
        ModelInfo { id: "gemini-2.0-flash".to_string(), name: "Gemini 2.0 Flash".to_string() },
        ModelInfo { id: "gemini-2.0-flash-lite".to_string(), name: "Gemini 2.0 Flash Lite".to_string() },
        ModelInfo { id: "gemini-2.5-flash".to_string(), name: "Gemini 2.5 Flash".to_string() },
        ModelInfo { id: "gemini-2.5-pro".to_string(), name: "Gemini 2.5 Pro".to_string() },
        ModelInfo { id: "gemini-3.5-flash".to_string(), name: "Gemini 3.5 Flash".to_string() },
    ]
}

pub fn ollama() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "llama3".to_string(), name: "Llama 3".to_string() },
        ModelInfo { id: "llama3.1".to_string(), name: "Llama 3.1".to_string() },
        ModelInfo { id: "llama3.2".to_string(), name: "Llama 3.2".to_string() },
        ModelInfo { id: "mistral".to_string(), name: "Mistral".to_string() },
        ModelInfo { id: "mixtral".to_string(), name: "Mixtral".to_string() },
        ModelInfo { id: "codellama".to_string(), name: "Code Llama".to_string() },
        ModelInfo { id: "phi3".to_string(), name: "Phi-3".to_string() },
        ModelInfo { id: "qwen2".to_string(), name: "Qwen2".to_string() },
        ModelInfo { id: "deepseek-coder".to_string(), name: "DeepSeek Coder".to_string() },
        ModelInfo { id: "deepseek-coder-v2".to_string(), name: "DeepSeek Coder V2".to_string() },
    ]
}

pub fn azure() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string() },
        ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string() },
        ModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string() },
        ModelInfo { id: "gpt-35-turbo".to_string(), name: "GPT-3.5 Turbo".to_string() },
    ]
}

pub fn cohere() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "command-r-plus".to_string(), name: "Command R+".to_string() },
        ModelInfo { id: "command-r".to_string(), name: "Command R".to_string() },
        ModelInfo { id: "command".to_string(), name: "Command".to_string() },
        ModelInfo { id: "command-light".to_string(), name: "Command Light".to_string() },
    ]
}

pub fn mira() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "mira-chat".to_string(), name: "Mira Chat".to_string() },
        ModelInfo { id: "mira-fast".to_string(), name: "Mira Fast".to_string() },
    ]
}

pub fn galadriel() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "galadriel-chat".to_string(), name: "Galadriel Chat".to_string() },
        ModelInfo { id: "galadriel-fast".to_string(), name: "Galadriel Fast".to_string() },
    ]
}

pub fn llamafile() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "llamafile".to_string(), name: "Llamafile".to_string() },
        ModelInfo { id: "mistral".to_string(), name: "Mistral".to_string() },
        ModelInfo { id: "codellama".to_string(), name: "Code Llama".to_string() },
    ]
}

pub fn perplexity() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "sonar".to_string(), name: "Sonar".to_string() },
        ModelInfo { id: "sonar-pro".to_string(), name: "Sonar Pro".to_string() },
        ModelInfo { id: "sonar-reasoning".to_string(), name: "Sonar Reasoning".to_string() },
        ModelInfo { id: "sonar-deep-research".to_string(), name: "Sonar Deep Research".to_string() },
    ]
}

pub fn moonshot() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "kimi-k2".to_string(), name: "Kimi K2".to_string() },
        ModelInfo { id: "kimi-k2.5".to_string(), name: "Kimi K2.5".to_string() },
        ModelInfo { id: "kimi-k2.6".to_string(), name: "Kimi K2.6".to_string() },
    ]
}

pub fn hyperbolic() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "meta-llama/Llama-3.3-70B-Instruct".to_string(), name: "Llama 3.3 70B".to_string() },
        ModelInfo { id: "meta-llama/Llama-3.1-8B-Instruct".to_string(), name: "Llama 3.1 8B".to_string() },
        ModelInfo { id: "Qwen/QwQ-32B-Preview".to_string(), name: "Qwen QwQ 32B".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V3".to_string(), name: "DeepSeek V3".to_string() },
    ]
}
