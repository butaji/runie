use runie_ai::ModelRegistry;
use runie_tui::{Msg, Tui};

use crate::settings::Settings;

/// Check if user needs onboarding (no provider or model configured)
///
/// Onboarding is NOT needed if:
/// - Config file exists with provider AND model set
///
/// API key presence is NOT checked here because:
/// - API key is for provider creation, not onboarding eligibility
/// - User may provide API key via environment variable later
pub fn needs_onboarding(settings: &Settings) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TempConfig {
        path: PathBuf,
        orig_runie_home: Option<String>,
    }

    impl TempConfig {
        fn new() -> Self {
            let temp_dir = std::env::temp_dir().join("runie_test_config");
            let config_path = temp_dir.join("config.toml");
            
            // Save original RUNIE_HOME
            let orig_runie_home = std::env::var("RUNIE_HOME").ok();
            
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
            }
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            // Restore original RUNIE_HOME
            std::env::remove_var("RUNIE_HOME");
            if let Some(orig) = &self.orig_runie_home {
                std::env::set_var("RUNIE_HOME", orig);
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
        
        // Clear any API key env vars that might interfere
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("GOOGLE_API_KEY");
        std::env::remove_var("RUNIE_API_KEY");
        std::env::remove_var("MINIMAX_API_KEY");
        
        let settings = Settings::load();
        
        // Verify settings were loaded from config
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        
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
