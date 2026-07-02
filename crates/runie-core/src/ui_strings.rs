//! Centralized user-facing strings for Runie.
//!
//! All user-facing messages (errors, warnings, info, help text, prompts)
//! should be defined here to ensure consistency and ease of localization.
//!
//! String constants use `&'static str` for static strings and
//! functions for dynamic messages (format strings).

/// Session command strings.
pub mod session {
    /// Message when no saved sessions exist.
    pub const NO_SAVED_SESSIONS: &str = "No saved sessions. Use /save name to create one.";
    /// Format for listing saved sessions.
    pub fn saved_sessions(sessions: &[String]) -> String {
        format!("Saved sessions:\n{}", sessions.join("\n"))
    }
    /// Format for session list error.
    pub fn session_list_error(err: &str) -> String {
        format!("Could not list sessions: {}", err)
    }
    /// Message for session reset.
    pub const STATE_CLEARED: &str = "State cleared.";
    /// Message when history is empty.
    pub const NO_HISTORY: &str = "No history.";
    /// Format for conversation history.
    pub fn history(total: usize, entries: &[String]) -> String {
        format!("Conversation history ({} messages):\n{}", total, entries.join("\n"))
    }
    /// Format for session info.
    pub fn session_info(info: &str) -> String {
        info.to_string()
    }
    /// Format for save usage error.
    pub const SAVE_USAGE: &str = "Usage: /save name";
    /// Format for save in progress.
    pub fn saving_session(name: &str) -> String {
        format!("Saving session '{}'…", name)
    }
    /// Format for save success.
    pub fn session_saved(name: &str) -> String {
        format!("Session '{}' saved.", name)
    }
    /// Format for save error.
    pub fn save_error(err: &str) -> String {
        format!("Could not save session: {}", err)
    }
    /// Format for load usage error.
    pub const LOAD_USAGE: &str = "Usage: /load name";
    /// Format for load success.
    pub fn session_loaded(name: &str) -> String {
        format!("Session '{}' loaded.", name)
    }
    /// Format for load not found.
    pub fn session_not_found(name: &str) -> String {
        format!("Session '{}' not found. Use /sessions to list saved sessions.", name)
    }
    /// Format for delete usage error.
    pub const DELETE_USAGE: &str = "Usage: /delete name";
    /// Format for delete success.
    pub fn session_deleted(name: &str) -> String {
        format!("Session '{}' deleted.", name)
    }
    /// Format for import usage error.
    pub const IMPORT_USAGE: &str = "Usage: /import path/to/session.json";
    /// Format for import in progress.
    pub fn importing_session(path: &str) -> String {
        format!("Importing session from '{}'…", path)
    }
    /// Format for import read error.
    pub fn import_read_error(path: &str) -> String {
        format!("Could not read '{}'", path)
    }
    /// Format for import success.
    pub fn session_imported(path: &str) -> String {
        format!("Session imported from '{}'", path)
    }
    /// Format for import parse error.
    pub fn import_error(path: &str) -> String {
        format!("Could not import session from '{}'", path)
    }
    /// Format for export usage error.
    pub const EXPORT_USAGE: &str = "Usage: /export path/to/session.json";
    /// Format for export in progress.
    pub fn exporting_session(path: &str) -> String {
        format!("Exporting session to '{}'…", path)
    }
    /// Format for export success.
    pub fn session_exported(path: &str) -> String {
        format!("Session exported to '{}'", path)
    }
    /// Format for export error.
    pub fn export_error(err: &str) -> String {
        format!("Could not export: {}", err)
    }
    /// Message when no session store is available.
    pub const NO_SESSION_STORE: &str = "No session store available.";
    /// Message when resume load fails.
    pub const RESUME_LOAD_FAILED: &str = "Could not load session.";
    /// Message when no sessions to resume.
    pub const NO_SESSIONS_TO_RESUME: &str = "No sessions to resume.";
    /// Format for resume success.
    pub fn session_resumed(name: &str) -> String {
        format!("Loaded '{}'.", name)
    }
    /// Message for new session started.
    pub const NEW_SESSION_STARTED: &str = "New session started";
}

/// Model command strings.
pub mod model {
    /// Message when no providers are connected.
    pub const NO_PROVIDERS: &str = "No connected providers. Use /provider to add a provider first.";
    /// Format for model usage.
    pub fn usage(provider: &str, model: &str) -> String {
        format!(
            "Current: {}/{}. Format: /model provider/model or /model model",
            provider, model
        )
    }
    /// Format for model unavailable warning.
    pub fn model_unavailable(provider: &str, model: &str) -> String {
        format!(
            "Model {}/{} is not available. Connect the provider and choose models with /provider.",
            provider, model
        )
    }
    /// Format for model switched.
    pub fn model_switched(provider: &str, model: &str) -> String {
        format!("Switched to {}/{}", provider, model)
    }
    /// Format for thinking level set.
    pub fn thinking_level(level: &str) -> String {
        format!("Thinking level set to: {}", level)
    }
    /// Format for thinking error.
    pub fn thinking_error(err: &str) -> String {
        format!("Error: {}", err)
    }
}

/// System command strings.
pub mod system {
    /// Message when nothing to copy.
    pub const NOTHING_TO_COPY: &str = "No assistant response to copy";
    /// Message when no skills loaded.
    pub const NO_SKILLS: &str = "No skills loaded.";
    /// Format for loaded skills header.
    pub const LOADED_SKILLS: &str = "Loaded skills:";
    /// Format for skill info.
    pub fn skill_info(name: &str, description: Option<&str>, context: Option<&str>) -> String {
        let mut lines = vec![format!("Skill: {}", name)];
        if let Some(desc) = description {
            if !desc.is_empty() {
                lines.push(format!("Description: {}", desc));
            }
        }
        if let Some(ctx) = context {
            if !ctx.is_empty() {
                lines.push(format!("Context: {}", ctx));
            }
        }
        lines.join("\n")
    }
    /// Format for skill not found.
    pub fn skill_not_found(name: &str) -> String {
        format!("Skill '{}' not found. Use /skills.", name)
    }
}

/// Command parsing strings.
pub mod commands {
    /// Format for invalid command syntax.
    pub fn invalid_syntax(err: &str) -> String {
        format!("Invalid command syntax: {}. Try /help.", err)
    }
    /// Format for unknown command.
    pub fn unknown_command(name: &str) -> String {
        format!("Unknown command: /{}. Try /help.", name)
    }
}

/// Help panel strings.
pub mod help {
    /// Header for the help filter.
    pub const FILTER_HEADER: &str = "Type to filter · Esc closes";
}

/// Trust status strings.
pub mod trust {
    /// Trust status for trusted project.
    pub const STATUS_TRUSTED: &str = "trusted";
    /// Trust status for untrusted project.
    pub const STATUS_UNTRUSTED: &str = "untrusted";
    /// Trust status for default.
    pub const STATUS_DEFAULT: &str = "default";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_strings_are_static() {
        // Ensure static strings are non-empty
        assert!(!session::NO_SAVED_SESSIONS.is_empty());
        assert!(!session::STATE_CLEARED.is_empty());
        assert!(!session::NO_HISTORY.is_empty());
        assert!(!session::SAVE_USAGE.is_empty());
        assert!(!session::LOAD_USAGE.is_empty());
        assert!(!session::DELETE_USAGE.is_empty());
        assert!(!session::IMPORT_USAGE.is_empty());
        assert!(!session::EXPORT_USAGE.is_empty());
    }

    #[test]
    fn model_strings_are_static() {
        assert!(!model::NO_PROVIDERS.is_empty());
    }

    #[test]
    fn system_strings_are_static() {
        assert!(!system::NOTHING_TO_COPY.is_empty());
        assert!(!system::NO_SKILLS.is_empty());
    }

    #[test]
    fn format_functions_work() {
        assert!(session::session_saved("test").contains("test"));
        assert!(model::model_switched("openai", "gpt-4").contains("gpt-4"));
        assert!(system::skill_not_found("xyz").contains("xyz"));
    }

    #[test]
    fn help_strings_are_static() {
        assert!(!help::FILTER_HEADER.is_empty());
    }
}
