use crate::model::AppState;

impl AppState {
    pub(crate) fn reload_all(&mut self) {
        let config = crate::config_reload::Config::load_from(&crate::config_reload::config_path());
        if let Some(provider) = &config.provider {
            self.config.config_provider = provider.clone();
        }
        if let Some(model) = config.default_model() {
            self.config.config_model = model.to_string();
        }
        if let Some(theme) = &config.theme {
            self.config.theme_name = theme.clone();
        }
        self.config.vim_mode = config.vim_mode();
        self.skills = crate::skills::load_all();
        let prompts_section = config.prompts();
        self.prompts = crate::prompts::load_prompts(
            prompts_section.default.as_deref(),
            prompts_section.custom.as_deref(),
        );
        self.add_system_msg(
            "Reloaded config, keybindings, theme, skills, and prompts.".to_string(),
        );
    }

    pub(crate) fn show_diagnostics(&mut self) {
        let mut lines = vec!["Diagnostics:".to_string()];
        let config_path = crate::config_reload::config_path();
        lines.push(format!(
            "  Config: {}",
            if config_path.exists() {
                config_path.display().to_string()
            } else {
                "not found".to_string()
            }
        ));
        let kb_path = crate::keybindings::default_keybindings_path();
        lines.push(format!(
            "  Keybindings: {}",
            if kb_path.as_ref().map(|p| p.exists()).unwrap_or(false) {
                kb_path.unwrap().display().to_string()
            } else {
                "default".to_string()
            }
        ));
        lines.push(format!("  Theme: {}", self.config.theme_name));
        lines.push(format!(
            "  Provider: {}/{}",
            self.config.current_provider, self.config.current_model
        ));
        lines.push(format!("  Read-only: {}", self.config.read_only));
        lines.push(format!(
            "  Scoped models: {}",
            self.config.scoped_models.len()
        ));
        self.add_system_msg(lines.join("\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_all_reloads_skills() {
        let mut state = AppState::default();
        state.skills = vec![crate::skills::Skill {
            name: "dummy".into(),
            description: "dummy".into(),
            context: "".into(),
            user_invocable: false,
            file_path: std::path::PathBuf::from("dummy.md"),
        }];
        state.reload_all();
        // In test environment load_all returns empty (no skill dirs exist)
        assert!(
            state.skills.is_empty(),
            "reload_all should reload skills from disk"
        );
        let last = state.session.messages.last().unwrap();
        assert!(
            last.content.contains("Reloaded"),
            "Should confirm reload: {}",
            last.content
        );
        // Prompts should also be reloaded (empty in test env)
        assert!(
            !state.prompts.is_empty(),
            "reload_all should reload prompts"
        );
        assert_eq!(state.prompts[0].name, "default");
    }
}
