#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use crate::settings::Settings;
    use crate::settings::CliSettings;
    use crate::settings::runie_dir;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.enable_thinking, true);
    }

    /// Guard that saves env vars, clears them, sets test values, and restores on drop
    struct EnvGuard {
        originals: Vec<(String, Option<String>)>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self { originals: Vec::new() }
        }

        fn save_and_clear(&mut self, var: &str) {
            let original = std::env::var(var).ok();
            std::env::remove_var(var);
            self.originals.push((var.to_string(), original));
        }

        fn set(&self, var: &str, val: &str) {
            std::env::set_var(var, val);
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (var, original) in self.originals.drain(..) {
                std::env::remove_var(&var);
                if let Some(val) = original {
                    std::env::set_var(&var, val);
                }
            }
        }
    }

    #[test]
    fn test_merge_env() {
        // Test merge_env directly on a default Settings instance
        // This avoids issues with load() and external config files
        
        // Clear env vars and set test values
        std::env::remove_var("RUNIE_MODEL");
        std::env::remove_var("RUNIE_PROVIDER");
        std::env::remove_var("RUNIE_MAX_TURNS");
        std::env::remove_var("RUNIE_ENABLE_THINKING");
        std::env::remove_var("RUNIE_SHELL");
        
        std::env::set_var("RUNIE_MODEL", "claude-3-opus");
        std::env::set_var("RUNIE_MAX_TURNS", "20");

        // Create settings from defaults and merge env
        let mut settings = Settings::default();
        settings.merge_env();
        
        // Verify env vars were applied
        assert_eq!(settings.model, "claude-3-opus", "Model should be from RUNIE_MODEL env var");
        assert_eq!(settings.max_turns, 20, "max_turns should be from RUNIE_MAX_TURNS env var");
        
        // Clean up - restore original behavior
        std::env::remove_var("RUNIE_MODEL");
        std::env::remove_var("RUNIE_MAX_TURNS");
    }

    #[test]
    fn test_merge_cli() {
        let mut settings = Settings::default();
        let cli = CliSettings {
            model: Some("anthropic".to_string()),
            max_turns: Some(5),
            ..Default::default()
        };
        settings.merge_cli(&cli);
        assert_eq!(settings.model, "anthropic");
        assert_eq!(settings.max_turns, 5);
    }

    #[test]
    fn test_settings_default_values() {
        let settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.enable_thinking, true);
        assert_eq!(settings.shell, std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()));
    }

    #[test]
    fn test_settings_layered_resolution() {
        let mut guard = EnvGuard::new();
        guard.save_and_clear("RUNIE_MODEL");
        guard.save_and_clear("RUNIE_PROVIDER");
        guard.save_and_clear("RUNIE_MAX_TURNS");

        // Start with defaults
        let mut settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);

        // Layer: env vars override defaults
        std::env::set_var("RUNIE_MODEL", "claude-3-opus");
        std::env::set_var("RUNIE_PROVIDER", "anthropic");
        std::env::set_var("RUNIE_MAX_TURNS", "25");
        settings.merge_env();
        assert_eq!(settings.model, "claude-3-opus");
        assert_eq!(settings.provider, "anthropic");
        assert_eq!(settings.max_turns, 25);

        // Layer: CLI overrides env
        let cli = CliSettings {
            model: Some("gpt-5".to_string()),
            provider: Some("openai".to_string()),
            max_turns: Some(50),
            ..Default::default()
        };
        settings.merge_cli(&cli);
        assert_eq!(settings.model, "gpt-5");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 50);
    }

    #[test]
    fn test_dev_folder_sets_runie_home() {
        use std::path::PathBuf;

        // RUNIE_HOME env var should be respected
        std::env::set_var("RUNIE_HOME", "./tmp");
        let dir = runie_dir();
        assert!(dir.is_some());
        assert_eq!(dir.unwrap(), PathBuf::from("./tmp"));
        std::env::remove_var("RUNIE_HOME");

        // Without RUNIE_HOME, falls back to ~/.runie
        std::env::remove_var("RUNIE_HOME");
        let dir = runie_dir();
        assert!(dir.is_some());
        // Should contain .runie
        assert!(dir.unwrap().to_string_lossy().contains(".runie"));
    }

    #[test]
    fn test_validate_model_true() {
        let settings = Settings {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert!(settings.validate_model());
    }

    #[test]
    fn test_validate_model_false() {
        let settings = Settings {
            model: "invalid-model-xyz".to_string(),
            provider: "openai".to_string(),
            ..Default::default()
        };
        assert!(!settings.validate_model());
    }

    #[test]
    fn test_settings_merge_cli_overrides() {
        let mut settings = Settings::default();
        // Default values
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.provider, "openai");
        assert_eq!(settings.max_turns, 10);
        assert_eq!(settings.enable_thinking, true);

        // CLI overrides everything
        let cli = CliSettings {
            model: Some("gpt-5".to_string()),
            provider: Some("anthropic".to_string()),
            api_key: Some("sk-test123".to_string()),
            max_turns: Some(100),
            enable_thinking: Some(false),
            shell: Some("/bin/zsh".to_string()),
        };
        settings.merge_cli(&cli);

        assert_eq!(settings.model, "gpt-5");
        assert_eq!(settings.provider, "anthropic");
        assert_eq!(settings.api_key, Some("sk-test123".to_string()));
        assert_eq!(settings.max_turns, 100);
        assert_eq!(settings.enable_thinking, false);
        assert_eq!(settings.shell, "/bin/zsh");
    }
}
