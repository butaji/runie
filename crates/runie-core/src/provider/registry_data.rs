//! Static provider/model data table.
//!
//! This file contains only the static data definitions and is intentionally
//! kept separate to keep registry.rs under the 500-line limit.

use super::{ModelMeta, ProviderMeta};

/// Mock provider entry — included in `known_providers()` only when
/// `is_mock_enabled()` returns true.
pub(super) const MOCK_PROVIDER: ProviderMeta = ProviderMeta::new(
    "mock",
    "Mock (dev only)",
    "http://localhost/mock",
    "",
    &[ModelMeta::new("echo")],
);

/// Static provider/model data table.
pub(super) static KNOWN_PROVIDERS: &[ProviderMeta] = &[
    ProviderMeta::new(
        "anthropic",
        "Anthropic",
        "https://api.anthropic.com/v1",
        "ANTHROPIC_API_KEY",
        &[
            ModelMeta::new("claude-sonnet-4-6")
                .with_cost(3.0, 15.0)
                .with_thinking()
                .with_vision()
                .with_context_window(200_000),
            ModelMeta::new("claude-opus-4-7")
                .with_cost(15.0, 75.0)
                .with_thinking()
                .with_vision()
                .with_context_window(200_000),
            ModelMeta::new("claude-haiku-4-5")
                .with_cost(0.25, 1.25)
                .with_vision()
                .with_context_window(200_000),
        ],
    ),
    ProviderMeta::new(
        "openai",
        "OpenAI",
        "https://api.openai.com/v1",
        "OPENAI_API_KEY",
        &[
            ModelMeta::new("gpt-4o")
                .with_cost(5.0, 15.0)
                .with_vision()
                .with_tokenizer("o200k_base")
                .with_context_window(128_000),
            ModelMeta::new("gpt-4o-mini")
                .with_cost(0.15, 0.6)
                .with_vision()
                .with_tokenizer("o200k_base")
                .with_context_window(128_000),
            ModelMeta::new("o3-mini")
                .with_cost(1.1, 4.4)
                .with_thinking()
                .with_no_system()
                .with_tokenizer("o200k_base")
                .with_context_window(200_000),
            ModelMeta::new("o1")
                .with_cost(15.0, 60.0)
                .with_thinking()
                .with_no_system()
                .with_vision()
                .with_tokenizer("o200k_base")
                .with_context_window(200_000),
        ],
    ),
    ProviderMeta::new(
        "google",
        "Google Gemini",
        "https://generativelanguage.googleapis.com/v1beta",
        "GEMINI_API_KEY",
        &[
            ModelMeta::new("gemini-2.5-pro")
                .with_cost(1.25, 10.0)
                .with_thinking()
                .with_vision()
                .with_context_window(1_000_000),
            ModelMeta::new("gemini-2.5-flash")
                .with_cost(0.15, 0.6)
                .with_thinking()
                .with_vision()
                .with_context_window(1_000_000),
            ModelMeta::new("gemini-2.0-flash")
                .with_cost(0.1, 0.4)
                .with_vision()
                .with_context_window(1_000_000),
        ],
    ),
    ProviderMeta::new(
        "deepseek",
        "DeepSeek",
        "https://api.deepseek.com/v1",
        "DEEPSEEK_API_KEY",
        &[
            ModelMeta::new("deepseek-chat")
                .with_cost(0.14, 0.28)
                .with_context_window(64_000),
            ModelMeta::new("deepseek-reasoner")
                .with_cost(0.55, 2.19)
                .with_thinking()
                .with_context_window(64_000),
        ],
    ),
    ProviderMeta::new(
        "openrouter",
        "OpenRouter",
        "https://openrouter.ai/api/v1",
        "OPENROUTER_API_KEY",
        &[
            ModelMeta::new("anthropic/claude-sonnet-4-6")
                .with_cost(3.0, 15.0)
                .with_thinking()
                .with_vision()
                .with_context_window(200_000),
            ModelMeta::new("openai/gpt-4o")
                .with_cost(5.0, 15.0)
                .with_vision()
                .with_context_window(128_000),
            ModelMeta::new("google/gemini-2.5-pro")
                .with_cost(1.25, 10.0)
                .with_thinking()
                .with_vision()
                .with_context_window(1_000_000),
            ModelMeta::new("deepseek/deepseek-chat")
                .with_cost(0.5, 2.0)
                .with_context_window(64_000),
            ModelMeta::new("deepseek/deepseek-r1")
                .with_cost(0.55, 2.19)
                .with_thinking()
                .with_context_window(64_000),
        ],
    ),
    ProviderMeta::new(
        "groq",
        "Groq",
        "https://api.groq.com/openai/v1",
        "GROQ_API_KEY",
        &[
            ModelMeta::new("llama-3.3-70b-versatile")
                .with_cost(0.59, 0.79)
                .with_context_window(128_000),
            ModelMeta::new("gemma2-9b-it")
                .with_cost(0.2, 0.2)
                .with_context_window(128_000),
            ModelMeta::new("mixtral-8x7b-32768")
                .with_cost(0.24, 0.24)
                .with_context_window(128_000),
        ],
    ),
    ProviderMeta::new(
        "mistral",
        "Mistral",
        "https://api.mistral.ai/v1",
        "MISTRAL_API_KEY",
        &[
            ModelMeta::new("mistral-large-latest")
                .with_cost(2.0, 6.0)
                .with_context_window(128_000),
            ModelMeta::new("codestral-latest")
                .with_cost(2.0, 6.0)
                .with_context_window(128_000),
            ModelMeta::new("devstral-latest")
                .with_cost(2.0, 6.0)
                .with_context_window(128_000),
        ],
    ),
    ProviderMeta::new(
        "fireworks",
        "Fireworks",
        "https://api.fireworks.ai/inference/v1",
        "FIREWORKS_API_KEY",
        &[
            ModelMeta::new("accounts/fireworks/models/deepseek-v3")
                .with_cost(0.9, 0.9)
                .with_context_window(128_000),
            ModelMeta::new("accounts/fireworks/models/llama-v3p1-405b-instruct")
                .with_cost(2.0, 2.0)
                .with_context_window(128_000),
        ],
    ),
    ProviderMeta::new(
        "together",
        "Together AI",
        "https://api.together.xyz/v1",
        "TOGETHER_API_KEY",
        &[
            ModelMeta::new("meta-llama/Llama-3.3-70B-Instruct-Turbo")
                .with_cost(0.88, 0.88)
                .with_context_window(128_000),
            ModelMeta::new("deepseek-ai/DeepSeek-V3")
                .with_cost(1.25, 1.25)
                .with_context_window(128_000),
        ],
    ),
    ProviderMeta::new(
        "minimax",
        "MiniMax",
        "https://api.minimaxi.chat/v1",
        "MINIMAX_API_KEY",
        &[
            ModelMeta::new("MiniMax-M3").with_context_window(256_000),
            ModelMeta::new("MiniMax-M2.7").with_context_window(256_000),
        ],
    ),
    ProviderMeta::new(
        "moonshotai",
        "Moonshot AI",
        "https://api.moonshot.cn/v1",
        "MOONSHOT_API_KEY",
        &[
            ModelMeta::new("kimi-k2.5").with_context_window(256_000),
            ModelMeta::new("kimi-k2.6").with_context_window(256_000),
            ModelMeta::new("kimi-k2-thinking").with_context_window(256_000),
        ],
    ),
    ProviderMeta::new(
        "xai",
        "xAI",
        "https://api.x.ai/v1",
        "XAI_API_KEY",
        &[
            ModelMeta::new("grok-3")
                .with_cost(3.0, 15.0)
                .with_vision()
                .with_context_window(128_000),
            ModelMeta::new("grok-3-mini")
                .with_cost(0.3, 0.5)
                .with_vision()
                .with_context_window(128_000),
        ],
    ),
    ProviderMeta::new(
        "ollama",
        "Ollama (local)",
        "http://localhost:11434/v1",
        "OLLAMA_HOST",
        &[
            ModelMeta::new("llama3.1").with_context_window(128_000),
            ModelMeta::new("qwen2.5-coder:7b").with_context_window(128_000),
            ModelMeta::new("mistral").with_context_window(128_000),
        ],
    ),
];
