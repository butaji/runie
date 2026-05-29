use std::collections::HashMap;
use crate::ModelInfo;
use super::providers::*;

pub fn get_provider_models(provider: &str) -> Option<Vec<ModelInfo>> {
    let mut registry: HashMap<&str, Vec<ModelInfo>> = HashMap::new();

    registry.insert("openai", openai());
    registry.insert("anthropic", anthropic());
    registry.insert("groq", groq());
    registry.insert("together", together());
    registry.insert("xai", xai());
    registry.insert("mistral", mistral());
    registry.insert("deepseek", deepseek());
    registry.insert("openrouter", all_openrouter());
    registry.insert("minimax", minimax());
    registry.insert("huggingface", huggingface());
    registry.insert("zai", zai());
    registry.insert("google", google());
    registry.insert("gemini", google());
    registry.insert("ollama", ollama());
    registry.insert("azure", azure());
    registry.insert("cohere", cohere());
    registry.insert("mira", mira());
    registry.insert("galadriel", galadriel());
    registry.insert("llamafile", llamafile());
    registry.insert("perplexity", perplexity());
    registry.insert("moonshot", moonshot());
    registry.insert("hyperbolic", hyperbolic());

    registry.get(provider).cloned()
}
