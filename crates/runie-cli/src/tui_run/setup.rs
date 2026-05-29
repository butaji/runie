use runie_ai::ModelRegistry;
use runie_tui::{Msg, Tui};

use crate::settings::Settings;

/// Check if user needs onboarding (no provider, model, or API key configured)
pub fn needs_onboarding(settings: &Settings) -> bool {
    // No provider configured
    if settings.provider.is_empty() {
        return true;
    }
    // No model configured
    if settings.model.is_empty() {
        return true;
    }
    // API key set in settings file
    if settings.api_key.is_some() {
        return false;
    }
    // No API key in environment
    if std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("ANTHROPIC_API_KEY").is_err()
        && std::env::var("GOOGLE_API_KEY").is_err()
        && std::env::var("RUNIE_API_KEY").is_err()
        && std::env::var("MINIMAX_API_KEY").is_err()
    {
        return true;
    }
    false
}

/// Update top bar context percentages from current state (Critical #4)
pub fn update_top_bar_context(tui: &mut Tui, settings: &Settings) {
    // Calculate estimated tokens from message history (rough: ~4 chars/token)
    let total_chars: usize = tui.state.messages.iter().map(|msg| match msg {
        runie_tui::MessageItem::User { text, .. } => text.len(),
        runie_tui::MessageItem::Assistant { text, .. } => text.len(),
        runie_tui::MessageItem::System { text } => text.len(),
        runie_tui::MessageItem::ToolCall { name, args, result, .. } => {
            name.len() + args.len() + result.as_ref().map(|s| s.len()).unwrap_or(0)
        }
        _ => 0,
    }).sum();

    let estimated_tokens = total_chars / 4;

    // Look up context window for current model
    let registry = ModelRegistry::new();
    let context_window = registry.get(&settings.model)
        .map(|m| m.context_window)
        .unwrap_or(128_000); // default fallback

    // Update top bar via Msg (Critical #4)
    tui.update(Msg::UpdateTopBarContext {
        model: settings.model.clone(),
        context_window: Some(context_window),
        estimated_tokens: Some(estimated_tokens),
    });
}
