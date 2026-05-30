mod render;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_bugs;
#[cfg(test)]
mod tests_scoring;
#[cfg(test)]
mod tests_registry;

pub mod builder;
pub use builder::*;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
};
use crate::theme::ThemeWrapper;
use std::time::Instant;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteCommand {
    NewSession,
    ClearChat,
    SwitchModel,
    Quit,
    Cancel,
}

#[derive(Debug, Clone)]
pub struct CommandUsage {
    pub use_count: u32,
    pub last_used: Option<Instant>,
}

#[derive(Debug, Clone)]
pub struct PaletteCommandDef {
    pub id: String,
    pub label: String,
    pub description: String,
    pub category: String,
    pub aliases: Vec<String>,
    pub keybinding: Option<String>,
    pub requires_args: bool,
    pub arg_hint: String,
}

impl PaletteCommandDef {
    fn to_palette_command(&self, _arg: &str) -> PaletteCommand {
        match self.id.as_str() {
            "new_session" => PaletteCommand::NewSession,
            "clear_chat" => PaletteCommand::ClearChat,
            "switch_model" => PaletteCommand::SwitchModel,
            "quit" => PaletteCommand::Quit,
            _ => PaletteCommand::Cancel,
        }
    }
}

#[derive(Clone)]
pub struct CommandPalette {
    requires_args: HashMap<String, PaletteCommandDef>,
    all_commands: Vec<PaletteCommandDef>,
    pub filtered_commands: Vec<usize>,
    pub selected: usize,
    pub argument_input: String,
    pub is_argument_mode: bool,
    pub pending_command: Option<String>,
    usage_stats: HashMap<String, CommandUsage>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        let all_commands = vec![
            PaletteCommandDef { id: "new_session".into(), label: "New Session".into(), description: "Start a fresh chat session".into(), category: "session".into(), aliases: vec!["n".into(), "new".into()], keybinding: Some("Ctrl+N".into()), requires_args: false, arg_hint: "".into() },
            PaletteCommandDef { id: "clear_chat".into(), label: "Clear Chat".into(), description: "Clear all messages".into(), category: "chat".into(), aliases: vec!["c".into(), "clear".into()], keybinding: Some("Ctrl+L".into()), requires_args: false, arg_hint: "".into() },
            PaletteCommandDef { id: "switch_model".into(), label: "Switch Model...".into(), description: "Change the AI model".into(), category: "config".into(), aliases: vec!["model".into(), "m".into()], keybinding: None, requires_args: false, arg_hint: "model name".into() },
            PaletteCommandDef { id: "quit".into(), label: "Quit".into(), description: "Exit the application".into(), category: "app".into(), aliases: vec!["q".into(), "quit".into(), "exit".into()], keybinding: Some("Ctrl+Q".into()), requires_args: false, arg_hint: "".into() },
        ];

        let mut requires_args_map = HashMap::new();
        for cmd in &all_commands {
            if cmd.requires_args {
                requires_args_map.insert(cmd.id.clone(), cmd.clone());
            }
        }

        Self { requires_args: requires_args_map, all_commands, filtered_commands: Vec::new(), selected: 0, argument_input: String::new(), is_argument_mode: false, pending_command: None, usage_stats: HashMap::new() }
    }

    pub fn all_commands(&self) -> &[PaletteCommandDef] { &self.all_commands }

    fn fuzzy_score(query: &str, command: &PaletteCommandDef) -> f32 {
        let query_lower = query.to_lowercase();
        if query_lower.is_empty() {
            return 1.0;
        }

        // Tier 1: Exact match on label or id
        if let Some(score) = Self::exact_match_score(&query_lower, command) {
            return score;
        }

        // Tier 2: Prefix match on label or id
        if let Some(score) = Self::prefix_match_score(&query_lower, command) {
            return score;
        }

        // Tier 3: Contains match on label or id
        if let Some(score) = Self::contains_match_score(&query_lower, command) {
            return score;
        }

        // Tier 4: Alias matching
        if let Some(score) = Self::alias_match_score(&query_lower, command) {
            return score;
        }

        // Tier 5: Fuzzy character match
        Self::fuzzy_match_score(&query_lower, &command.label)
    }

    fn exact_match_score(query: &str, command: &PaletteCommandDef) -> Option<f32> {
        let label_lower = command.label.to_lowercase();
        let id_lower = command.id.to_lowercase();
        if label_lower == query || id_lower == query {
            Some(1000.0)
        } else {
            None
        }
    }

    fn prefix_match_score(query: &str, command: &PaletteCommandDef) -> Option<f32> {
        let label_lower = command.label.to_lowercase();
        let id_lower = command.id.to_lowercase();
        if label_lower.starts_with(query) || id_lower.starts_with(query) {
            Some(900.0)
        } else {
            None
        }
    }

    fn contains_match_score(query: &str, command: &PaletteCommandDef) -> Option<f32> {
        let label_lower = command.label.to_lowercase();
        let id_lower = command.id.to_lowercase();
        if label_lower.contains(query) || id_lower.contains(query) {
            Some(700.0)
        } else {
            None
        }
    }

    fn alias_match_score(query: &str, command: &PaletteCommandDef) -> Option<f32> {
        for alias in &command.aliases {
            let alias_lower = alias.to_lowercase();
            if alias_lower == query {
                return Some(850.0);
            }
        }
        for alias in &command.aliases {
            let alias_lower = alias.to_lowercase();
            if alias_lower.starts_with(query) {
                return Some(800.0);
            }
        }
        for alias in &command.aliases {
            let alias_lower = alias.to_lowercase();
            if alias_lower.contains(query) {
                return Some(600.0);
            }
        }
        None
    }

    fn fuzzy_match_score(query: &str, label: &str) -> f32 {
        let label_lower = label.to_lowercase();
        if Self::is_fuzzy_match(query, &label_lower) {
            500.0 + (query.len() as f32 / label_lower.len() as f32) * 100.0
        } else {
            0.0
        }
    }

    fn is_fuzzy_match(query: &str, target: &str) -> bool {
        let mut query_chars = query.chars();
        let mut current = query_chars.next();
        for ch in target.chars() {
            if let Some(q) = current {
                if ch == q { current = query_chars.next(); }
            }
        }
        current.is_none()
    }

    pub fn filter(&mut self, query: &str) {
        // BUG-07 FIX: Reset selected to 0 when filter changes
        self.selected = 0;

        // P1 FIX: When query is empty, show all commands sorted by usage frequency
        if query.is_empty() {
            let mut indices: Vec<usize> = (0..self.all_commands.len()).collect();
            indices.sort_by(|&a, &b| {
                let freq_a = self.usage_stats.get(&self.all_commands[a].id).map(|u| u.use_count).unwrap_or(0);
                let freq_b = self.usage_stats.get(&self.all_commands[b].id).map(|u| u.use_count).unwrap_or(0);
                freq_b.cmp(&freq_a)
            });
            self.filtered_commands = indices;
            return;
        }

        let query_lower = query.to_lowercase();
        let mut scored: Vec<(usize, f32)> = self.all_commands.iter().enumerate()
            .map(|(idx, cmd)| (idx, Self::fuzzy_score(&query_lower, cmd)))
            .filter(|(_, score)| *score > 0.0)
            .collect();
        scored.sort_by(|a, b| {
            let score_cmp = b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal);
            if score_cmp != std::cmp::Ordering::Equal { return score_cmp; }
            let freq_a = self.usage_stats.get(&self.all_commands[a.0].id).map(|u| u.use_count).unwrap_or(0);
            let freq_b = self.usage_stats.get(&self.all_commands[b.0].id).map(|u| u.use_count).unwrap_or(0);
            freq_b.cmp(&freq_a)
        });
        self.filtered_commands = scored.into_iter().map(|(idx, _)| idx).collect();
    }

    pub fn confirm(&mut self, selected_idx: usize) -> Option<PaletteCommand> {
        if self.is_argument_mode { return self.confirm_with_argument(); }
        if self.filtered_commands.is_empty() || selected_idx >= self.filtered_commands.len() { return None; }
        let idx = self.filtered_commands[selected_idx];
        let cmd_def = self.all_commands[idx].clone();
        if cmd_def.requires_args {
            self.is_argument_mode = true;
            self.argument_input.clear();
            self.pending_command = Some(cmd_def.id.clone());
            return None;
        }
        self.track_usage(&cmd_def.id);
        Some(cmd_def.to_palette_command(""))
    }

    pub fn confirm_with_argument(&mut self) -> Option<PaletteCommand> {
        let cmd_id = self.pending_command.take()?;
        let cmd_def = self.requires_args.get(&cmd_id)?.clone();
        let arg = self.argument_input.clone();
        self.track_usage(&cmd_def.id);
        self.is_argument_mode = false;
        self.argument_input.clear();
        self.pending_command = None;
        Some(cmd_def.to_palette_command(&arg))
    }

    fn track_usage(&mut self, command_id: &str) {
        self.usage_stats.entry(command_id.to_string())
            .and_modify(|u| { u.use_count += 1; u.last_used = Some(Instant::now()); })
            .or_insert(CommandUsage { use_count: 1, last_used: Some(Instant::now()) });
    }

    pub fn insert_char(&mut self, ch: char) { if self.is_argument_mode { self.argument_input.push(ch); } }
    pub fn backspace(&mut self) { if self.is_argument_mode { self.argument_input.pop(); } }
    pub fn clear_input(&mut self) { if self.is_argument_mode { self.argument_input.clear(); } }

    // BUG-08 FIX: Cancel argument mode and return to command selection
    pub fn cancel_argument_mode(&mut self) {
        self.is_argument_mode = false;
        self.argument_input.clear();
        self.pending_command = None;
    }

    pub fn selected_command(&self, _selected_idx: usize) -> Option<&PaletteCommandDef> {
        if self.is_argument_mode {
            self.pending_command.as_ref().and_then(|id| self.requires_args.get(id)).map(|cmd| cmd as &PaletteCommandDef)
        } else if !self.filtered_commands.is_empty() {
            Some(&self.all_commands[self.filtered_commands[0]])
        } else { None }
    }

    pub fn is_argument_mode_active(&self) -> bool { self.is_argument_mode }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) { render::render(self, area, buf, theme); }
}
