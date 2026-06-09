//! Command Registry — unified slash commands and command palette

use crate::model::AppState;
use crate::Event;
use std::collections::HashMap;

pub mod handlers;

/// Dialog types that can be opened via commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    CommandPalette,
    ModelSelector,
    Settings,
}

/// Active dialog state with per-dialog data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogState {
    CommandPalette {
        filter: String,
        selected: usize,
    },
    ModelSelector {
        filter: String,
        selected: usize,
    },
    Settings,
}

/// Result of executing a slash command
#[derive(Debug, Clone, PartialEq)]
pub enum CommandResult {
    Message(String),
    Event(Event),
    OpenDialog(Dialog),
    None,
}

/// Command category for grouping in help and palette
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandCategory {
    Session,
    Model,
    Tool,
    System,
    Help,
}

impl CommandCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            CommandCategory::Session => "Session",
            CommandCategory::Model => "Model",
            CommandCategory::Tool => "Tool",
            CommandCategory::System => "System",
            CommandCategory::Help => "Help",
        }
    }
}

/// Handler function signature for commands
pub type CommandHandler = fn(&mut AppState, &str) -> CommandResult;

/// Optional completer for command arguments
pub type CommandCompleter = fn(&str) -> Vec<String>;

/// Definition of a single command
#[derive(Clone)]
pub struct CommandDef {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub category: CommandCategory,
    pub handler: CommandHandler,
    pub completer: Option<CommandCompleter>,
}

/// Registry of all available commands
#[derive(Clone)]
pub struct CommandRegistry {
    commands: HashMap<String, CommandDef>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        };
        handlers::register_all(&mut registry);
        registry
    }

    pub fn register(&mut self, def: CommandDef) {
        for alias in &def.aliases {
            self.aliases.insert(alias.clone(), def.name.clone());
        }
        self.commands.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&CommandDef> {
        self.commands
            .get(name)
            .or_else(|| self.aliases.get(name).and_then(|n| self.commands.get(n)))
    }

    pub fn list(&self) -> Vec<&CommandDef> {
        let mut defs: Vec<&CommandDef> = self.commands.values().collect();
        defs.sort_by_key(|d| (&d.category, &d.name));
        defs
    }

    pub fn list_by_category(&self) -> Vec<(CommandCategory, Vec<&CommandDef>)> {
        let defs = self.list();
        let mut result: Vec<(CommandCategory, Vec<&CommandDef>)> = Vec::new();
        for def in defs {
            if let Some(last) = result.last_mut() {
                if last.0 == def.category {
                    last.1.push(def);
                    continue;
                }
            }
            result.push((def.category.clone(), vec![def]));
        }
        result
    }

    pub fn help_text(&self, provider: &str, model: &str) -> String {
        let mut lines = vec!["Commands:".to_string()];
        for (category, defs) in self.list_by_category() {
            lines.push(format!("\n  {}", category.as_str()));
            for def in defs {
                let alias_str = if def.aliases.is_empty() {
                    String::new()
                } else {
                    format!(", {}", def.aliases.join(", "))
                };
                lines.push(format!(
                    "  /{}{} — {}",
                    def.name, alias_str, def.description
                ));
            }
        }
        lines.push(format!(
            "\nCurrent model: {}/{}\nUse Up/Down to navigate history.\nEnter — send | Alt+Enter — follow-up | Esc — abort | Ctrl+S — steer",
            provider, model
        ));
        lines.join("\n")
    }
}

/// Filter commands by name or description (case-insensitive).
pub fn filter_commands<'a>(registry: &'a CommandRegistry, query: &str) -> Vec<&'a CommandDef> {
    let q = query.to_lowercase();
    registry
        .list()
        .into_iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q)
                || e.description.to_lowercase().contains(&q)
        })
        .collect()
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Dispatch a slash command through the registry.
    /// Returns `None` for non-slash input so caller can treat it as a user message.
    pub(crate) fn handle_slash(&mut self, content: &str) -> Option<CommandResult> {
        if !content.starts_with('/') {
            return None;
        }
        let input = content.trim_start_matches('/');
        let (name, args) = input.split_once(' ').unwrap_or((input, ""));
        match self.registry.get(name) {
            Some(cmd) => Some((cmd.handler)(self, args)),
            None => Some(CommandResult::Message(format!(
                "Unknown command: /{name}. Try /help."
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_get_by_name() {
        let state = AppState::default();
        let cmd = state.registry.get("model");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().name, "model");
    }

    #[test]
    fn registry_get_by_alias() {
        let state = AppState::default();
        let cmd = state.registry.get("m");
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().name, "model");
    }

    #[test]
    fn registry_list_returns_all() {
        let state = AppState::default();
        let defs = state.registry.list();
        assert!(
            defs.len() >= 22,
            "registry should have 22+ commands, got {}",
            defs.len()
        );
    }

    #[test]
    fn registry_list_groups_by_category() {
        let state = AppState::default();
        let groups = state.registry.list_by_category();
        assert!(!groups.is_empty());
        let total: usize = groups.iter().map(|g| g.1.len()).sum();
        assert!(total >= 22);
    }

    #[test]
    fn handler_model_switches() {
        let mut state = AppState::default();
        let cmd = state.registry.get("model").unwrap();
        let result = (cmd.handler)(&mut state, "gpt-4o");
        assert_eq!(state.current_model, "gpt-4o");
        assert!(matches!(result, CommandResult::Message(_)));
    }

    #[test]
    fn handler_help_generates_list() {
        let mut state = AppState::default();
        let cmd = state.registry.get("help").unwrap();
        let result = (cmd.handler)(&mut state, "");
        if let CommandResult::Message(msg) = result {
            assert!(msg.contains("Commands:"));
            assert!(msg.contains("/model"));
            assert!(msg.contains("/save"));
        } else {
            panic!("help should return Message, got {:?}", result);
        }
    }

    #[test]
    fn handler_quit_sets_flag() {
        let mut state = AppState::default();
        let cmd = state.registry.get("quit").unwrap();
        let result = (cmd.handler)(&mut state, "");
        assert!(matches!(result, CommandResult::Event(Event::Quit)));
        state.update(Event::Quit);
        assert!(state.should_quit);
    }

    #[test]
    fn unknown_command_returns_error() {
        let mut state = AppState::default();
        let result = state.handle_slash("/foo");
        assert!(matches!(result, Some(CommandResult::Message(msg)) if msg.contains("Unknown command")));
    }

    #[test]
    fn slash_event_dispatches_to_registry() {
        let mut state = AppState::default();
        type_str(&mut state, "/model gpt-4o");
        state.update(Event::Submit);
        assert_eq!(state.current_model, "gpt-4o");
    }

    #[test]
    fn alias_event_dispatches_correctly() {
        let mut state = AppState::default();
        type_str(&mut state, "/m gpt-4o");
        state.update(Event::Submit);
        assert_eq!(state.current_model, "gpt-4o");
    }

    // Palette filter tests (Layer 1)

    #[test]
    fn filter_empty_shows_all() {
        let state = AppState::default();
        let all = state.registry.list();
        let filtered = filter_commands(&state.registry, "");
        assert_eq!(filtered.len(), all.len());
    }

    #[test]
    fn filter_matches_name() {
        let state = AppState::default();
        let filtered = filter_commands(&state.registry, "comp");
        assert!(
            filtered.iter().any(|c| c.name == "compact"),
            "'comp' should match 'compact'"
        );
    }

    #[test]
    fn filter_matches_description() {
        let state = AppState::default();
        let filtered = filter_commands(&state.registry, "copy");
        assert!(
            filtered.iter().any(|c| c.name == "copy"),
            "'copy' should match 'copy' command description"
        );
    }

    #[test]
    fn filter_case_insensitive() {
        let state = AppState::default();
        let lower = filter_commands(&state.registry, "comp");
        let upper = filter_commands(&state.registry, "COMP");
        assert_eq!(lower.len(), upper.len());
        assert!(upper.iter().any(|c| c.name == "compact"));
    }

    #[test]
    fn select_wraps_up() {
        let mut state = AppState::default();
        state.update(Event::ToggleCommandPalette);
        // Up at first should wrap to last
        state.update(Event::PaletteUp);
        let count = filter_commands(&state.registry, "").len();
        if let Some(DialogState::CommandPalette { selected, .. }) = &state.open_dialog {
            assert_eq!(*selected, count - 1, "Up at first should wrap to last");
        } else {
            panic!("Palette should be open");
        }
    }

    #[test]
    fn select_wraps_down() {
        let mut state = AppState::default();
        state.update(Event::ToggleCommandPalette);
        let count = filter_commands(&state.registry, "").len();
        // Down at last should wrap to first
        for _ in 0..count {
            state.update(Event::PaletteDown);
        }
        if let Some(DialogState::CommandPalette { selected, .. }) = &state.open_dialog {
            assert_eq!(*selected, 0, "Down at last should wrap to first");
        } else {
            panic!("Palette should be open");
        }
    }

    fn type_str(state: &mut AppState, text: &str) {
        for c in text.chars() {
            state.update(Event::Input(c));
        }
    }

    // Session info tests (Layer 1)

    #[test]
    fn session_info_counts_messages() {
        let mut state = AppState::default();
        state.messages = vec![
            crate::model::ChatMessage { role: crate::model::Role::User, content: "hi".into(), timestamp: 0.0, id: "u1".into(), ..Default::default()},
            crate::model::ChatMessage { role: crate::model::Role::Assistant, content: "hello".into(), timestamp: 0.0, id: "a1".into(), ..Default::default()},
            crate::model::ChatMessage { role: crate::model::Role::Tool, content: "tool out".into(), timestamp: 0.0, id: "t1".into(), ..Default::default()},
            crate::model::ChatMessage { role: crate::model::Role::User, content: "again".into(), timestamp: 0.0, id: "u2".into(), ..Default::default()},
        ];
        let cmd = state.registry.get("session").unwrap();
        let result = (cmd.handler)(&mut state, "");
        if let CommandResult::Message(msg) = result {
            assert!(msg.contains("Messages: 4 total (2 user, 1 assistant, 1 tool)"), "got: {}", msg);
        } else {
            panic!("session should return Message, got {:?}", result);
        }
    }

    #[test]
    fn session_info_shows_tokens() {
        let mut state = AppState::default();
        state.messages = vec![
            crate::model::ChatMessage { role: crate::model::Role::User, content: "hello world".into(), timestamp: 0.0, id: "u1".into(), ..Default::default()},
        ];
        let cmd = state.registry.get("session").unwrap();
        let result = (cmd.handler)(&mut state, "");
        if let CommandResult::Message(msg) = result {
            assert!(msg.contains("Tokens:"), "Token estimate should be present, got: {}", msg);
        } else {
            panic!("session should return Message, got {:?}", result);
        }
    }

    #[test]
    fn slash_session_dispatches() {
        let mut state = AppState::default();
        state.messages.push(crate::model::ChatMessage { role: crate::model::Role::User, content: "test".into(), timestamp: 0.0, id: "u1".into(), ..Default::default()});
        type_str(&mut state, "/session");
        state.update(Event::Submit);
        let last = state.messages.last().unwrap();
        assert_eq!(last.role, crate::model::Role::System);
        assert!(last.content.contains("Messages:"));
    }
}
