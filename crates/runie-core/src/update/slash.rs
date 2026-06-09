use crate::model::AppState;

impl AppState {
    pub(crate) fn handle_slash(&mut self, content: &str) -> Option<String> {
        match content {
            "/reset" => {
                *self = AppState::default();
                Some("State cleared.".to_string())
            }
            "/help" => Some(self.help_text()),
            "/model" => Some(self.model_usage()),
            "/save" => Some("Usage: /save name".to_string()),
            "/load" => Some("Usage: /load name".to_string()),
            "/delete" => Some("Usage: /delete name".to_string()),
            "/sessions" => Some(self.sessions_list()),
            "/compact" => Some(self.handle_compact(None)),
            "/history" => Some(self.handle_history_cmd()),
            _ => self.handle_slash_with_arg(content),
        }
    }

    fn handle_slash_with_arg(&mut self, content: &str) -> Option<String> {
        if let Some(rest) = content.strip_prefix("/model ") {
            return Some(self.handle_model(rest));
        }
        if let Some(name) = content.strip_prefix("/save ") {
            return Some(self.handle_save(name));
        }
        if let Some(name) = content.strip_prefix("/load ") {
            return Some(self.handle_load(name));
        }
        if let Some(name) = content.strip_prefix("/delete ") {
            return Some(self.handle_delete(name));
        }
        if let Some(rest) = content.strip_prefix("/compact ") {
            return Some(self.handle_compact(Some(rest)));
        }
        None
    }

    fn model_usage(&self) -> String {
        format!(
            "Current model: {}/{}. Usage: /model provider/model or /model model",
            self.current_provider, self.current_model
        )
    }

    fn handle_model(&mut self, rest: &str) -> String {
        let rest = rest.trim();
        if rest.is_empty() {
            return self.model_usage();
        }
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        match parts.len() {
            2 => {
                self.current_provider = parts[0].to_string();
                self.current_model = parts[1].to_string();
                format!("Switched to {}/{}", self.current_provider, self.current_model)
            }
            1 => {
                self.current_model = parts[0].to_string();
                format!("Switched to {}/{}", self.current_provider, self.current_model)
            }
            _ => self.model_usage(),
        }
    }

    fn handle_save(&self, name: &str) -> String {
        let session = crate::session::Session {
            name: name.to_string(),
            created_at: super::now(),
            updated_at: super::now(),
            messages: self.messages.clone(),
            provider: self.current_provider.clone(),
            model: self.current_model.clone(),
        };
        match crate::session::save(name, &session) {
            Ok(()) => format!("Session '{}' saved.", name),
            Err(e) => format!("Could not save '{}': {}", name, e),
        }
    }

    fn handle_load(&mut self, name: &str) -> String {
        match crate::session::load(name) {
            Ok(session) => {
                self.messages = session.messages;
                self.current_provider = session.provider;
                self.current_model = session.model;
                self.messages_changed();
                format!("Session '{}' loaded.", name)
            }
            Err(_) => format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ),
        }
    }

    fn handle_delete(&self, name: &str) -> String {
        match crate::session::delete(name) {
            Ok(()) => format!("Session '{}' deleted.", name),
            Err(_) => format!(
                "Session '{}' not found. Use /sessions to list saved sessions.",
                name
            ),
        }
    }

    fn sessions_list(&self) -> String {
        match crate::session::list() {
            Ok(sessions) => {
                if sessions.is_empty() {
                    "No saved sessions. Use /save name to create one.".to_string()
                } else {
                    format!("Saved sessions:\n{}", sessions.join("\n"))
                }
            }
            Err(e) => format!("Could not list sessions: {}", e),
        }
    }

    fn handle_compact(&mut self, custom: Option<&str>) -> String {
        let keep = 2000usize;
        let msg = self.compact(keep);
        if let Some(instruction) = custom {
            format!("{} (focus: {})", msg, instruction)
        } else {
            msg
        }
    }

    fn handle_history_cmd(&self) -> String {
        if self.input_history.is_empty() {
            return "No history. Commands you send become part of your history.".to_string();
        }
        let count = self.input_history.len();
        let last = self.input_history.iter().rev().take(10).collect::<Vec<_>>();
        let entries: Vec<String> = last
            .iter()
            .enumerate()
            .map(|(i, e)| format!("{}: {}", i + 1, e))
            .collect();
        format!(
            "History ({} total, showing last 10). Use ↑/↓ to navigate.\n{}",
            count,
            entries.join("\n")
        )
    }

    pub fn help_text(&self) -> String {
        format!(
            "Commands:\n\
            /model [provider/model|model] — switch model (current: {}/{})\n\
            /save name — save current session\n\
            /load name — load a saved session\n\
            /sessions — list saved sessions\n\
            /delete name — delete a saved session\n\
            /compact [prompt] — compact older messages\n\
            /reset — clear all state\n\
            /help — show this help\n\
            /history — show recent history\n\
            Use Up/Down to navigate history.\n\
            Enter — send | Alt+Enter — follow-up | Esc — abort | Ctrl+S — steer",
            self.current_provider, self.current_model
        )
    }
}