use runie_ai::ModelRegistry;
use runie_tui::{Msg, Tui};

use crate::settings::Settings;

/// Check if user needs onboarding (no provider or model configured)
///
/// Onboarding is NOT needed if:
/// - Config file exists with provider AND model set
/// - User has previously selected "don't show again" (skip_onboarding)
///
/// API key presence is NOT checked here because:
/// - API key is for provider creation, not onboarding eligibility
/// - User may provide API key via environment variable later
pub fn needs_onboarding(settings: &Settings) -> bool {
    // Check skip_onboarding flag first - if set, never show onboarding
    if settings.skip_onboarding {
        return false;
    }

    // Provider and model must be set in config
    if settings.provider.is_empty() {
        return true;
    }
    if settings.model.is_empty() {
        return true;
    }
    // Config has provider and model - no onboarding needed
    // (API key is handled separately by provider creation)
    false
}

/// Update top bar context percentages from current state (Critical #4)
pub fn update_top_bar_context(tui: &mut Tui, settings: &Settings) {
    // Calculate estimated tokens from message history (rough: ~4 chars/token)
    let total_chars: usize = tui.messages().iter().map(|msg| match msg {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TempConfig {
        path: PathBuf,
        orig_runie_home: Option<String>,
        orig_runie_model: Option<String>,
        orig_runie_provider: Option<String>,
        orig_runie_max_turns: Option<String>,
    }

    impl TempConfig {
        fn new() -> Self {
            let temp_dir = std::env::temp_dir().join("runie_test_config");
            let config_path = temp_dir.join("config.toml");
            
            // Save original env vars
            let orig_runie_home = std::env::var("RUNIE_HOME").ok();
            let orig_runie_model = std::env::var("RUNIE_MODEL").ok();
            let orig_runie_provider = std::env::var("RUNIE_PROVIDER").ok();
            let orig_runie_max_turns = std::env::var("RUNIE_MAX_TURNS").ok();
            
            // Clear env vars that could interfere with config file test
            std::env::remove_var("RUNIE_HOME");
            std::env::remove_var("RUNIE_MODEL");
            std::env::remove_var("RUNIE_PROVIDER");
            std::env::remove_var("RUNIE_MAX_TURNS");
            
            // Create temp config with provider and model
            let config_content = r#"# Runie configuration
model = "gpt-4o"
provider = "openai"
max_turns = 10
enable_thinking = true
"#;
            fs::create_dir_all(&temp_dir).unwrap();
            fs::write(&config_path, config_content).unwrap();
            
            // Set RUNIE_HOME to temp dir
            std::env::set_var("RUNIE_HOME", temp_dir.to_string_lossy().as_ref());
            
            Self {
                path: config_path,
                orig_runie_home,
                orig_runie_model,
                orig_runie_provider,
                orig_runie_max_turns,
            }
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            // Clear test env vars
            std::env::remove_var("RUNIE_HOME");
            std::env::remove_var("RUNIE_MODEL");
            std::env::remove_var("RUNIE_PROVIDER");
            std::env::remove_var("RUNIE_MAX_TURNS");
            
            // Restore original env vars
            if let Some(orig) = self.orig_runie_home.clone() {
                std::env::set_var("RUNIE_HOME", orig);
            }
            if let Some(orig) = self.orig_runie_model.clone() {
                std::env::set_var("RUNIE_MODEL", orig);
            }
            if let Some(orig) = self.orig_runie_provider.clone() {
                std::env::set_var("RUNIE_PROVIDER", orig);
            }
            if let Some(orig) = self.orig_runie_max_turns.clone() {
                std::env::set_var("RUNIE_MAX_TURNS", orig);
            }
            
            // Clean up temp dir
            if let Some(parent) = self.path.parent() {
                let _ = fs::remove_dir_all(parent);
            }
        }
    }

    #[test]
    fn test_tmp_config_skips_onboarding() {
        // Simulate --dev-folder=./tmp_config
        // Settings loaded from tmp_config/config.toml
        // Should NOT need onboarding
        let _temp = TempConfig::new();
        
        // Clear any API key env vars that might interfere (not handled by TempConfig)
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");
        std::env::remove_var("RUNIE_API_KEY");
        std::env::remove_var("MINIMAX_API_KEY");
        
        let settings = Settings::load();
        
        // Verify settings were loaded from config
        assert_eq!(settings.model, "gpt-4o", "Model should be from config file, not env var");
        assert_eq!(settings.provider, "openai", "Provider should be from config file");
        
        // Onboarding should NOT be needed since config has model and provider
        assert!(!needs_onboarding(&settings), "Onboarding should be skipped when config has model and provider");
    }

    #[test]
    fn test_config_loaded_flag() {
        // Config loaded flag should be true when config file exists
        let _temp = TempConfig::new();
        
        let settings = Settings::load();
        assert!(settings.config_loaded, "config_loaded should be true when config file exists");
    }
}
