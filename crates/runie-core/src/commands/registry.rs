//! Command Registry - manages command registration and dispatch

use crate::model::AppState;
use crate::dialog::PanelStack;
use std::collections::HashMap;
use super::{CommandCategory, CommandDef, CommandResult, DialogType, CommandFlow};

/// Registry of all commands
#[derive(Clone)]
pub struct CommandRegistry {
    commands: HashMap<String, CommandDef>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self { commands: HashMap::new(), aliases: HashMap::new() };
        super::handlers::register_all(&mut registry);
        registry
    }
    
    pub fn register(&mut self, def: CommandDef) {
        for alias in &def.aliases {
            self.aliases.insert(alias.clone(), def.name.clone());
        }
        self.commands.insert(def.name.clone(), def);
    }
    
    pub fn get(&self, name: &str) -> Option<&CommandDef> {
        self.commands.get(name)
            .or_else(|| self.aliases.get(name).and_then(|n| self.commands.get(n)))
    }
    
    pub fn list(&self) -> Vec<&CommandDef> {
        let mut defs: Vec<_> = self.commands.values().collect();
        defs.sort_by_key(|d| (&d.category, &d.name));
        defs
    }
    
    pub fn list_by_category(&self) -> Vec<(CommandCategory, Vec<&CommandDef>)> {
        let mut result: Vec<(CommandCategory, Vec<&CommandDef>)> = Vec::new();
        for def in self.list() {
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
        let mut lines = vec!["Commands:".into()];
        for (cat, defs) in self.list_by_category() {
            lines.push(format!("\n  {}", cat.label()));
            for def in defs {
                let aliases = if def.aliases.is_empty() {
                    String::new()
                } else {
                    format!(", {}", def.aliases.join(", "))
                };
                lines.push(format!("  /{}{} — {}", def.name, aliases, def.desc));
            }
        }
        lines.push(format!(
            "\nCurrent model: {}/{}\nUse Up/Down to navigate history.\nEnter — send | Alt+Enter — follow-up | Esc — abort | Ctrl+S — steer",
            provider, model
        ));
        lines.join("\n")
    }
}

impl Default for CommandRegistry {
    fn default() -> Self { Self::new() }
}

/// Filter commands by name/description
pub fn filter_commands<'a>(reg: &'a CommandRegistry, query: &str) -> Vec<&'a CommandDef> {
    let q = query.to_lowercase();
    reg.list().into_iter()
        .filter(|d| d.name.to_lowercase().contains(&q) || d.desc.to_lowercase().contains(&q))
        .collect()
}

// ============================================================================
// Dialog State (for UI layer)
// ============================================================================

/// Active dialog state
#[derive(Debug, Clone, PartialEq)]
pub enum DialogState {
    CommandPalette { filter: String, selected: usize },
    ModelSelector { filter: String, selected: usize },
    Settings { category: crate::settings::SettingsCategory, selected: usize },
    ScopedModels { selected: usize },
    SessionTree { filter: crate::session_tree::SessionTreeFilter, selected: usize },
    PanelStack(PanelStack),
}

impl DialogState {
    /// Return the underlying panel stack for panel-based dialogs.
    pub fn panel_stack(&self) -> Option<&PanelStack> {
        match self {
            DialogState::PanelStack(stack) => Some(stack),
            _ => None,
        }
    }
}

// ============================================================================
// Command Dispatch
// ============================================================================

impl AppState {
    /// Dispatch a slash command
    pub(crate) fn handle_slash(&mut self, content: &str) -> Option<CommandResult> {
        if !content.starts_with('/') { return None; }
        
        let input = content.trim_start_matches('/');
        let (name, args) = input.split_once(' ').unwrap_or((input, ""));
        
        match self.registry.get(name) {
            Some(cmd) => {
                let cmd_name = cmd.name.clone();
                let result = cmd.flow.clone().exec(self, &cmd_name, args);
                if matches!(result, CommandResult::None) { None } else { Some(result) }
            }
            None => Some(CommandResult::Message(format!("Unknown command: /{name}. Try /help."))),
        }
    }
}
