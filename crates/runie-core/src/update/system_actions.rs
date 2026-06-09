use crate::model::AppState;

impl AppState {
    pub(crate) fn reload_all(&mut self) {
        let config = crate::config_reload::Config::load_from(
            &crate::config_reload::config_path());
        if let Some(provider) = &config.provider {
            self.config_provider = provider.clone();
        }
        if let Some(model) = config.default_model() {
            self.config_model = model.to_string();
        }
        if let Some(theme) = &config.theme {
            self.theme_name = theme.clone();
        }
        self.add_system_msg("Reloaded config, keybindings, and theme.".to_string());
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
        lines.push(format!("  Theme: {}", self.theme_name));
        lines.push(format!(
            "  Provider: {}/{}",
            self.current_provider, self.current_model
        ));
        lines.push(format!("  Read-only: {}", self.read_only));
        lines.push(format!("  Scoped models: {}", self.scoped_models.len()));
        self.add_system_msg(lines.join("\n"));
    }
}
