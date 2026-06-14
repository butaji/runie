//! Command Registry - manages command registration and dispatch

use super::{CommandCategory, CommandDef, CommandResult};
use crate::dialog::PanelStack;
use crate::model::AppState;
use std::collections::HashMap;

/// Registry of all commands
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
        self.commands
            .get(name)
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
            result.push((def.category, vec![def]));
        }
        result
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter commands by name/description
pub fn filter_commands<'a>(reg: &'a CommandRegistry, query: &str) -> Vec<&'a CommandDef> {
    let q = query.to_lowercase();
    reg.list()
        .into_iter()
        .filter(|d| d.name.to_lowercase().contains(&q) || d.desc.to_lowercase().contains(&q))
        .collect()
}

// ============================================================================
// Dialog State (for UI layer)
// ============================================================================

/// Active dialog state — all variants backed by PanelStack.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogState {
    CommandPalette(PanelStack),
    ModelSelector(PanelStack),
    Settings(PanelStack),
    ScopedModels(PanelStack),
    SessionTree(PanelStack),
    PanelStack(PanelStack),
}

macro_rules! with_panel_stack {
    ($self:expr, $stack:ident, $body:expr) => {
        match $self {
            DialogState::CommandPalette($stack) => $body,
            DialogState::ModelSelector($stack) => $body,
            DialogState::Settings($stack) => $body,
            DialogState::ScopedModels($stack) => $body,
            DialogState::SessionTree($stack) => $body,
            DialogState::PanelStack($stack) => $body,
        }
    };
}

impl DialogState {
    pub fn panel_stack(&self) -> &PanelStack {
        with_panel_stack!(self, s, s)
    }

    pub fn panel_stack_mut(&mut self) -> &mut PanelStack {
        with_panel_stack!(self, s, s)
    }
}

// ============================================================================
// Command Dispatch
// ============================================================================

impl AppState {
    /// Dispatch a slash command
    pub(crate) fn handle_slash(&mut self, content: &str) -> Option<CommandResult> {
        if !content.starts_with('/') {
            return None;
        }

        let input = content.trim_start_matches('/');
        let (name, args) = input.split_once(' ').unwrap_or((input, ""));

        match self.registry.get(name) {
            Some(cmd) => {
                let (cmd_name, flow) = (cmd.name.clone(), cmd.flow.clone());
                let result = flow.exec(self, &cmd_name, args);
                if matches!(result, CommandResult::None) {
                    None
                } else {
                    Some(result)
                }
            }
            None => Some(CommandResult::Message(format!(
                "Unknown command: /{name}. Try /help."
            ))),
        }
    }
}
