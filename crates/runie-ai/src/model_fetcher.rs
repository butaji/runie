use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
}

#[async_trait]
pub trait ModelFetcher: Send + Sync {
    async fn fetch_models(&self, _api_key: &str) -> Result<Vec<ModelInfo>, FetchError>;
}

pub struct ProviderModelFetcher {
    provider_id: String,
}

impl ProviderModelFetcher {
    pub fn new(provider_id: &str, _base_url: Option<&str>) -> Self {
        Self {
            provider_id: provider_id.to_lowercase(),
        }
    }
}

#[async_trait]
impl ModelFetcher for ProviderModelFetcher {
    async fn fetch_models(&self, _api_key: &str) -> Result<Vec<ModelInfo>, FetchError> {
        get_provider_models(&self.provider_id)
            .ok_or_else(|| FetchError::UnsupportedProvider(self.provider_id.clone()))
    }
}

pub fn create_fetcher(provider_id: &str) -> Box<dyn ModelFetcher> {
    Box::new(ProviderModelFetcher::new(provider_id, None))
}

pub fn get_provider_models(provider: &str) -> Option<Vec<ModelInfo>> {
    let mut registry: HashMap<&str, Vec<ModelInfo>> = HashMap::new();

    // Auto-generated from Pi models.generated.ts
    // Do not edit manually

    // openai
    registry.insert("openai", vec![
        ModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string() },
        ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string() },
        ModelInfo { id: "gpt-4.1".to_string(), name: "GPT-4.1".to_string() },
        ModelInfo { id: "gpt-4.1-mini".to_string(), name: "GPT-4.1 mini".to_string() },
        ModelInfo { id: "gpt-4.1-nano".to_string(), name: "GPT-4.1 nano".to_string() },
        ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string() },
        ModelInfo { id: "gpt-4o-2024-05-13".to_string(), name: "GPT-4o (2024-05-13)".to_string() },
        ModelInfo { id: "gpt-4o-2024-08-06".to_string(), name: "GPT-4o (2024-08-06)".to_string() },
        ModelInfo { id: "gpt-4o-2024-11-20".to_string(), name: "GPT-4o (2024-11-20)".to_string() },
        ModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o mini".to_string() },
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
        ModelInfo { id: "o1".to_string(), name: "o1".to_string() },
        ModelInfo { id: "o1-pro".to_string(), name: "o1-pro".to_string() },
        ModelInfo { id: "o3".to_string(), name: "o3".to_string() },
        ModelInfo { id: "o3-deep-research".to_string(), name: "o3-deep-research".to_string() },
        ModelInfo { id: "o3-mini".to_string(), name: "o3-mini".to_string() },
        ModelInfo { id: "o3-pro".to_string(), name: "o3-pro".to_string() },
        ModelInfo { id: "o4-mini".to_string(), name: "o4-mini".to_string() },
        ModelInfo { id: "o4-mini-deep-research".to_string(), name: "o4-mini-deep-research".to_string() },
    ]);

    // anthropic
    registry.insert("anthropic", vec![
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
    ]);

    // groq
    registry.insert("groq", vec![
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
    ]);

    // together
    registry.insert("together", vec![
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
    ]);

    // xai
    registry.insert("xai", vec![
        ModelInfo { id: "grok-3".to_string(), name: "Grok 3".to_string() },
        ModelInfo { id: "grok-3-fast".to_string(), name: "Grok 3 Fast".to_string() },
        ModelInfo { id: "grok-4.20-0309-non-reasoning".to_string(), name: "Grok 4.20 (Non-Reasoning)".to_string() },
        ModelInfo { id: "grok-4.20-0309-reasoning".to_string(), name: "Grok 4.20 (Reasoning)".to_string() },
        ModelInfo { id: "grok-4.3".to_string(), name: "Grok 4.3".to_string() },
        ModelInfo { id: "grok-build-0.1".to_string(), name: "Grok Build 0.1".to_string() },
        ModelInfo { id: "grok-code-fast-1".to_string(), name: "Grok Code Fast 1".to_string() },
    ]);

    // mistral
    registry.insert("mistral", vec![
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
    ]);

    // deepseek
    registry.insert("deepseek", vec![
        ModelInfo { id: "deepseek-v4-flash".to_string(), name: "DeepSeek V4 Flash".to_string() },
        ModelInfo { id: "deepseek-v4-pro".to_string(), name: "DeepSeek V4 Pro".to_string() },
    ]);

    // openrouter
    registry.insert("openrouter", vec![
        ModelInfo { id: "ai21/jamba-large-1.7".to_string(), name: "AI21: Jamba Large 1.7".to_string() },
        ModelInfo { id: "alibaba/tongyi-deepresearch-30b-a3b".to_string(), name: "Tongyi DeepResearch 30B A3B".to_string() },
        ModelInfo { id: "amazon/nova-2-lite-v1".to_string(), name: "Amazon: Nova 2 Lite".to_string() },
        ModelInfo { id: "amazon/nova-lite-v1".to_string(), name: "Amazon: Nova Lite 1.0".to_string() },
        ModelInfo { id: "amazon/nova-micro-v1".to_string(), name: "Amazon: Nova Micro 1.0".to_string() },
        ModelInfo { id: "amazon/nova-premier-v1".to_string(), name: "Amazon: Nova Premier 1.0".to_string() },
        ModelInfo { id: "amazon/nova-pro-v1".to_string(), name: "Amazon: Nova Pro 1.0".to_string() },
        ModelInfo { id: "anthropic/claude-3-haiku".to_string(), name: "Anthropic: Claude 3 Haiku".to_string() },
        ModelInfo { id: "anthropic/claude-3.5-haiku".to_string(), name: "Anthropic: Claude 3.5 Haiku".to_string() },
        ModelInfo { id: "anthropic/claude-haiku-4.5".to_string(), name: "Anthropic: Claude Haiku 4.5".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4".to_string(), name: "Anthropic: Claude Opus 4".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4.1".to_string(), name: "Anthropic: Claude Opus 4.1".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4.5".to_string(), name: "Anthropic: Claude Opus 4.5".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4.6".to_string(), name: "Anthropic: Claude Opus 4.6".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4.6-fast".to_string(), name: "Anthropic: Claude Opus 4.6 (Fast)".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4.7".to_string(), name: "Anthropic: Claude Opus 4.7".to_string() },
        ModelInfo { id: "anthropic/claude-opus-4.7-fast".to_string(), name: "Anthropic: Claude Opus 4.7 (Fast)".to_string() },
        ModelInfo { id: "anthropic/claude-sonnet-4".to_string(), name: "Anthropic: Claude Sonnet 4".to_string() },
        ModelInfo { id: "anthropic/claude-sonnet-4.5".to_string(), name: "Anthropic: Claude Sonnet 4.5".to_string() },
        ModelInfo { id: "anthropic/claude-sonnet-4.6".to_string(), name: "Anthropic: Claude Sonnet 4.6".to_string() },
        ModelInfo { id: "arcee-ai/trinity-large-thinking".to_string(), name: "Arcee AI: Trinity Large Thinking".to_string() },
        ModelInfo { id: "arcee-ai/trinity-large-thinking:free".to_string(), name: "Arcee AI: Trinity Large Thinking (free)".to_string() },
        ModelInfo { id: "arcee-ai/trinity-mini".to_string(), name: "Arcee AI: Trinity Mini".to_string() },
        ModelInfo { id: "arcee-ai/virtuoso-large".to_string(), name: "Arcee AI: Virtuoso Large".to_string() },
        ModelInfo { id: "auto".to_string(), name: "Auto".to_string() },
        ModelInfo { id: "baidu/cobuddy:free".to_string(), name: "Baidu Qianfan: CoBuddy (free)".to_string() },
        ModelInfo { id: "baidu/ernie-4.5-21b-a3b".to_string(), name: "Baidu: ERNIE 4.5 21B A3B".to_string() },
        ModelInfo { id: "baidu/ernie-4.5-vl-28b-a3b".to_string(), name: "Baidu: ERNIE 4.5 VL 28B A3B".to_string() },
        ModelInfo { id: "bytedance-seed/seed-1.6".to_string(), name: "ByteDance Seed: Seed 1.6".to_string() },
        ModelInfo { id: "bytedance-seed/seed-1.6-flash".to_string(), name: "ByteDance Seed: Seed 1.6 Flash".to_string() },
        ModelInfo { id: "bytedance-seed/seed-2.0-lite".to_string(), name: "ByteDance Seed: Seed-2.0-Lite".to_string() },
        ModelInfo { id: "bytedance-seed/seed-2.0-mini".to_string(), name: "ByteDance Seed: Seed-2.0-Mini".to_string() },
        ModelInfo { id: "cohere/command-r-08-2024".to_string(), name: "Cohere: Command R (08-2024)".to_string() },
        ModelInfo { id: "cohere/command-r-plus-08-2024".to_string(), name: "Cohere: Command R+ (08-2024)".to_string() },
        ModelInfo { id: "deepseek/deepseek-chat".to_string(), name: "DeepSeek: DeepSeek V3".to_string() },
        ModelInfo { id: "deepseek/deepseek-chat-v3-0324".to_string(), name: "DeepSeek: DeepSeek V3 0324".to_string() },
        ModelInfo { id: "deepseek/deepseek-chat-v3.1".to_string(), name: "DeepSeek: DeepSeek V3.1".to_string() },
        ModelInfo { id: "deepseek/deepseek-r1".to_string(), name: "DeepSeek: R1".to_string() },
        ModelInfo { id: "deepseek/deepseek-r1-0528".to_string(), name: "DeepSeek: R1 0528".to_string() },
        ModelInfo { id: "deepseek/deepseek-v3.1-terminus".to_string(), name: "DeepSeek: DeepSeek V3.1 Terminus".to_string() },
        ModelInfo { id: "deepseek/deepseek-v3.2".to_string(), name: "DeepSeek: DeepSeek V3.2".to_string() },
        ModelInfo { id: "deepseek/deepseek-v3.2-exp".to_string(), name: "DeepSeek: DeepSeek V3.2 Exp".to_string() },
        ModelInfo { id: "deepseek/deepseek-v4-flash".to_string(), name: "DeepSeek: DeepSeek V4 Flash".to_string() },
        ModelInfo { id: "deepseek/deepseek-v4-flash:free".to_string(), name: "DeepSeek: DeepSeek V4 Flash (free)".to_string() },
        ModelInfo { id: "deepseek/deepseek-v4-pro".to_string(), name: "DeepSeek: DeepSeek V4 Pro".to_string() },
        ModelInfo { id: "essentialai/rnj-1-instruct".to_string(), name: "EssentialAI: Rnj 1 Instruct".to_string() },
        ModelInfo { id: "google/gemini-2.0-flash-001".to_string(), name: "Google: Gemini 2.0 Flash".to_string() },
        ModelInfo { id: "google/gemini-2.0-flash-lite-001".to_string(), name: "Google: Gemini 2.0 Flash Lite".to_string() },
        ModelInfo { id: "google/gemini-2.5-flash".to_string(), name: "Google: Gemini 2.5 Flash".to_string() },
        ModelInfo { id: "google/gemini-2.5-flash-lite".to_string(), name: "Google: Gemini 2.5 Flash Lite".to_string() },
        ModelInfo { id: "google/gemini-2.5-flash-lite-preview-09-2025".to_string(), name: "Google: Gemini 2.5 Flash Lite Preview 09-2025".to_string() },
        ModelInfo { id: "google/gemini-2.5-pro".to_string(), name: "Google: Gemini 2.5 Pro".to_string() },
        ModelInfo { id: "google/gemini-2.5-pro-preview".to_string(), name: "Google: Gemini 2.5 Pro Preview 06-05".to_string() },
        ModelInfo { id: "google/gemini-2.5-pro-preview-05-06".to_string(), name: "Google: Gemini 2.5 Pro Preview 05-06".to_string() },
        ModelInfo { id: "google/gemini-3-flash-preview".to_string(), name: "Google: Gemini 3 Flash Preview".to_string() },
        ModelInfo { id: "google/gemini-3.1-flash-lite".to_string(), name: "Google: Gemini 3.1 Flash Lite".to_string() },
        ModelInfo { id: "google/gemini-3.1-flash-lite-preview".to_string(), name: "Google: Gemini 3.1 Flash Lite Preview".to_string() },
        ModelInfo { id: "google/gemini-3.1-pro-preview".to_string(), name: "Google: Gemini 3.1 Pro Preview".to_string() },
        ModelInfo { id: "google/gemini-3.1-pro-preview-customtools".to_string(), name: "Google: Gemini 3.1 Pro Preview Custom Tools".to_string() },
        ModelInfo { id: "google/gemini-3.5-flash".to_string(), name: "Google: Gemini 3.5 Flash".to_string() },
        ModelInfo { id: "google/gemma-3-12b-it".to_string(), name: "Google: Gemma 3 12B".to_string() },
        ModelInfo { id: "google/gemma-3-27b-it".to_string(), name: "Google: Gemma 3 27B".to_string() },
        ModelInfo { id: "google/gemma-4-26b-a4b-it".to_string(), name: "Google: Gemma 4 26B A4B ".to_string() },
        ModelInfo { id: "google/gemma-4-26b-a4b-it:free".to_string(), name: "Google: Gemma 4 26B A4B  (free)".to_string() },
        ModelInfo { id: "google/gemma-4-31b-it".to_string(), name: "Google: Gemma 4 31B".to_string() },
        ModelInfo { id: "google/gemma-4-31b-it:free".to_string(), name: "Google: Gemma 4 31B (free)".to_string() },
        ModelInfo { id: "ibm-granite/granite-4.1-8b".to_string(), name: "IBM: Granite 4.1 8B".to_string() },
        ModelInfo { id: "inception/mercury-2".to_string(), name: "Inception: Mercury 2".to_string() },
        ModelInfo { id: "inclusionai/ling-2.6-1t".to_string(), name: "inclusionAI: Ling-2.6-1T".to_string() },
        ModelInfo { id: "inclusionai/ling-2.6-flash".to_string(), name: "inclusionAI: Ling-2.6-flash".to_string() },
        ModelInfo { id: "inclusionai/ring-2.6-1t".to_string(), name: "inclusionAI: Ring-2.6-1T".to_string() },
        ModelInfo { id: "kwaipilot/kat-coder-pro-v2".to_string(), name: "Kwaipilot: KAT-Coder-Pro V2".to_string() },
        ModelInfo { id: "meta-llama/llama-3.1-70b-instruct".to_string(), name: "Meta: Llama 3.1 70B Instruct".to_string() },
        ModelInfo { id: "meta-llama/llama-3.1-8b-instruct".to_string(), name: "Meta: Llama 3.1 8B Instruct".to_string() },
        ModelInfo { id: "meta-llama/llama-3.3-70b-instruct".to_string(), name: "Meta: Llama 3.3 70B Instruct".to_string() },
        ModelInfo { id: "meta-llama/llama-3.3-70b-instruct:free".to_string(), name: "Meta: Llama 3.3 70B Instruct (free)".to_string() },
        ModelInfo { id: "meta-llama/llama-4-scout".to_string(), name: "Meta: Llama 4 Scout".to_string() },
        ModelInfo { id: "minimax/minimax-m1".to_string(), name: "MiniMax: MiniMax M1".to_string() },
        ModelInfo { id: "minimax/minimax-m2".to_string(), name: "MiniMax: MiniMax M2".to_string() },
        ModelInfo { id: "minimax/minimax-m2.1".to_string(), name: "MiniMax: MiniMax M2.1".to_string() },
        ModelInfo { id: "minimax/minimax-m2.5".to_string(), name: "MiniMax: MiniMax M2.5".to_string() },
        ModelInfo { id: "minimax/minimax-m2.5:free".to_string(), name: "MiniMax: MiniMax M2.5 (free)".to_string() },
        ModelInfo { id: "minimax/minimax-m2.7".to_string(), name: "MiniMax: MiniMax M2.7".to_string() },
        ModelInfo { id: "mistralai/codestral-2508".to_string(), name: "Mistral: Codestral 2508".to_string() },
        ModelInfo { id: "mistralai/devstral-2512".to_string(), name: "Mistral: Devstral 2 2512".to_string() },
        ModelInfo { id: "mistralai/devstral-medium".to_string(), name: "Mistral: Devstral Medium".to_string() },
        ModelInfo { id: "mistralai/devstral-small".to_string(), name: "Mistral: Devstral Small 1.1".to_string() },
        ModelInfo { id: "mistralai/ministral-14b-2512".to_string(), name: "Mistral: Ministral 3 14B 2512".to_string() },
        ModelInfo { id: "mistralai/ministral-3b-2512".to_string(), name: "Mistral: Ministral 3 3B 2512".to_string() },
        ModelInfo { id: "mistralai/ministral-8b-2512".to_string(), name: "Mistral: Ministral 3 8B 2512".to_string() },
        ModelInfo { id: "mistralai/mistral-large".to_string(), name: "Mistral Large".to_string() },
        ModelInfo { id: "mistralai/mistral-large-2407".to_string(), name: "Mistral Large 2407".to_string() },
        ModelInfo { id: "mistralai/mistral-large-2411".to_string(), name: "Mistral: Mistral Large 3 2411".to_string() },
        ModelInfo { id: "mistralai/mistral-large-2512".to_string(), name: "Mistral: Mistral Large 3 2512".to_string() },
        ModelInfo { id: "mistralai/mistral-medium-3".to_string(), name: "Mistral: Mistral Medium 3".to_string() },
        ModelInfo { id: "mistralai/mistral-medium-3-5".to_string(), name: "Mistral: Mistral Medium 3.5".to_string() },
        ModelInfo { id: "mistralai/mistral-medium-3.1".to_string(), name: "Mistral: Mistral Medium 3.1".to_string() },
        ModelInfo { id: "mistralai/mistral-nemo".to_string(), name: "Mistral: Mistral Nemo".to_string() },
        ModelInfo { id: "mistralai/mistral-saba".to_string(), name: "Mistral: Saba".to_string() },
        ModelInfo { id: "mistralai/mistral-small-2603".to_string(), name: "Mistral: Mistral Small 4".to_string() },
        ModelInfo { id: "mistralai/mistral-small-3.2-24b-instruct".to_string(), name: "Mistral: Mistral Small 3.2 24B".to_string() },
        ModelInfo { id: "mistralai/mixtral-8x22b-instruct".to_string(), name: "Mistral: Mixtral 8x22B Instruct".to_string() },
        ModelInfo { id: "mistralai/pixtral-large-2411".to_string(), name: "Mistral: Pixtral Large 2411".to_string() },
        ModelInfo { id: "mistralai/voxtral-small-24b-2507".to_string(), name: "Mistral: Voxtral Small 24B 2507".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2".to_string(), name: "MoonshotAI: Kimi K2 0711".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2-0905".to_string(), name: "MoonshotAI: Kimi K2 0905".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2-thinking".to_string(), name: "MoonshotAI: Kimi K2 Thinking".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2.5".to_string(), name: "MoonshotAI: Kimi K2.5".to_string() },
        ModelInfo { id: "moonshotai/kimi-k2.6".to_string(), name: "MoonshotAI: Kimi K2.6".to_string() },
        ModelInfo { id: "nex-agi/deepseek-v3.1-nex-n1".to_string(), name: "Nex AGI: DeepSeek V3.1 Nex N1".to_string() },
        ModelInfo { id: "nvidia/llama-3.3-nemotron-super-49b-v1.5".to_string(), name: "NVIDIA: Llama 3.3 Nemotron Super 49B V1.5".to_string() },
        ModelInfo { id: "nvidia/nemotron-3-nano-30b-a3b".to_string(), name: "NVIDIA: Nemotron 3 Nano 30B A3B".to_string() },
        ModelInfo { id: "nvidia/nemotron-3-nano-30b-a3b:free".to_string(), name: "NVIDIA: Nemotron 3 Nano 30B A3B (free)".to_string() },
        ModelInfo { id: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free".to_string(), name: "NVIDIA: Nemotron 3 Nano Omni (free)".to_string() },
        ModelInfo { id: "nvidia/nemotron-3-super-120b-a12b".to_string(), name: "NVIDIA: Nemotron 3 Super".to_string() },
        ModelInfo { id: "nvidia/nemotron-3-super-120b-a12b:free".to_string(), name: "NVIDIA: Nemotron 3 Super (free)".to_string() },
        ModelInfo { id: "nvidia/nemotron-nano-12b-v2-vl:free".to_string(), name: "NVIDIA: Nemotron Nano 12B 2 VL (free)".to_string() },
        ModelInfo { id: "nvidia/nemotron-nano-9b-v2".to_string(), name: "NVIDIA: Nemotron Nano 9B V2".to_string() },
        ModelInfo { id: "nvidia/nemotron-nano-9b-v2:free".to_string(), name: "NVIDIA: Nemotron Nano 9B V2 (free)".to_string() },
        ModelInfo { id: "openai/gpt-3.5-turbo".to_string(), name: "OpenAI: GPT-3.5 Turbo".to_string() },
        ModelInfo { id: "openai/gpt-3.5-turbo-0613".to_string(), name: "OpenAI: GPT-3.5 Turbo (older v0613)".to_string() },
        ModelInfo { id: "openai/gpt-3.5-turbo-16k".to_string(), name: "OpenAI: GPT-3.5 Turbo 16k".to_string() },
        ModelInfo { id: "openai/gpt-4".to_string(), name: "OpenAI: GPT-4".to_string() },
        ModelInfo { id: "openai/gpt-4-0314".to_string(), name: "OpenAI: GPT-4 (older v0314)".to_string() },
        ModelInfo { id: "openai/gpt-4-1106-preview".to_string(), name: "OpenAI: GPT-4 Turbo (older v1106)".to_string() },
        ModelInfo { id: "openai/gpt-4-turbo".to_string(), name: "OpenAI: GPT-4 Turbo".to_string() },
        ModelInfo { id: "openai/gpt-4-turbo-preview".to_string(), name: "OpenAI: GPT-4 Turbo Preview".to_string() },
        ModelInfo { id: "openai/gpt-4.1".to_string(), name: "OpenAI: GPT-4.1".to_string() },
        ModelInfo { id: "openai/gpt-4.1-mini".to_string(), name: "OpenAI: GPT-4.1 Mini".to_string() },
        ModelInfo { id: "openai/gpt-4.1-nano".to_string(), name: "OpenAI: GPT-4.1 Nano".to_string() },
        ModelInfo { id: "openai/gpt-4o".to_string(), name: "OpenAI: GPT-4o".to_string() },
        ModelInfo { id: "openai/gpt-4o-2024-05-13".to_string(), name: "OpenAI: GPT-4o (2024-05-13)".to_string() },
        ModelInfo { id: "openai/gpt-4o-2024-08-06".to_string(), name: "OpenAI: GPT-4o (2024-08-06)".to_string() },
        ModelInfo { id: "openai/gpt-4o-2024-11-20".to_string(), name: "OpenAI: GPT-4o (2024-11-20)".to_string() },
        ModelInfo { id: "openai/gpt-4o-audio-preview".to_string(), name: "OpenAI: GPT-4o Audio".to_string() },
        ModelInfo { id: "openai/gpt-4o-mini".to_string(), name: "OpenAI: GPT-4o-mini".to_string() },
        ModelInfo { id: "openai/gpt-4o-mini-2024-07-18".to_string(), name: "OpenAI: GPT-4o-mini (2024-07-18)".to_string() },
        ModelInfo { id: "openai/gpt-5".to_string(), name: "OpenAI: GPT-5".to_string() },
        ModelInfo { id: "openai/gpt-5-codex".to_string(), name: "OpenAI: GPT-5 Codex".to_string() },
        ModelInfo { id: "openai/gpt-5-mini".to_string(), name: "OpenAI: GPT-5 Mini".to_string() },
        ModelInfo { id: "openai/gpt-5-nano".to_string(), name: "OpenAI: GPT-5 Nano".to_string() },
        ModelInfo { id: "openai/gpt-5-pro".to_string(), name: "OpenAI: GPT-5 Pro".to_string() },
        ModelInfo { id: "openai/gpt-5.1".to_string(), name: "OpenAI: GPT-5.1".to_string() },
        ModelInfo { id: "openai/gpt-5.1-chat".to_string(), name: "OpenAI: GPT-5.1 Chat".to_string() },
        ModelInfo { id: "openai/gpt-5.1-codex".to_string(), name: "OpenAI: GPT-5.1-Codex".to_string() },
        ModelInfo { id: "openai/gpt-5.1-codex-max".to_string(), name: "OpenAI: GPT-5.1-Codex-Max".to_string() },
        ModelInfo { id: "openai/gpt-5.1-codex-mini".to_string(), name: "OpenAI: GPT-5.1-Codex-Mini".to_string() },
        ModelInfo { id: "openai/gpt-5.2".to_string(), name: "OpenAI: GPT-5.2".to_string() },
        ModelInfo { id: "openai/gpt-5.2-chat".to_string(), name: "OpenAI: GPT-5.2 Chat".to_string() },
        ModelInfo { id: "openai/gpt-5.2-codex".to_string(), name: "OpenAI: GPT-5.2-Codex".to_string() },
        ModelInfo { id: "openai/gpt-5.2-pro".to_string(), name: "OpenAI: GPT-5.2 Pro".to_string() },
        ModelInfo { id: "openai/gpt-5.3-chat".to_string(), name: "OpenAI: GPT-5.3 Chat".to_string() },
        ModelInfo { id: "openai/gpt-5.3-codex".to_string(), name: "OpenAI: GPT-5.3-Codex".to_string() },
        ModelInfo { id: "openai/gpt-5.4".to_string(), name: "OpenAI: GPT-5.4".to_string() },
        ModelInfo { id: "openai/gpt-5.4-mini".to_string(), name: "OpenAI: GPT-5.4 Mini".to_string() },
        ModelInfo { id: "openai/gpt-5.4-nano".to_string(), name: "OpenAI: GPT-5.4 Nano".to_string() },
        ModelInfo { id: "openai/gpt-5.4-pro".to_string(), name: "OpenAI: GPT-5.4 Pro".to_string() },
        ModelInfo { id: "openai/gpt-5.5".to_string(), name: "OpenAI: GPT-5.5".to_string() },
        ModelInfo { id: "openai/gpt-5.5-pro".to_string(), name: "OpenAI: GPT-5.5 Pro".to_string() },
        ModelInfo { id: "openai/gpt-audio".to_string(), name: "OpenAI: GPT Audio".to_string() },
        ModelInfo { id: "openai/gpt-audio-mini".to_string(), name: "OpenAI: GPT Audio Mini".to_string() },
        ModelInfo { id: "openai/gpt-chat-latest".to_string(), name: "OpenAI: GPT Chat Latest".to_string() },
        ModelInfo { id: "openai/gpt-oss-120b".to_string(), name: "OpenAI: gpt-oss-120b".to_string() },
        ModelInfo { id: "openai/gpt-oss-120b:free".to_string(), name: "OpenAI: gpt-oss-120b (free)".to_string() },
        ModelInfo { id: "openai/gpt-oss-20b".to_string(), name: "OpenAI: gpt-oss-20b".to_string() },
        ModelInfo { id: "openai/gpt-oss-20b:free".to_string(), name: "OpenAI: gpt-oss-20b (free)".to_string() },
        ModelInfo { id: "openai/gpt-oss-safeguard-20b".to_string(), name: "OpenAI: gpt-oss-safeguard-20b".to_string() },
        ModelInfo { id: "openai/o1".to_string(), name: "OpenAI: o1".to_string() },
        ModelInfo { id: "openai/o3".to_string(), name: "OpenAI: o3".to_string() },
        ModelInfo { id: "openai/o3-deep-research".to_string(), name: "OpenAI: o3 Deep Research".to_string() },
        ModelInfo { id: "openai/o3-mini".to_string(), name: "OpenAI: o3 Mini".to_string() },
        ModelInfo { id: "openai/o3-mini-high".to_string(), name: "OpenAI: o3 Mini High".to_string() },
        ModelInfo { id: "openai/o3-pro".to_string(), name: "OpenAI: o3 Pro".to_string() },
        ModelInfo { id: "openai/o4-mini".to_string(), name: "OpenAI: o4 Mini".to_string() },
        ModelInfo { id: "openai/o4-mini-deep-research".to_string(), name: "OpenAI: o4 Mini Deep Research".to_string() },
        ModelInfo { id: "openai/o4-mini-high".to_string(), name: "OpenAI: o4 Mini High".to_string() },
        ModelInfo { id: "openrouter/auto".to_string(), name: "Auto Router".to_string() },
        ModelInfo { id: "openrouter/free".to_string(), name: "Free Models Router".to_string() },
        ModelInfo { id: "openrouter/owl-alpha".to_string(), name: "Owl Alpha".to_string() },
        ModelInfo { id: "poolside/laguna-m.1:free".to_string(), name: "Poolside: Laguna M.1 (free)".to_string() },
        ModelInfo { id: "poolside/laguna-xs.2:free".to_string(), name: "Poolside: Laguna XS.2 (free)".to_string() },
        ModelInfo { id: "prime-intellect/intellect-3".to_string(), name: "Prime Intellect: INTELLECT-3".to_string() },
        ModelInfo { id: "qwen/qwen-2.5-72b-instruct".to_string(), name: "Qwen2.5 72B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen-2.5-7b-instruct".to_string(), name: "Qwen: Qwen2.5 7B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen-plus".to_string(), name: "Qwen: Qwen-Plus".to_string() },
        ModelInfo { id: "qwen/qwen-plus-2025-07-28".to_string(), name: "Qwen: Qwen Plus 0728".to_string() },
        ModelInfo { id: "qwen/qwen-plus-2025-07-28:thinking".to_string(), name: "Qwen: Qwen Plus 0728 (thinking)".to_string() },
        ModelInfo { id: "qwen/qwen3-14b".to_string(), name: "Qwen: Qwen3 14B".to_string() },
        ModelInfo { id: "qwen/qwen3-235b-a22b".to_string(), name: "Qwen: Qwen3 235B A22B".to_string() },
        ModelInfo { id: "qwen/qwen3-235b-a22b-2507".to_string(), name: "Qwen: Qwen3 235B A22B Instruct 2507".to_string() },
        ModelInfo { id: "qwen/qwen3-235b-a22b-thinking-2507".to_string(), name: "Qwen: Qwen3 235B A22B Thinking 2507".to_string() },
        ModelInfo { id: "qwen/qwen3-30b-a3b".to_string(), name: "Qwen: Qwen3 30B A3B".to_string() },
        ModelInfo { id: "qwen/qwen3-30b-a3b-instruct-2507".to_string(), name: "Qwen: Qwen3 30B A3B Instruct 2507".to_string() },
        ModelInfo { id: "qwen/qwen3-30b-a3b-thinking-2507".to_string(), name: "Qwen: Qwen3 30B A3B Thinking 2507".to_string() },
        ModelInfo { id: "qwen/qwen3-32b".to_string(), name: "Qwen: Qwen3 32B".to_string() },
        ModelInfo { id: "qwen/qwen3-8b".to_string(), name: "Qwen: Qwen3 8B".to_string() },
        ModelInfo { id: "qwen/qwen3-coder".to_string(), name: "Qwen: Qwen3 Coder 480B A35B".to_string() },
        ModelInfo { id: "qwen/qwen3-coder-30b-a3b-instruct".to_string(), name: "Qwen: Qwen3 Coder 30B A3B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen3-coder-flash".to_string(), name: "Qwen: Qwen3 Coder Flash".to_string() },
        ModelInfo { id: "qwen/qwen3-coder-next".to_string(), name: "Qwen: Qwen3 Coder Next".to_string() },
        ModelInfo { id: "qwen/qwen3-coder-plus".to_string(), name: "Qwen: Qwen3 Coder Plus".to_string() },
        ModelInfo { id: "qwen/qwen3-coder:free".to_string(), name: "Qwen: Qwen3 Coder 480B A35B (free)".to_string() },
        ModelInfo { id: "qwen/qwen3-max".to_string(), name: "Qwen: Qwen3 Max".to_string() },
        ModelInfo { id: "qwen/qwen3-max-thinking".to_string(), name: "Qwen: Qwen3 Max Thinking".to_string() },
        ModelInfo { id: "qwen/qwen3-next-80b-a3b-instruct".to_string(), name: "Qwen: Qwen3 Next 80B A3B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen3-next-80b-a3b-instruct:free".to_string(), name: "Qwen: Qwen3 Next 80B A3B Instruct (free)".to_string() },
        ModelInfo { id: "qwen/qwen3-next-80b-a3b-thinking".to_string(), name: "Qwen: Qwen3 Next 80B A3B Thinking".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-235b-a22b-instruct".to_string(), name: "Qwen: Qwen3 VL 235B A22B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-235b-a22b-thinking".to_string(), name: "Qwen: Qwen3 VL 235B A22B Thinking".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-30b-a3b-instruct".to_string(), name: "Qwen: Qwen3 VL 30B A3B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-30b-a3b-thinking".to_string(), name: "Qwen: Qwen3 VL 30B A3B Thinking".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-32b-instruct".to_string(), name: "Qwen: Qwen3 VL 32B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-8b-instruct".to_string(), name: "Qwen: Qwen3 VL 8B Instruct".to_string() },
        ModelInfo { id: "qwen/qwen3-vl-8b-thinking".to_string(), name: "Qwen: Qwen3 VL 8B Thinking".to_string() },
        ModelInfo { id: "qwen/qwen3.5-122b-a10b".to_string(), name: "Qwen: Qwen3.5-122B-A10B".to_string() },
        ModelInfo { id: "qwen/qwen3.5-27b".to_string(), name: "Qwen: Qwen3.5-27B".to_string() },
        ModelInfo { id: "qwen/qwen3.5-35b-a3b".to_string(), name: "Qwen: Qwen3.5-35B-A3B".to_string() },
        ModelInfo { id: "qwen/qwen3.5-397b-a17b".to_string(), name: "Qwen: Qwen3.5 397B A17B".to_string() },
        ModelInfo { id: "qwen/qwen3.5-9b".to_string(), name: "Qwen: Qwen3.5-9B".to_string() },
        ModelInfo { id: "qwen/qwen3.5-flash-02-23".to_string(), name: "Qwen: Qwen3.5-Flash".to_string() },
        ModelInfo { id: "qwen/qwen3.5-plus-02-15".to_string(), name: "Qwen: Qwen3.5 Plus 2026-02-15".to_string() },
        ModelInfo { id: "qwen/qwen3.5-plus-20260420".to_string(), name: "Qwen: Qwen3.5 Plus 2026-04-20".to_string() },
        ModelInfo { id: "qwen/qwen3.6-27b".to_string(), name: "Qwen: Qwen3.6 27B".to_string() },
        ModelInfo { id: "qwen/qwen3.6-35b-a3b".to_string(), name: "Qwen: Qwen3.6 35B A3B".to_string() },
        ModelInfo { id: "qwen/qwen3.6-flash".to_string(), name: "Qwen: Qwen3.6 Flash".to_string() },
        ModelInfo { id: "qwen/qwen3.6-max-preview".to_string(), name: "Qwen: Qwen3.6 Max Preview".to_string() },
        ModelInfo { id: "qwen/qwen3.6-plus".to_string(), name: "Qwen: Qwen3.6 Plus".to_string() },
        ModelInfo { id: "qwen/qwen3.7-max".to_string(), name: "Qwen: Qwen3.7 Max".to_string() },
        ModelInfo { id: "rekaai/reka-edge".to_string(), name: "Reka Edge".to_string() },
        ModelInfo { id: "relace/relace-search".to_string(), name: "Relace: Relace Search".to_string() },
        ModelInfo { id: "sao10k/l3-euryale-70b".to_string(), name: "Sao10k: Llama 3 Euryale 70B v2.1".to_string() },
        ModelInfo { id: "sao10k/l3.1-euryale-70b".to_string(), name: "Sao10K: Llama 3.1 Euryale 70B v2.2".to_string() },
        ModelInfo { id: "stepfun/step-3.5-flash".to_string(), name: "StepFun: Step 3.5 Flash".to_string() },
        ModelInfo { id: "tencent/hy3-preview".to_string(), name: "Tencent: Hy3 preview".to_string() },
        ModelInfo { id: "thedrummer/rocinante-12b".to_string(), name: "TheDrummer: Rocinante 12B".to_string() },
        ModelInfo { id: "thedrummer/unslopnemo-12b".to_string(), name: "TheDrummer: UnslopNemo 12B".to_string() },
        ModelInfo { id: "upstage/solar-pro-3".to_string(), name: "Upstage: Solar Pro 3".to_string() },
        ModelInfo { id: "x-ai/grok-4.20".to_string(), name: "xAI: Grok 4.20".to_string() },
        ModelInfo { id: "x-ai/grok-4.3".to_string(), name: "xAI: Grok 4.3".to_string() },
        ModelInfo { id: "x-ai/grok-build-0.1".to_string(), name: "xAI: Grok Build 0.1".to_string() },
        ModelInfo { id: "xiaomi/mimo-v2-flash".to_string(), name: "Xiaomi: MiMo-V2-Flash".to_string() },
        ModelInfo { id: "xiaomi/mimo-v2-omni".to_string(), name: "Xiaomi: MiMo-V2-Omni".to_string() },
        ModelInfo { id: "xiaomi/mimo-v2-pro".to_string(), name: "Xiaomi: MiMo-V2-Pro".to_string() },
        ModelInfo { id: "xiaomi/mimo-v2.5".to_string(), name: "Xiaomi: MiMo-V2.5".to_string() },
        ModelInfo { id: "xiaomi/mimo-v2.5-pro".to_string(), name: "Xiaomi: MiMo-V2.5-Pro".to_string() },
        ModelInfo { id: "z-ai/glm-4-32b".to_string(), name: "Z.ai: GLM 4 32B ".to_string() },
        ModelInfo { id: "z-ai/glm-4.5".to_string(), name: "Z.ai: GLM 4.5".to_string() },
        ModelInfo { id: "z-ai/glm-4.5-air".to_string(), name: "Z.ai: GLM 4.5 Air".to_string() },
        ModelInfo { id: "z-ai/glm-4.5-air:free".to_string(), name: "Z.ai: GLM 4.5 Air (free)".to_string() },
        ModelInfo { id: "z-ai/glm-4.5v".to_string(), name: "Z.ai: GLM 4.5V".to_string() },
        ModelInfo { id: "z-ai/glm-4.6".to_string(), name: "Z.ai: GLM 4.6".to_string() },
        ModelInfo { id: "z-ai/glm-4.6v".to_string(), name: "Z.ai: GLM 4.6V".to_string() },
        ModelInfo { id: "z-ai/glm-4.7".to_string(), name: "Z.ai: GLM 4.7".to_string() },
        ModelInfo { id: "z-ai/glm-4.7-flash".to_string(), name: "Z.ai: GLM 4.7 Flash".to_string() },
        ModelInfo { id: "z-ai/glm-5".to_string(), name: "Z.ai: GLM 5".to_string() },
        ModelInfo { id: "z-ai/glm-5-turbo".to_string(), name: "Z.ai: GLM 5 Turbo".to_string() },
        ModelInfo { id: "z-ai/glm-5.1".to_string(), name: "Z.ai: GLM 5.1".to_string() },
        ModelInfo { id: "z-ai/glm-5v-turbo".to_string(), name: "Z.ai: GLM 5V Turbo".to_string() },
        ModelInfo { id: "~anthropic/claude-haiku-latest".to_string(), name: "Anthropic Claude Haiku Latest".to_string() },
        ModelInfo { id: "~anthropic/claude-opus-latest".to_string(), name: "Anthropic: Claude Opus Latest".to_string() },
        ModelInfo { id: "~anthropic/claude-sonnet-latest".to_string(), name: "Anthropic Claude Sonnet Latest".to_string() },
        ModelInfo { id: "~google/gemini-flash-latest".to_string(), name: "Google Gemini Flash Latest".to_string() },
        ModelInfo { id: "~google/gemini-pro-latest".to_string(), name: "Google Gemini Pro Latest".to_string() },
        ModelInfo { id: "~moonshotai/kimi-latest".to_string(), name: "MoonshotAI Kimi Latest".to_string() },
        ModelInfo { id: "~openai/gpt-latest".to_string(), name: "OpenAI GPT Latest".to_string() },
        ModelInfo { id: "~openai/gpt-mini-latest".to_string(), name: "OpenAI GPT Mini Latest".to_string() },
    ]);

    // minimax
    registry.insert("minimax", vec![
        ModelInfo { id: "MiniMax-M2.7".to_string(), name: "MiniMax-M2.7".to_string() },
        ModelInfo { id: "MiniMax-M2.7-highspeed".to_string(), name: "MiniMax-M2.7-highspeed".to_string() },
    ]);

    // huggingface
    registry.insert("huggingface", vec![
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
    ]);

    // zai
    registry.insert("zai", vec![
        ModelInfo { id: "glm-4.5-air".to_string(), name: "GLM-4.5-Air".to_string() },
        ModelInfo { id: "glm-4.7".to_string(), name: "GLM-4.7".to_string() },
        ModelInfo { id: "glm-5-turbo".to_string(), name: "GLM-5-Turbo".to_string() },
        ModelInfo { id: "glm-5.1".to_string(), name: "GLM-5.1".to_string() },
        ModelInfo { id: "glm-5v-turbo".to_string(), name: "GLM-5V-Turbo".to_string() },
    ]);

    // google / gemini (alias)
    registry.insert("google", vec![
        ModelInfo { id: "gemini-1.5-pro".to_string(), name: "Gemini 1.5 Pro".to_string() },
        ModelInfo { id: "gemini-1.5-flash".to_string(), name: "Gemini 1.5 Flash".to_string() },
        ModelInfo { id: "gemini-1.5-flash-8b".to_string(), name: "Gemini 1.5 Flash-8B".to_string() },
        ModelInfo { id: "gemini-2.0-flash".to_string(), name: "Gemini 2.0 Flash".to_string() },
        ModelInfo { id: "gemini-2.0-flash-lite".to_string(), name: "Gemini 2.0 Flash Lite".to_string() },
        ModelInfo { id: "gemini-2.5-flash".to_string(), name: "Gemini 2.5 Flash".to_string() },
        ModelInfo { id: "gemini-2.5-pro".to_string(), name: "Gemini 2.5 Pro".to_string() },
        ModelInfo { id: "gemini-3.5-flash".to_string(), name: "Gemini 3.5 Flash".to_string() },
    ]);
    registry.insert("gemini", registry.get("google").unwrap().clone());

    // ollama
    registry.insert("ollama", vec![
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
    ]);

    // azure
    registry.insert("azure", vec![
        ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string() },
        ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string() },
        ModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string() },
        ModelInfo { id: "gpt-35-turbo".to_string(), name: "GPT-3.5 Turbo".to_string() },
    ]);

    // cohere
    registry.insert("cohere", vec![
        ModelInfo { id: "command-r-plus".to_string(), name: "Command R+".to_string() },
        ModelInfo { id: "command-r".to_string(), name: "Command R".to_string() },
        ModelInfo { id: "command".to_string(), name: "Command".to_string() },
        ModelInfo { id: "command-light".to_string(), name: "Command Light".to_string() },
    ]);

    // mira
    registry.insert("mira", vec![
        ModelInfo { id: "mira-chat".to_string(), name: "Mira Chat".to_string() },
        ModelInfo { id: "mira-fast".to_string(), name: "Mira Fast".to_string() },
    ]);

    // galadriel
    registry.insert("galadriel", vec![
        ModelInfo { id: "galadriel-chat".to_string(), name: "Galadriel Chat".to_string() },
        ModelInfo { id: "galadriel-fast".to_string(), name: "Galadriel Fast".to_string() },
    ]);

    // llamafile
    registry.insert("llamafile", vec![
        ModelInfo { id: "llamafile".to_string(), name: "Llamafile".to_string() },
        ModelInfo { id: "mistral".to_string(), name: "Mistral".to_string() },
        ModelInfo { id: "codellama".to_string(), name: "Code Llama".to_string() },
    ]);

    // perplexity
    registry.insert("perplexity", vec![
        ModelInfo { id: "sonar".to_string(), name: "Sonar".to_string() },
        ModelInfo { id: "sonar-pro".to_string(), name: "Sonar Pro".to_string() },
        ModelInfo { id: "sonar-reasoning".to_string(), name: "Sonar Reasoning".to_string() },
        ModelInfo { id: "sonar-deep-research".to_string(), name: "Sonar Deep Research".to_string() },
    ]);

    // moonshot
    registry.insert("moonshot", vec![
        ModelInfo { id: "kimi-k2".to_string(), name: "Kimi K2".to_string() },
        ModelInfo { id: "kimi-k2.5".to_string(), name: "Kimi K2.5".to_string() },
        ModelInfo { id: "kimi-k2.6".to_string(), name: "Kimi K2.6".to_string() },
    ]);

    // hyperbolic
    registry.insert("hyperbolic", vec![
        ModelInfo { id: "meta-llama/Llama-3.3-70B-Instruct".to_string(), name: "Llama 3.3 70B".to_string() },
        ModelInfo { id: "meta-llama/Llama-3.1-8B-Instruct".to_string(), name: "Llama 3.1 8B".to_string() },
        ModelInfo { id: "Qwen/QwQ-32B-Preview".to_string(), name: "Qwen QwQ 32B".to_string() },
        ModelInfo { id: "deepseek-ai/DeepSeek-V3".to_string(), name: "DeepSeek V3".to_string() },
    ]);

    registry.get(provider).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_creation() {
        let model = ModelInfo {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
        };
        assert_eq!(model.id, "gpt-4");
        assert_eq!(model.name, "GPT-4");
    }

    #[test]
    fn test_fetch_error_display() {
        let err = FetchError::ApiError("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = FetchError::UnsupportedProvider("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = FetchError::HttpError("connection failed".to_string());
        assert!(err.to_string().contains("connection failed"));
    }

    #[test]
    fn test_create_fetcher_returns_trait_object() {
        let fetcher = create_fetcher("openai");
        let _ = fetcher;
    }

    #[test]
    fn test_get_provider_models_openai() {
        let models = get_provider_models("openai").unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "gpt-4o"));
    }

    #[test]
    fn test_get_provider_models_anthropic() {
        let models = get_provider_models("anthropic").unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "claude-3-5-sonnet-20241022"));
    }

    #[test]
    fn test_get_provider_models_unsupported() {
        assert!(get_provider_models("nonexistent").is_none());
    }

    #[test]
    fn test_model_registry_lookup_known_provider() {
        let models = get_provider_models("openai");
        assert!(models.is_some());
        let models = models.unwrap();
        assert!(!models.is_empty());
    }

    #[test]
    fn test_model_registry_lookup_unknown_provider() {
        let models = get_provider_models("nonexistent");
        assert!(models.is_none());
    }

    #[test]
    fn test_all_providers_have_models() {
        let providers = vec![
            "openai", "anthropic", "groq", "together", "xai", "mistral",
            "deepseek", "openrouter", "ollama", "minimax", "azure",
            "huggingface", "cohere", "zai", "mira", "galadriel",
            "llamafile", "perplexity", "moonshot", "hyperbolic",
        ];

        for provider in providers {
            let models = get_provider_models(provider);
            assert!(
                models.is_some(),
                "Provider {} should have models",
                provider
            );
            let models = models.unwrap();
            assert!(
                !models.is_empty(),
                "Provider {} should have non-empty models",
                provider
            );
        }
    }

    #[test]
    fn test_provider_id_case_insensitive() {
        // Test that various casings all create a fetcher (doesn't panic)
        let _ = create_fetcher("OPENAI");
        let _ = create_fetcher("OpenAI");
        let _ = create_fetcher("openai");
    }

    #[test]
    fn test_model_info_partial_eq() {
        let m1 = ModelInfo { id: "a".to_string(), name: "A".to_string() };
        let m2 = ModelInfo { id: "a".to_string(), name: "A".to_string() };
        let m3 = ModelInfo { id: "b".to_string(), name: "B".to_string() };
        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }

    #[tokio::test]
    async fn test_provider_fetcher_returns_models() {
        let fetcher = ProviderModelFetcher::new("openai", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
        let models = result.unwrap();
        assert!(!models.is_empty());
    }

    #[tokio::test]
    async fn test_provider_fetcher_unsupported_returns_error() {
        let fetcher = ProviderModelFetcher::new("nonexistent-provider", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, FetchError::UnsupportedProvider(_)));
    }

    #[tokio::test]
    async fn test_provider_fetcher_case_insensitive() {
        let fetcher = ProviderModelFetcher::new("OPENAI", None);
        let result = fetcher.fetch_models("any-key").await;
        assert!(result.is_ok());
    }
}
