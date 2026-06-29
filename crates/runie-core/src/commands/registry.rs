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

impl DialogState {
    /// Access the panel stack if this is an `Active` dialog.
    pub fn panel_stack(&self) -> Option<&PanelStack> {
        match self {
            DialogState::Welcome => None,
            DialogState::Active { kind: _, panels } => Some(panels),
        }
    }

    /// Mutably access the panel stack if this is an `Active` dialog.
    pub fn panel_stack_mut(&mut self) -> Option<&mut PanelStack> {
        match self {
            DialogState::Welcome => None,
            DialogState::Active { kind: _, panels } => Some(panels),
        }
    }
}

// ============================================================================
// DialogState state machine tests
// ============================================================================

#[cfg(test)]
mod dialog_state_tests {
    use super::*;

    #[test]
    fn dialog_panel_stack_accessor() {
        // Welcome state has no panel stack
        let mut welcome = DialogState::Welcome;
        assert!(welcome.panel_stack().is_none());
        assert!(welcome.panel_stack_mut().is_none());

        // Active state exposes the panel stack
        let stack = PanelStack::new(crate::dialog::Panel::new("test", "Test"));
        let active = DialogState::Active { kind: DialogKind::CommandPalette, panels: stack.clone() };
        assert_eq!(active.panel_stack(), Some(&stack));
    }

    #[test]
    fn dialog_transitions_are_valid() {
        let welcome = DialogState::Welcome;
        assert!(welcome.panel_stack().is_none());

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
            assert!(active.panel_stack().is_some(), "{kind:?} should have a panel stack");
        }
    }

    #[test]
    fn dialog_prompt_data_unique() {
        let welcome = DialogState::Welcome;
        let welcome2 = DialogState::Welcome;
        assert_eq!(welcome, welcome2);

        let stack = PanelStack::new(crate::dialog::Panel::new("a", "A"));
        let active_a = DialogState::Active { kind: DialogKind::CommandPalette, panels: stack.clone() };
        let active_b = DialogState::Active { kind: DialogKind::ModelSelector, panels: stack.clone() };
        assert_ne!(active_a, active_b);

        let stack2 = PanelStack::new(crate::dialog::Panel::new("b", "B"));
        let active_c = DialogState::Active { kind: DialogKind::CommandPalette, panels: stack2 };
        assert_ne!(active_a, active_c);
    }

    #[test]
    fn active_variant_carries_kind() {
        let stack = PanelStack::new(crate::dialog::Panel::new("x", "X"));
        let active = DialogState::Active { kind: DialogKind::Settings, panels: stack };
        assert!(active.panel_stack().is_some());
        assert!(!matches!(active, DialogState::Welcome));
    }
}

// ============================================================================
// Command Dispatch
// ============================================================================

impl AppState {
    /// Dispatch a slash command
    pub fn handle_slash(&mut self, content: &str) -> Option<CommandResult> {
        if !content.starts_with('/') {
            return None;
        }

        let input = content.trim_start_matches('/');
        let (name, args) = input.split_once(' ').unwrap_or((input, ""));

        match self.registry().get(name) {
            Some(spec) => {
                let (cmd_name, flow) = (spec.name.clone(), spec.flow.clone());
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
