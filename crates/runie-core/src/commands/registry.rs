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
        super::dsl::handlers::register_all(&mut registry);
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

/// Kind of active dialog — which panel was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    CommandPalette,
    ModelSelector,
    Settings,
    ScopedModels,
    SessionTree,
    Generic,
}

/// Active dialog state — collapsed from 7 variants to 2.
/// `Welcome` is the initial screen; `Active` covers all panel-backed dialogs.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogState {
    Welcome,
    Active { kind: DialogKind, panels: PanelStack },
}

macro_rules! with_panel_stack {
    ($self:expr_2021, $stack:ident, $body:expr_2021) => {
        match $self {
            DialogState::Welcome => None,
            DialogState::Active { kind: _, panels: $stack } => Some($body),
        }
    };
}

impl DialogState {
    pub fn panel_stack(&self) -> Option<&PanelStack> {
        with_panel_stack!(self, s, s)
    }

    pub fn panel_stack_mut(&mut self) -> Option<&mut PanelStack> {
        with_panel_stack!(self, s, s)
    }
}

// ============================================================================
// DialogState state machine tests
// ============================================================================

#[cfg(test)]
mod dialog_state_tests {
    use super::*;

    // Layer 1: dialog_transitions_are_valid
    // The with_panel_stack! macro provides a uniform interface for all PanelStack variants.
    // Invalid transitions: trying to get panels from Welcome returns None.
    #[test]
    fn dialog_transitions_are_valid() {
        // Welcome has no panel stack — panel_stack() returns None
        let welcome = DialogState::Welcome;
        assert!(welcome.panel_stack().is_none());

        // Active variants all have a panel stack
        let stack = PanelStack::new(crate::dialog::Panel::new("test", "Test"));
        for kind in [
            DialogKind::CommandPalette,
            DialogKind::ModelSelector,
            DialogKind::Settings,
            DialogKind::ScopedModels,
            DialogKind::SessionTree,
            DialogKind::Generic,
        ] {
            let active = DialogState::Active { kind, panels: stack.clone() };
            assert!(
                active.panel_stack().is_some(),
                "{kind:?} should have a panel stack"
            );
        }
    }

    // Layer 1: dialog_prompt_data_unique
    // Only the Active variant carries panel data; Welcome carries none.
    #[test]
    fn dialog_prompt_data_unique() {
        // Welcome stores no panel data
        let welcome = DialogState::Welcome;
        let welcome2 = DialogState::Welcome;
        assert_eq!(welcome, welcome2); // Zero data variant

        // Active variants carry their kind AND panels
        let stack = PanelStack::new(crate::dialog::Panel::new("a", "A"));
        let active_a = DialogState::Active { kind: DialogKind::CommandPalette, panels: stack.clone() };
        let active_b = DialogState::Active { kind: DialogKind::ModelSelector, panels: stack.clone() };
        assert_ne!(active_a, active_b, "Different kinds are different states");

        let stack2 = PanelStack::new(crate::dialog::Panel::new("b", "B"));
        let active_c = DialogState::Active { kind: DialogKind::CommandPalette, panels: stack2 };
        assert_ne!(active_a, active_c, "Different panels are different states");
    }

    // Layer 1: Active variant exposes its kind
    #[test]
    fn active_variant_carries_kind() {
        let stack = PanelStack::new(crate::dialog::Panel::new("x", "X"));
        let active = DialogState::Active { kind: DialogKind::Settings, panels: stack };
        // panel_stack() confirms we have the stack
        assert!(active.panel_stack().is_some());
        // Active is not Welcome
        assert!(!matches!(active, DialogState::Welcome));
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

        match self.registry().get(name) {
            Some(cmd) => {
                let (cmd_name, flow) = (cmd.name.clone(), cmd.flow.clone());
                // Track usage for ranking
                self.record_command_usage(&cmd_name);
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
