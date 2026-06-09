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
    ScopedModels,
}

/// Active dialog state with per-dialog data.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogState {
    CommandPalette {
        filter: String,
        selected: usize,
    },
    ModelSelector {
        filter: String,
        selected: usize,
    },
    Settings {
        category: crate::settings::SettingsCategory,
        selected: usize,
    },
    ScopedModels {
        selected: usize,
    },
    SessionTree {
        filter: crate::session_tree::SessionTreeFilter,
        selected: usize,
    },
    PanelStack(crate::dialog::PanelStack),
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

    pub fn clear(&mut self) {
        self.commands.clear();
        self.aliases.clear();
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
mod tests;
