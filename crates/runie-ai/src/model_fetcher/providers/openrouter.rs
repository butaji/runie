use crate::ModelInfo;

// OpenRouter models - split into multiple parts due to size

pub fn openrouter_a() -> Vec<ModelInfo> {
    vec![
        ("ai21/jamba-large-1.7", "AI21: Jamba Large 1.7"), ("alibaba/tongyi-deepresearch-30b-a3b", "Tongyi DeepResearch 30B A3B"),
        ("amazon/nova-2-lite-v1", "Amazon: Nova 2 Lite"), ("amazon/nova-lite-v1", "Amazon: Nova Lite 1.0"),
        ("amazon/nova-micro-v1", "Amazon: Nova Micro 1.0"), ("amazon/nova-premier-v1", "Amazon: Nova Premier 1.0"),
        ("amazon/nova-pro-v1", "Amazon: Nova Pro 1.0"), ("anthropic/claude-3-haiku", "Anthropic: Claude 3 Haiku"),
        ("anthropic/claude-3.5-haiku", "Anthropic: Claude 3.5 Haiku"), ("anthropic/claude-haiku-4.5", "Anthropic: Claude Haiku 4.5"),
        ("anthropic/claude-opus-4", "Anthropic: Claude Opus 4"), ("anthropic/claude-opus-4.1", "Anthropic: Claude Opus 4.1"),
        ("anthropic/claude-opus-4.5", "Anthropic: Claude Opus 4.5"), ("anthropic/claude-opus-4.6", "Anthropic: Claude Opus 4.6"),
        ("anthropic/claude-opus-4.6-fast", "Anthropic: Claude Opus 4.6 (Fast)"), ("anthropic/claude-opus-4.7", "Anthropic: Claude Opus 4.7"),
        ("anthropic/claude-opus-4.7-fast", "Anthropic: Claude Opus 4.7 (Fast)"), ("anthropic/claude-sonnet-4", "Anthropic: Claude Sonnet 4"),
        ("anthropic/claude-sonnet-4.5", "Anthropic: Claude Sonnet 4.5"), ("anthropic/claude-sonnet-4.6", "Anthropic: Claude Sonnet 4.6"),
        ("arcee-ai/trinity-large-thinking", "Arcee AI: Trinity Large Thinking"), ("arcee-ai/trinity-large-thinking:free", "Arcee AI: Trinity Large Thinking (free)"),
        ("arcee-ai/trinity-mini", "Arcee AI: Trinity Mini"), ("arcee-ai/virtuoso-large", "Arcee AI: Virtuoso Large"),
        ("auto", "Auto"), ("baidu/cobuddy:free", "Baidu Qianfan: CoBuddy (free)"),
        ("baidu/ernie-4.5-21b-a3b", "Baidu: ERNIE 4.5 21B A3B"), ("baidu/ernie-4.5-vl-28b-a3b", "Baidu: ERNIE 4.5 VL 28B A3B"),
        ("bytedance-seed/seed-1.6", "ByteDance Seed: Seed 1.6"), ("bytedance-seed/seed-1.6-flash", "ByteDance Seed: Seed 1.6 Flash"),
        ("bytedance-seed/seed-2.0-lite", "ByteDance Seed: Seed-2.0-Lite"), ("bytedance-seed/seed-2.0-mini", "ByteDance Seed: Seed-2.0-Mini"),
        ("cohere/command-r-08-2024", "Cohere: Command R (08-2024)"), ("cohere/command-r-plus-08-2024", "Cohere: Command R+ (08-2024)"),
        ("deepseek/deepseek-chat", "DeepSeek: DeepSeek V3"), ("deepseek/deepseek-chat-v3-0324", "DeepSeek: DeepSeek V3 0324"),
        ("deepseek/deepseek-chat-v3.1", "DeepSeek: DeepSeek V3.1"), ("deepseek/deepseek-r1", "DeepSeek: R1"),
        ("deepseek/deepseek-r1-0528", "DeepSeek: R1 0528"), ("deepseek/deepseek-v3.1-terminus", "DeepSeek: DeepSeek V3.1 Terminus"),
        ("deepseek/deepseek-v3.2", "DeepSeek: DeepSeek V3.2"), ("deepseek/deepseek-v3.2-exp", "DeepSeek: DeepSeek V3.2 Exp"),
        ("deepseek/deepseek-v4-flash", "DeepSeek: DeepSeek V4 Flash"), ("deepseek/deepseek-v4-flash:free", "DeepSeek: DeepSeek V4 Flash (free)"),
        ("deepseek/deepseek-v4-pro", "DeepSeek: DeepSeek V4 Pro"), ("essentialai/rnj-1-instruct", "EssentialAI: Rnj 1 Instruct"),
        ("google/gemini-2.0-flash-001", "Google: Gemini 2.0 Flash"), ("google/gemini-2.0-flash-lite-001", "Google: Gemini 2.0 Flash Lite"),
        ("google/gemini-2.5-flash", "Google: Gemini 2.5 Flash"), ("google/gemini-2.5-flash-lite", "Google: Gemini 2.5 Flash Lite"),
    ].into_iter().map(|(id, name)| ModelInfo { id: id.to_string(), name: name.to_string() }).collect()
}

pub fn openrouter_b() -> Vec<ModelInfo> {
    vec![
        ("google/gemini-2.5-flash-lite-preview-09-2025", "Google: Gemini 2.5 Flash Lite Preview 09-2025"), ("google/gemini-2.5-pro", "Google: Gemini 2.5 Pro"),
        ("google/gemini-2.5-pro-preview", "Google: Gemini 2.5 Pro Preview 06-05"), ("google/gemini-2.5-pro-preview-05-06", "Google: Gemini 2.5 Pro Preview 05-06"),
        ("google/gemini-3-flash-preview", "Google: Gemini 3 Flash Preview"), ("google/gemini-3.1-flash-lite", "Google: Gemini 3.1 Flash Lite"),
        ("google/gemini-3.1-flash-lite-preview", "Google: Gemini 3.1 Flash Lite Preview"), ("google/gemini-3.1-pro-preview", "Google: Gemini 3.1 Pro Preview"),
        ("google/gemini-3.1-pro-preview-customtools", "Google: Gemini 3.1 Pro Preview Custom Tools"), ("google/gemini-3.5-flash", "Google: Gemini 3.5 Flash"),
        ("google/gemma-3-12b-it", "Google: Gemma 3 12B"), ("google/gemma-3-27b-it", "Google: Gemma 3 27B"),
        ("google/gemma-4-26b-a4b-it", "Google: Gemma 4 26B A4B "), ("google/gemma-4-26b-a4b-it:free", "Google: Gemma 4 26B A4B  (free)"),
        ("google/gemma-4-31b-it", "Google: Gemma 4 31B"), ("google/gemma-4-31b-it:free", "Google: Gemma 4 31B (free)"),
        ("ibm-granite/granite-4.1-8b", "IBM: Granite 4.1 8B"), ("inception/mercury-2", "Inception: Mercury 2"),
        ("inclusionai/ling-2.6-1t", "inclusionAI: Ling-2.6-1T"), ("inclusionai/ling-2.6-flash", "inclusionAI: Ling-2.6-flash"),
        ("inclusionai/ring-2.6-1t", "inclusionAI: Ring-2.6-1T"), ("kwaipilot/kat-coder-pro-v2", "Kwaipilot: KAT-Coder-Pro V2"),
        ("meta-llama/llama-3.1-70b-instruct", "Meta: Llama 3.1 70B Instruct"), ("meta-llama/llama-3.1-8b-instruct", "Meta: Llama 3.1 8B Instruct"),
        ("meta-llama/llama-3.3-70b-instruct", "Meta: Llama 3.3 70B Instruct"), ("meta-llama/llama-3.3-70b-instruct:free", "Meta: Llama 3.3 70B Instruct (free)"),
        ("meta-llama/llama-4-scout", "Meta: Llama 4 Scout"), ("minimax/minimax-m1", "MiniMax: MiniMax M1"),
        ("minimax/minimax-m2", "MiniMax: MiniMax M2"), ("minimax/minimax-m2.1", "MiniMax: MiniMax M2.1"),
        ("minimax/minimax-m2.5", "MiniMax: MiniMax M2.5"), ("minimax/minimax-m2.5:free", "MiniMax: MiniMax M2.5 (free)"),
        ("minimax/minimax-m2.7", "MiniMax: MiniMax M2.7"), ("mistralai/codestral-2508", "Mistral: Codestral 2508"),
        ("mistralai/devstral-2512", "Mistral: Devstral 2 2512"), ("mistralai/devstral-medium", "Mistral: Devstral Medium"),
        ("mistralai/devstral-small", "Mistral: Devstral Small 1.1"), ("mistralai/ministral-14b-2512", "Mistral: Ministral 3 14B 2512"),
        ("mistralai/ministral-3b-2512", "Mistral: Ministral 3 3B 2512"), ("mistralai/ministral-8b-2512", "Mistral: Ministral 3 8B 2512"),
        ("mistralai/mistral-large", "Mistral Large"), ("mistralai/mistral-large-2407", "Mistral Large 2407"),
        ("mistralai/mistral-large-2411", "Mistral: Mistral Large 3 2411"), ("mistralai/mistral-large-2512", "Mistral: Mistral Large 3 2512"),
        ("mistralai/mistral-medium-3", "Mistral: Mistral Medium 3"), ("mistralai/mistral-medium-3-5", "Mistral: Mistral Medium 3.5"),
        ("mistralai/mistral-medium-3.1", "Mistral: Mistral Medium 3.1"), ("mistralai/mistral-nemo", "Mistral: Mistral Nemo"),
        ("mistralai/mistral-saba", "Mistral: Saba"), ("mistralai/mistral-small-2603", "Mistral: Mistral Small 4"),
        ("mistralai/mistral-small-3.2-24b-instruct", "Mistral: Mistral Small 3.2 24B"),
    ].into_iter().map(|(id, name)| ModelInfo { id: id.to_string(), name: name.to_string() }).collect()
}

pub fn openrouter_c() -> Vec<ModelInfo> {
    vec![
        ("mistralai/mixtral-8x22b-instruct", "Mistral: Mixtral 8x22B Instruct"), ("mistralai/pixtral-large-2411", "Mistral: Pixtral Large 2411"),
        ("mistralai/voxtral-small-24b-2507", "Mistral: Voxtral Small 24B 2507"), ("moonshotai/kimi-k2", "MoonshotAI: Kimi K2 0711"),
        ("moonshotai/kimi-k2-0905", "MoonshotAI: Kimi K2 0905"), ("moonshotai/kimi-k2-thinking", "MoonshotAI: Kimi K2 Thinking"),
        ("moonshotai/kimi-k2.5", "MoonshotAI: Kimi K2.5"), ("moonshotai/kimi-k2.6", "MoonshotAI: Kimi K2.6"),
        ("nex-agi/deepseek-v3.1-nex-n1", "Nex AGI: DeepSeek V3.1 Nex N1"), ("nvidia/llama-3.3-nemotron-super-49b-v1.5", "NVIDIA: Llama 3.3 Nemotron Super 49B V1.5"),
        ("nvidia/nemotron-3-nano-30b-a3b", "NVIDIA: Nemotron 3 Nano 30B A3B"), ("nvidia/nemotron-3-nano-30b-a3b:free", "NVIDIA: Nemotron 3 Nano 30B A3B (free)"),
        ("nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free", "NVIDIA: Nemotron 3 Nano Omni (free)"), ("nvidia/nemotron-3-super-120b-a12b", "NVIDIA: Nemotron 3 Super"),
        ("nvidia/nemotron-3-super-120b-a12b:free", "NVIDIA: Nemotron 3 Super (free)"), ("nvidia/nemotron-nano-12b-v2-vl:free", "NVIDIA: Nemotron Nano 12B 2 VL (free)"),
        ("nvidia/nemotron-nano-9b-v2", "NVIDIA: Nemotron Nano 9B V2"), ("nvidia/nemotron-nano-9b-v2:free", "NVIDIA: Nemotron Nano 9B V2 (free)"),
        ("openai/gpt-3.5-turbo", "OpenAI: GPT-3.5 Turbo"), ("openai/gpt-3.5-turbo-0613", "OpenAI: GPT-3.5 Turbo (older v0613)"),
        ("openai/gpt-3.5-turbo-16k", "OpenAI: GPT-3.5 Turbo 16k"), ("openai/gpt-4", "OpenAI: GPT-4"),
        ("openai/gpt-4-0314", "OpenAI: GPT-4 (older v0314)"), ("openai/gpt-4-1106-preview", "OpenAI: GPT-4 Turbo (older v1106)"),
        ("openai/gpt-4-turbo", "OpenAI: GPT-4 Turbo"), ("openai/gpt-4-turbo-preview", "OpenAI: GPT-4 Turbo Preview"),
        ("openai/gpt-4.1", "OpenAI: GPT-4.1"), ("openai/gpt-4.1-mini", "OpenAI: GPT-4.1 Mini"),
        ("openai/gpt-4.1-nano", "OpenAI: GPT-4.1 Nano"), ("openai/gpt-4o", "OpenAI: GPT-4o"),
        ("openai/gpt-4o-2024-05-13", "OpenAI: GPT-4o (2024-05-13)"), ("openai/gpt-4o-2024-08-06", "OpenAI: GPT-4o (2024-08-06)"),
        ("openai/gpt-4o-2024-11-20", "OpenAI: GPT-4o (2024-11-20)"), ("openai/gpt-4o-audio-preview", "OpenAI: GPT-4o Audio"),
        ("openai/gpt-4o-mini", "OpenAI: GPT-4o-mini"), ("openai/gpt-4o-mini-2024-07-18", "OpenAI: GPT-4o-mini (2024-07-18)"),
        ("openai/gpt-5", "OpenAI: GPT-5"), ("openai/gpt-5-codex", "OpenAI: GPT-5 Codex"),
        ("openai/gpt-5-mini", "OpenAI: GPT-5 Mini"), ("openai/gpt-5-nano", "OpenAI: GPT-5 Nano"),
        ("openai/gpt-5-pro", "OpenAI: GPT-5 Pro"), ("openai/gpt-5.1", "OpenAI: GPT-5.1"),
        ("openai/gpt-5.1-chat", "OpenAI: GPT-5.1 Chat"), ("openai/gpt-5.1-codex", "OpenAI: GPT-5.1-Codex"),
        ("openai/gpt-5.1-codex-max", "OpenAI: GPT-5.1-Codex-Max"), ("openai/gpt-5.1-codex-mini", "OpenAI: GPT-5.1-Codex-Mini"),
        ("openai/gpt-5.2", "OpenAI: GPT-5.2"), ("openai/gpt-5.2-chat", "OpenAI: GPT-5.2 Chat"),
        ("openai/gpt-5.2-codex", "OpenAI: GPT-5.2-Codex"), ("openai/gpt-5.2-pro", "OpenAI: GPT-5.2 Pro"),
    ].into_iter().map(|(id, name)| ModelInfo { id: id.to_string(), name: name.to_string() }).collect()
}

pub fn openrouter_d() -> Vec<ModelInfo> {
    vec![
        ("openai/gpt-5.3-chat", "OpenAI: GPT-5.3 Chat"), ("openai/gpt-5.3-codex", "OpenAI: GPT-5.3-Codex"),
        ("openai/gpt-5.4", "OpenAI: GPT-5.4"), ("openai/gpt-5.4-mini", "OpenAI: GPT-5.4 Mini"),
        ("openai/gpt-5.4-nano", "OpenAI: GPT-5.4 Nano"), ("openai/gpt-5.4-pro", "OpenAI: GPT-5.4 Pro"),
        ("openai/gpt-5.5", "OpenAI: GPT-5.5"), ("openai/gpt-5.5-pro", "OpenAI: GPT-5.5 Pro"),
        ("openai/gpt-audio", "OpenAI: GPT Audio"), ("openai/gpt-audio-mini", "OpenAI: GPT Audio Mini"),
        ("openai/gpt-chat-latest", "OpenAI: GPT Chat Latest"), ("openai/gpt-oss-120b", "OpenAI: gpt-oss-120b"),
        ("openai/gpt-oss-120b:free", "OpenAI: gpt-oss-120b (free)"), ("openai/gpt-oss-20b", "OpenAI: gpt-oss-20b"),
        ("openai/gpt-oss-20b:free", "OpenAI: gpt-oss-20b (free)"), ("openai/gpt-oss-safeguard-20b", "OpenAI: gpt-oss-safeguard-20b"),
        ("openai/o1", "OpenAI: o1"), ("openai/o3", "OpenAI: o3"),
        ("openai/o3-deep-research", "OpenAI: o3 Deep Research"), ("openai/o3-mini", "OpenAI: o3 Mini"),
        ("openai/o3-mini-high", "OpenAI: o3 Mini High"), ("openai/o3-pro", "OpenAI: o3 Pro"),
        ("openai/o4-mini", "OpenAI: o4 Mini"), ("openai/o4-mini-deep-research", "OpenAI: o4 Mini Deep Research"),
        ("openai/o4-mini-high", "OpenAI: o4 Mini High"), ("openrouter/auto", "Auto Router"),
        ("openrouter/free", "Free Models Router"), ("openrouter/owl-alpha", "Owl Alpha"),
        ("poolside/laguna-m.1:free", "Poolside: Laguna M.1 (free)"), ("poolside/laguna-xs.2:free", "Poolside: Laguna XS.2 (free)"),
        ("prime-intellect/intellect-3", "Prime Intellect: INTELLECT-3"), ("qwen/qwen-2.5-72b-instruct", "Qwen2.5 72B Instruct"),
        ("qwen/qwen-2.5-7b-instruct", "Qwen: Qwen2.5 7B Instruct"), ("qwen/qwen-plus", "Qwen: Qwen-Plus"),
        ("qwen/qwen-plus-2025-07-28", "Qwen: Qwen Plus 0728"), ("qwen/qwen-plus-2025-07-28:thinking", "Qwen: Qwen Plus 0728 (thinking)"),
        ("qwen/qwen3-14b", "Qwen: Qwen3 14B"), ("qwen/qwen3-235b-a22b", "Qwen: Qwen3 235B A22B"),
        ("qwen/qwen3-235b-a22b-2507", "Qwen: Qwen3 235B A22B Instruct 2507"), ("qwen/qwen3-235b-a22b-thinking-2507", "Qwen: Qwen3 235B A22B Thinking 2507"),
        ("qwen/qwen3-30b-a3b", "Qwen: Qwen3 30B A3B"), ("qwen/qwen3-30b-a3b-instruct-2507", "Qwen: Qwen3 30B A3B Instruct 2507"),
        ("qwen/qwen3-30b-a3b-thinking-2507", "Qwen: Qwen3 30B A3B Thinking 2507"), ("qwen/qwen3-32b", "Qwen: Qwen3 32B"),
        ("qwen/qwen3-8b", "Qwen: Qwen3 8B"), ("qwen/qwen3-coder", "Qwen: Qwen3 Coder 480B A35B"),
        ("qwen/qwen3-coder-30b-a3b-instruct", "Qwen: Qwen3 Coder 30B A3B Instruct"), ("qwen/qwen3-coder-flash", "Qwen: Qwen3 Coder Flash"),
        ("qwen/qwen3-coder-next", "Qwen: Qwen3 Coder Next"), ("qwen/qwen3-coder-plus", "Qwen: Qwen3 Coder Plus"),
        ("qwen/qwen3-coder:free", "Qwen: Qwen3 Coder 480B A35B (free)"),
    ].into_iter().map(|(id, name)| ModelInfo { id: id.to_string(), name: name.to_string() }).collect()
}

pub fn openrouter_e() -> Vec<ModelInfo> {
    vec![
        ("qwen/qwen3-max", "Qwen: Qwen3 Max"), ("qwen/qwen3-max-thinking", "Qwen: Qwen3 Max Thinking"),
        ("qwen/qwen3-next-80b-a3b-instruct", "Qwen: Qwen3 Next 80B A3B Instruct"), ("qwen/qwen3-next-80b-a3b-instruct:free", "Qwen: Qwen3 Next 80B A3B Instruct (free)"),
        ("qwen/qwen3-next-80b-a3b-thinking", "Qwen: Qwen3 Next 80B A3B Thinking"), ("qwen/qwen3-vl-235b-a22b-instruct", "Qwen: Qwen3 VL 235B A22B Instruct"),
        ("qwen/qwen3-vl-235b-a22b-thinking", "Qwen: Qwen3 VL 235B A22B Thinking"), ("qwen/qwen3-vl-30b-a3b-instruct", "Qwen: Qwen3 VL 30B A3B Instruct"),
        ("qwen/qwen3-vl-30b-a3b-thinking", "Qwen: Qwen3 VL 30B A3B Thinking"), ("qwen/qwen3-vl-32b-instruct", "Qwen: Qwen3 VL 32B Instruct"),
        ("qwen/qwen3-vl-8b-instruct", "Qwen: Qwen3 VL 8B Instruct"), ("qwen/qwen3-vl-8b-thinking", "Qwen: Qwen3 VL 8B Thinking"),
        ("qwen/qwen3.5-122b-a10b", "Qwen: Qwen3.5-122B-A10B"), ("qwen/qwen3.5-27b", "Qwen: Qwen3.5-27B"),
        ("qwen/qwen3.5-35b-a3b", "Qwen: Qwen3.5-35B-A3B"), ("qwen/qwen3.5-397b-a17b", "Qwen: Qwen3.5 397B A17B"),
        ("qwen/qwen3.5-9b", "Qwen: Qwen3.5-9B"), ("qwen/qwen3.5-flash-02-23", "Qwen: Qwen3.5-Flash"),
        ("qwen/qwen3.5-plus-02-15", "Qwen: Qwen3.5 Plus 2026-02-15"), ("qwen/qwen3.5-plus-20260420", "Qwen: Qwen3.5 Plus 2026-04-20"),
        ("qwen/qwen3.6-27b", "Qwen: Qwen3.6 27B"), ("qwen/qwen3.6-35b-a3b", "Qwen: Qwen3.6 35B A3B"),
        ("qwen/qwen3.6-flash", "Qwen: Qwen3.6 Flash"), ("qwen/qwen3.6-max-preview", "Qwen: Qwen3.6 Max Preview"),
        ("qwen/qwen3.6-plus", "Qwen: Qwen3.6 Plus"), ("qwen/qwen3.7-max", "Qwen: Qwen3.7 Max"),
        ("rekaai/reka-edge", "Reka Edge"), ("relace/relace-search", "Relace: Relace Search"),
        ("sao10k/l3-euryale-70b", "Sao10k: Llama 3 Euryale 70B v2.1"), ("sao10k/l3.1-euryale-70b", "Sao10K: Llama 3.1 Euryale 70B v2.2"),
        ("stepfun/step-3.5-flash", "StepFun: Step 3.5 Flash"), ("tencent/hy3-preview", "Tencent: Hy3 preview"),
        ("thedrummer/rocinante-12b", "TheDrummer: Rocinante 12B"), ("thedrummer/unslopnemo-12b", "TheDrummer: UnslopNemo 12B"),
        ("upstage/solar-pro-3", "Upstage: Solar Pro 3"), ("x-ai/grok-4.20", "xAI: Grok 4.20"),
        ("x-ai/grok-4.3", "xAI: Grok 4.3"), ("x-ai/grok-build-0.1", "xAI: Grok Build 0.1"),
        ("xiaomi/mimo-v2-flash", "Xiaomi: MiMo-V2-Flash"), ("xiaomi/mimo-v2-omni", "Xiaomi: MiMo-V2-Omni"),
        ("xiaomi/mimo-v2-pro", "Xiaomi: MiMo-V2-Pro"), ("xiaomi/mimo-v2.5", "Xiaomi: MiMo-V2.5"),
        ("xiaomi/mimo-v2.5-pro", "Xiaomi: MiMo-V2.5-Pro"), ("z-ai/glm-4-32b", "Z.ai: GLM 4 32B "),
        ("z-ai/glm-4.5", "Z.ai: GLM 4.5"), ("z-ai/glm-4.5-air", "Z.ai: GLM 4.5 Air"),
        ("z-ai/glm-4.5-air:free", "Z.ai: GLM 4.5 Air (free)"), ("z-ai/glm-4.5v", "Z.ai: GLM 4.5V"),
        ("z-ai/glm-4.6", "Z.ai: GLM 4.6"), ("z-ai/glm-4.6v", "Z.ai: GLM 4.6V"),
        ("z-ai/glm-4.7", "Z.ai: GLM 4.7"), ("z-ai/glm-4.7-flash", "Z.ai: GLM 4.7 Flash"),
        ("z-ai/glm-5", "Z.ai: GLM 5"), ("z-ai/glm-5-turbo", "Z.ai: GLM 5 Turbo"),
        ("z-ai/glm-5.1", "Z.ai: GLM 5.1"), ("z-ai/glm-5v-turbo", "Z.ai: GLM 5V Turbo"),
        ("~anthropic/claude-haiku-latest", "Anthropic Claude Haiku Latest"), ("~anthropic/claude-opus-latest", "Anthropic: Claude Opus Latest"),
        ("~anthropic/claude-sonnet-latest", "Anthropic Claude Sonnet Latest"), ("~google/gemini-flash-latest", "Google Gemini Flash Latest"),
        ("~google/gemini-pro-latest", "Google Gemini Pro Latest"), ("~moonshotai/kimi-latest", "MoonshotAI Kimi Latest"),
        ("~openai/gpt-latest", "OpenAI GPT Latest"), ("~openai/gpt-mini-latest", "OpenAI GPT Mini Latest"),
    ].into_iter().map(|(id, name)| ModelInfo { id: id.to_string(), name: name.to_string() }).collect()
}

pub fn all_openrouter() -> Vec<ModelInfo> {
    let mut models = openrouter_a();
    models.extend(openrouter_b());
    models.extend(openrouter_c());
    models.extend(openrouter_d());
    models.extend(openrouter_e());
    models
}
