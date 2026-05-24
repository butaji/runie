mod render;

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
    LoadSession,
    SaveSession,
    ClearChat,
    SwitchModel,
    ReadFile { path: String },
    EditFile { path: String },
    WriteFile { path: String },
    DeleteFile { path: String },
    CompactContext,
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
    fn to_palette_command(&self, arg: &str) -> PaletteCommand {
        match self.id.as_str() {
            "new_session" => PaletteCommand::NewSession,
            "load_session" => PaletteCommand::LoadSession,
            "save_session" => PaletteCommand::SaveSession,
            "clear_chat" => PaletteCommand::ClearChat,
            "switch_model" => PaletteCommand::SwitchModel,
            "read_file" => PaletteCommand::ReadFile { path: arg.to_string() },
            "edit_file" => PaletteCommand::EditFile { path: arg.to_string() },
            "write_file" => PaletteCommand::WriteFile { path: arg.to_string() },
            "delete_file" => PaletteCommand::DeleteFile { path: arg.to_string() },
            "compact_context" => PaletteCommand::CompactContext,
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
    pub argument_input: String,
    pub is_argument_mode: bool,
    pending_command: Option<String>,
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
            PaletteCommandDef { id: "load_session".into(), label: "Load Session...".into(), description: "Open an existing session".into(), category: "session".into(), aliases: vec!["load".into(), "open".into(), "l".into(), "o".into()], keybinding: Some("Ctrl+O".into()), requires_args: true, arg_hint: "session name".into() },
            PaletteCommandDef { id: "save_session".into(), label: "Save Session...".into(), description: "Save current session".into(), category: "session".into(), aliases: vec!["save".into(), "s".into()], keybinding: Some("Ctrl+S".into()), requires_args: true, arg_hint: "session name".into() },
            PaletteCommandDef { id: "clear_chat".into(), label: "Clear Chat".into(), description: "Clear all messages".into(), category: "chat".into(), aliases: vec!["c".into(), "clear".into()], keybinding: Some("Ctrl+L".into()), requires_args: false, arg_hint: "".into() },
            PaletteCommandDef { id: "switch_model".into(), label: "Switch Model...".into(), description: "Change the AI model".into(), category: "config".into(), aliases: vec!["model".into(), "m".into()], keybinding: None, requires_args: true, arg_hint: "model name".into() },
            PaletteCommandDef { id: "read_file".into(), label: "Read File...".into(), description: "Read contents of a file".into(), category: "file".into(), aliases: vec!["read".into(), "r".into(), "cat".into()], keybinding: Some("Ctrl+R".into()), requires_args: true, arg_hint: "filename".into() },
            PaletteCommandDef { id: "edit_file".into(), label: "Edit File...".into(), description: "Edit a file with AI assistance".into(), category: "file".into(), aliases: vec!["edit".into(), "e".into()], keybinding: None, requires_args: true, arg_hint: "filename".into() },
            PaletteCommandDef { id: "write_file".into(), label: "Write File...".into(), description: "Create or overwrite a file".into(), category: "file".into(), aliases: vec!["write".into(), "w".into(), "create".into()], keybinding: None, requires_args: true, arg_hint: "filename".into() },
            PaletteCommandDef { id: "delete_file".into(), label: "Delete File...".into(), description: "Delete a file".into(), category: "file".into(), aliases: vec!["delete".into(), "rm".into(), "del".into()], keybinding: None, requires_args: true, arg_hint: "filename".into() },
            PaletteCommandDef { id: "compact_context".into(), label: "Compact Context".into(), description: "Reduce context window usage".into(), category: "chat".into(), aliases: vec!["compact".into(), "compress".into()], keybinding: None, requires_args: false, arg_hint: "".into() },
            PaletteCommandDef { id: "quit".into(), label: "Quit".into(), description: "Exit the application".into(), category: "app".into(), aliases: vec!["q".into(), "quit".into(), "exit".into()], keybinding: Some("Ctrl+Q".into()), requires_args: false, arg_hint: "".into() },
        ];

        let mut requires_args_map = HashMap::new();
        for cmd in &all_commands {
            if cmd.requires_args {
                requires_args_map.insert(cmd.id.clone(), cmd.clone());
            }
        }

        Self { requires_args: requires_args_map, all_commands, filtered_commands: Vec::new(), argument_input: String::new(), is_argument_mode: false, pending_command: None, usage_stats: HashMap::new() }
    }

    pub fn all_commands(&self) -> &[PaletteCommandDef] { &self.all_commands }

    fn fuzzy_score(query: &str, command: &PaletteCommandDef) -> f32 {
        let query_lower = query.to_lowercase();
        let label_lower = command.label.to_lowercase();
        let id_lower = command.id.to_lowercase();
        if query_lower.is_empty() { return 1.0; }
        if label_lower == query_lower || id_lower == query_lower { return 1000.0; }
        if label_lower.starts_with(&query_lower) || id_lower.starts_with(&query_lower) { return 900.0; }
        if label_lower.contains(&query_lower) || id_lower.contains(&query_lower) { return 700.0; }
        for alias in &command.aliases {
            if alias.to_lowercase() == query_lower { return 850.0; }
        }
        for alias in &command.aliases {
            if alias.to_lowercase().starts_with(&query_lower) { return 800.0; }
        }
        for alias in &command.aliases {
            if alias.to_lowercase().contains(&query_lower) { return 600.0; }
        }
        if Self::is_fuzzy_match(&query_lower, &label_lower) {
            return 500.0 + (query_lower.len() as f32 / label_lower.len() as f32) * 100.0;
        }
        0.0
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
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(usize, f32)> = self.all_commands.iter().enumerate()
            .map(|(idx, cmd)| (idx, Self::fuzzy_score(&query_lower, cmd)))
            .filter(|(_, score)| *score > 0.0)
            .collect();
        scored.sort_by(|a, b| {
            let score_cmp = b.1.partial_cmp(&a.1).unwrap();
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

    fn confirm_with_argument(&mut self) -> Option<PaletteCommand> {
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
