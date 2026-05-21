use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Modifier},
};
use crate::theme::ThemeWrapper;

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteStep {
    Object,
    Action,
    Arguments,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub enum PaletteCommand {
    ReadFile { path: String },
    EditFile { path: String, prompt: String },
    RunAgent { name: String },
    SwitchModel { model: String },
    LoadSession { id: String },
    SaveSession { name: String },
    Cancel,
}

pub struct CommandPalette {
    pub step: PaletteStep,
    pub query: String,
    pub selected: usize,
    pub objects: Vec<PaletteItem>,
    pub actions: Vec<PaletteItem>,
    pub filtered_objects: Vec<usize>,
    pub filtered_actions: Vec<usize>,
    pub selected_object: Option<PaletteItem>,
    pub selected_action: Option<PaletteItem>,
    pub argument_input: String,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandPalette {
    pub fn new() -> Self {
        let objects = vec![
            PaletteItem { id: "file".into(), label: "File...".into(), icon: "▸".into(), category: "workspace".into() },
            PaletteItem { id: "agent".into(), label: "Agent...".into(), icon: "▸".into(), category: "agent".into() },
            PaletteItem { id: "model".into(), label: "Model...".into(), icon: "▸".into(), category: "config".into() },
            PaletteItem { id: "session".into(), label: "Session...".into(), icon: "▸".into(), category: "session".into() },
        ];

        let actions = vec![]; // Actions depend on selected object
        let filtered_objects: Vec<usize> = (0..objects.len()).collect();
        let filtered_actions = vec![];

        Self {
            step: PaletteStep::Object,
            query: String::new(),
            selected: 0,
            objects,
            actions,
            filtered_objects,
            filtered_actions,
            selected_object: None,
            selected_action: None,
            argument_input: String::new(),
        }
    }

    fn get_actions_for_object(object_id: &str) -> Vec<PaletteItem> {
        match object_id {
            "file" => vec![
                PaletteItem { id: "read".into(), label: "Read".into(), icon: "▸".into(), category: "file".into() },
                PaletteItem { id: "edit".into(), label: "Edit".into(), icon: "▸".into(), category: "file".into() },
                PaletteItem { id: "write".into(), label: "Write".into(), icon: "▸".into(), category: "file".into() },
                PaletteItem { id: "delete".into(), label: "Delete".into(), icon: "▸".into(), category: "file".into() },
            ],
            "agent" => vec![
                PaletteItem { id: "start".into(), label: "Start".into(), icon: "▸".into(), category: "agent".into() },
                PaletteItem { id: "stop".into(), label: "Stop".into(), icon: "▸".into(), category: "agent".into() },
                PaletteItem { id: "view".into(), label: "View".into(), icon: "▸".into(), category: "agent".into() },
                PaletteItem { id: "configure".into(), label: "Configure".into(), icon: "▸".into(), category: "agent".into() },
            ],
            "model" => vec![
                PaletteItem { id: "switch".into(), label: "Switch".into(), icon: "▸".into(), category: "model".into() },
                PaletteItem { id: "info".into(), label: "Info".into(), icon: "▸".into(), category: "model".into() },
                PaletteItem { id: "list".into(), label: "List".into(), icon: "▸".into(), category: "model".into() },
            ],
            "session" => vec![
                PaletteItem { id: "list".into(), label: "List".into(), icon: "▸".into(), category: "session".into() },
                PaletteItem { id: "load".into(), label: "Load".into(), icon: "▸".into(), category: "session".into() },
                PaletteItem { id: "save".into(), label: "Save".into(), icon: "▸".into(), category: "session".into() },
                PaletteItem { id: "delete".into(), label: "Delete".into(), icon: "▸".into(), category: "session".into() },
            ],
            _ => vec![],
        }
    }

    pub fn filter(&mut self) {
        match self.step {
            PaletteStep::Object => {
                self.filtered_objects = self
                    .objects
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| {
                        if self.query.is_empty() {
                            true
                        } else {
                            item.label.to_lowercase().contains(&self.query.to_lowercase())
                                || item.id.to_lowercase().contains(&self.query.to_lowercase())
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();
                if self.selected >= self.filtered_objects.len() {
                    self.selected = self.filtered_objects.len().saturating_sub(1);
                }
            }
            PaletteStep::Action => {
                self.filtered_actions = self
                    .actions
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| {
                        if self.query.is_empty() {
                            true
                        } else {
                            item.label.to_lowercase().contains(&self.query.to_lowercase())
                                || item.id.to_lowercase().contains(&self.query.to_lowercase())
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();
                if self.selected >= self.filtered_actions.len() {
                    self.selected = self.filtered_actions.len().saturating_sub(1);
                }
            }
            PaletteStep::Arguments => {
                // No filtering needed for arguments
            }
        }
    }

    pub fn next_item(&mut self) {
        let len = match self.step {
            PaletteStep::Object => self.filtered_objects.len(),
            PaletteStep::Action => self.filtered_actions.len(),
            PaletteStep::Arguments => 0,
        };
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    pub fn prev_item(&mut self) {
        let len = match self.step {
            PaletteStep::Object => self.filtered_objects.len(),
            PaletteStep::Action => self.filtered_actions.len(),
            PaletteStep::Arguments => 0,
        };
        if len == 0 {
            return;
        }
        self.selected = (self.selected + len - 1) % len;
    }

    pub fn confirm(&mut self) -> Option<PaletteCommand> {
        match self.step {
            PaletteStep::Object => {
                if self.filtered_objects.is_empty() {
                    return None;
                }
                let idx = self.filtered_objects[self.selected];
                let obj = self.objects[idx].clone();
                self.selected_object = Some(obj.clone());
                self.actions = Self::get_actions_for_object(&obj.id);
                self.filtered_actions = (0..self.actions.len()).collect();
                self.selected = 0;
                self.query.clear();
                self.step = PaletteStep::Action;
                None
            }
            PaletteStep::Action => {
                if self.filtered_actions.is_empty() {
                    return None;
                }
                let idx = self.filtered_actions[self.selected];
                let action = self.actions[idx].clone();
                self.selected_action = Some(action.clone());
                self.selected = 0;
                self.query.clear();
                self.step = PaletteStep::Arguments;

                // Check if action requires no arguments
                if action.id == "list" || action.id == "view" || action.id == "info" {
                    return self.execute_command();
                }
                None
            }
            PaletteStep::Arguments => {
                self.execute_command()
            }
        }
    }

    fn execute_command(&self) -> Option<PaletteCommand> {
        let obj = self.selected_object.as_ref()?;
        let action = self.selected_action.as_ref()?;

        let arg = self.argument_input.trim();

        match (obj.id.as_str(), action.id.as_str()) {
            ("file", "read") => Some(PaletteCommand::ReadFile { path: arg.to_string() }),
            ("file", "edit") => Some(PaletteCommand::EditFile { path: arg.to_string(), prompt: String::new() }),
            ("file", "write") => Some(PaletteCommand::EditFile { path: arg.to_string(), prompt: String::new() }),
            ("file", "delete") => Some(PaletteCommand::ReadFile { path: arg.to_string() }), // reuse for delete confirmation
            ("agent", "start") => Some(PaletteCommand::RunAgent { name: arg.to_string() }),
            ("agent", "stop") => Some(PaletteCommand::RunAgent { name: arg.to_string() }),
            ("agent", "view") => Some(PaletteCommand::RunAgent { name: arg.to_string() }),
            ("agent", "configure") => Some(PaletteCommand::RunAgent { name: arg.to_string() }),
            ("model", "switch") => Some(PaletteCommand::SwitchModel { model: arg.to_string() }),
            ("model", "info") => Some(PaletteCommand::SwitchModel { model: arg.to_string() }),
            ("model", "list") => Some(PaletteCommand::SwitchModel { model: String::new() }),
            ("session", "list") => Some(PaletteCommand::LoadSession { id: String::new() }),
            ("session", "load") => Some(PaletteCommand::LoadSession { id: arg.to_string() }),
            ("session", "save") => Some(PaletteCommand::SaveSession { name: arg.to_string() }),
            ("session", "delete") => Some(PaletteCommand::LoadSession { id: arg.to_string() }),
            _ => Some(PaletteCommand::Cancel),
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.push(ch);
        self.filter();
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.filter();
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
        self.filter();
    }

    pub fn reset(&mut self) {
        self.step = PaletteStep::Object;
        self.query.clear();
        self.selected = 0;
        self.filtered_objects = (0..self.objects.len()).collect();
        self.filtered_actions.clear();
        self.selected_object = None;
        self.selected_action = None;
        self.argument_input.clear();
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        let bg_panel: ratatui::style::Color = theme.color("bg.panel").into();
        let accent_primary: ratatui::style::Color = theme.color("accent.primary").into();
        let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();
        let text_primary: ratatui::style::Color = theme.color("text.primary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();
        let text_secondary: ratatui::style::Color = theme.color("text.secondary").into();
        let border_unfocused: ratatui::style::Color = theme.color("border.unfocused").into();

        // Clear background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.get_mut(x, y).set_style(Style::default().bg(bg_panel));
            }
        }

        // Draw full border around the area
        // Top border
        for x in area.x..area.x + area.width {
            buf.get_mut(x, area.y).set_char('─');
            buf.get_mut(x, area.y).set_style(Style::default().fg(border_unfocused));
        }
        // Bottom border
        for x in area.x..area.x + area.width {
            buf.get_mut(x, area.y + area.height - 1).set_char('─');
            buf.get_mut(x, area.y + area.height - 1).set_style(Style::default().fg(border_unfocused));
        }
        // Left border
        for y in area.y..area.y + area.height {
            buf.get_mut(area.x, y).set_char('│');
            buf.get_mut(area.x, y).set_style(Style::default().fg(border_unfocused));
        }
        // Right border
        for y in area.y..area.y + area.height {
            buf.get_mut(area.x + area.width - 1, y).set_char('│');
            buf.get_mut(area.x + area.width - 1, y).set_style(Style::default().fg(border_unfocused));
        }
        // Corners
        buf.get_mut(area.x, area.y).set_char('╭');
        buf.get_mut(area.x, area.y).set_style(Style::default().fg(border_unfocused));
        buf.get_mut(area.x + area.width - 1, area.y).set_char('╮');
        buf.get_mut(area.x + area.width - 1, area.y).set_style(Style::default().fg(border_unfocused));
        buf.get_mut(area.x, area.y + area.height - 1).set_char('╰');
        buf.get_mut(area.x, area.y + area.height - 1).set_style(Style::default().fg(border_unfocused));
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('╯');
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_style(Style::default().fg(border_unfocused));

        // Title in top border (left-aligned after corner)
        let title = " Command Palette ";
        let title_style = Style::default().fg(accent_primary).add_modifier(Modifier::BOLD);
        for (i, ch) in title.chars().enumerate() {
            let x = area.x + 1 + i as u16;
            if x < area.x + area.width - 1 {
                buf.get_mut(x, area.y).set_char(ch);
                buf.get_mut(x, area.y).set_style(title_style);
            }
        }

        // Close hint in top border (right-aligned)
        let close_hint = " [Esc] ";
        let close_start = area.x + area.width - 1 - close_hint.len() as u16;
        for (i, ch) in close_hint.chars().enumerate() {
            let x = close_start + i as u16;
            if x > area.x && x < area.x + area.width - 1 {
                buf.get_mut(x, area.y).set_char(ch);
                buf.get_mut(x, area.y).set_style(Style::default().fg(text_muted));
            }
        }

        // Calculate inner area (inside borders)
        let inner_x = area.x + 1;
        let inner_y = area.y + 1;
        let inner_w = area.width.saturating_sub(2);
        let inner_h = area.height.saturating_sub(2);

        // 3-pane layout inside inner area
        let pane_w = inner_w / 3;
        let pane_h = inner_h.saturating_sub(4); // Leave 4 rows for query + instructions
        let pane_y = inner_y + 1; // One row for headers

        let object_x = inner_x;
        let action_x = inner_x + pane_w;
        let arg_x = inner_x + pane_w * 2;

        // Draw pane headers
        let obj_header = " OBJECT ";
        let act_header = " ACTION ";
        let arg_header = " ARGS ";

        let obj_style = if self.step == PaletteStep::Object {
            Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_secondary)
        };
        let act_style = if self.step == PaletteStep::Action {
            Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_secondary)
        };
        let arg_style = if self.step == PaletteStep::Arguments {
            Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_secondary)
        };

        buf.set_string(object_x, inner_y, obj_header, obj_style);
        buf.set_string(action_x, inner_y, act_header, act_style);
        buf.set_string(arg_x, inner_y, arg_header, arg_style);

        // Draw vertical separators between panes (inside the modal, not at border)
        for y in pane_y..pane_y + pane_h {
            // Separator after object pane
            let sep1_x = action_x - 1;
            if sep1_x > area.x && sep1_x < area.x + area.width - 1 {
                buf.get_mut(sep1_x, y).set_char('│');
                buf.get_mut(sep1_x, y).set_style(Style::default().fg(border_unfocused));
            }
            // Separator after action pane
            let sep2_x = arg_x - 1;
            if sep2_x > area.x && sep2_x < area.x + area.width - 1 {
                buf.get_mut(sep2_x, y).set_char('│');
                buf.get_mut(sep2_x, y).set_style(Style::default().fg(border_unfocused));
            }
        }

        // Draw pane content
        let object_area = Rect::new(object_x, pane_y, pane_w.saturating_sub(1), pane_h);
        let action_area = Rect::new(action_x, pane_y, pane_w.saturating_sub(1), pane_h);
        let arg_area = Rect::new(arg_x, pane_y, pane_w.saturating_sub(1), pane_h);

        match self.step {
            PaletteStep::Object => {
                self.render_pane_items(object_area, buf, theme, &self.filtered_objects, &self.objects, PaletteStep::Object);
            }
            PaletteStep::Action => {
                self.render_pane_items(object_area, buf, theme, &self.filtered_objects, &self.objects, PaletteStep::Object);
                self.render_pane_items(action_area, buf, theme, &self.filtered_actions, &self.actions, PaletteStep::Action);
            }
            PaletteStep::Arguments => {
                self.render_pane_items(object_area, buf, theme, &self.filtered_objects, &self.objects, PaletteStep::Object);
                self.render_pane_items(action_area, buf, theme, &self.filtered_actions, &self.actions, PaletteStep::Action);
                self.render_arguments_pane(arg_area, buf, theme);
            }
        }

        // Query input at bottom (2 rows from bottom)
        let input_y = area.y + area.height - 3;
        let input_prompt = "▸ ";
        buf.set_string(inner_x, input_y, input_prompt, Style::default().fg(accent_primary));

        let query_text = if self.query.is_empty() {
            "type to filter..."
        } else {
            &self.query
        };
        let query_style = if self.query.is_empty() {
            Style::default().fg(text_muted)
        } else {
            Style::default().fg(text_primary)
        };
        buf.set_string(inner_x + 2, input_y, query_text, query_style);

        // Argument input if on arguments step
        if self.step == PaletteStep::Arguments {
            let arg_y = area.y + area.height - 4;
            let arg_label = "value: ";
            buf.set_string(inner_x, arg_y, arg_label, Style::default().fg(text_secondary));
            let arg_text = if self.argument_input.is_empty() {
                "enter value..."
            } else {
                &self.argument_input
            };
            let arg_style = if self.argument_input.is_empty() {
                Style::default().fg(text_muted)
            } else {
                Style::default().fg(text_primary)
            };
            buf.set_string(inner_x + 7, arg_y, arg_text, arg_style);
        }

        // Instructions at bottom
        let instr_y = area.y + area.height - 2;
        let instructions = "[↑↓] navigate  [Enter] select  [Esc] close";
        buf.set_string(inner_x, instr_y, instructions, Style::default().fg(text_muted));
    }

    fn render_pane_items(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper, indices: &[usize], items: &[PaletteItem], pane_step: PaletteStep) {
        let text_primary: ratatui::style::Color = theme.color("text.primary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();
        let accent_secondary: ratatui::style::Color = theme.color("accent.secondary").into();

        let visible_items = area.height as usize;
        let start_idx = if self.selected >= visible_items {
            self.selected - visible_items + 1
        } else {
            0
        };

        for i in 0..visible_items {
            let item_idx = start_idx + i;
            if item_idx >= indices.len() {
                break;
            }
            let global_idx = indices[item_idx];
            let item = &items[global_idx];
            let is_selected = item_idx == self.selected;
            let is_active_pane = match pane_step {
                PaletteStep::Object => self.step == PaletteStep::Object,
                PaletteStep::Action => self.step == PaletteStep::Action,
                PaletteStep::Arguments => false,
            };

            let y = area.y + i as u16;
            let icon = if is_selected && is_active_pane {
                "▸".to_string()
            } else {
                " ".to_string()
            };

            let label_style = if is_selected && is_active_pane {
                Style::default().fg(accent_secondary).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(text_primary)
            } else {
                Style::default().fg(text_muted)
            };

            buf.set_string(area.x + 1, y, &icon, label_style);
            buf.set_string(area.x + 3, y, &item.label, label_style);
        }
    }

    fn render_arguments_pane(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        let text_primary: ratatui::style::Color = theme.color("text.primary").into();
        let text_muted: ratatui::style::Color = theme.color("text.muted").into();

        if let (Some(obj), Some(action)) = (&self.selected_object, &self.selected_action) {
            // Show the action context
            let context = format!("{} → {}", obj.label, action.label);
            let context_style = Style::default().fg(text_primary);
            buf.set_string(area.x + 1, area.y, &context, context_style);

            // Show argument hint based on action
            let hint = match action.id.as_str() {
                "read" | "edit" | "write" | "delete" | "load" => "filename or path",
                "switch" => "model name",
                "save" => "session name",
                "start" | "stop" | "view" | "configure" => "agent name",
                _ => "value",
            };
            let hint_style = Style::default().fg(text_muted);
            buf.set_string(area.x + 1, area.y + 2, hint, hint_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_palette() -> CommandPalette {
        CommandPalette::new()
    }

    #[test]
    fn test_filter_matches_prefix() {
        let mut palette = make_palette();
        palette.query = "fil".to_string();
        palette.filter();
        assert!(!palette.filtered_objects.is_empty());
        assert!(palette.filtered_objects.len() < palette.objects.len());
    }

    #[test]
    fn test_filter_no_match() {
        let mut palette = make_palette();
        palette.query = "xyzzy".to_string();
        palette.filter();
        assert!(palette.filtered_objects.is_empty());
    }

    #[test]
    fn test_confirm_advances_step() {
        let mut palette = make_palette();
        assert_eq!(palette.step, PaletteStep::Object);

        // Select "File" object
        palette.selected = 0;
        let result = palette.confirm();
        assert!(result.is_none());
        assert_eq!(palette.step, PaletteStep::Action);
        assert!(palette.selected_object.is_some());
        assert_eq!(palette.selected_object.unwrap().id, "file");
        assert!(!palette.actions.is_empty());
    }

    #[test]
    fn test_next_item_wraps() {
        let mut palette = make_palette();
        palette.filter();
        let count = palette.filtered_objects.len();
        assert!(count > 0);

        // Go to last item
        palette.selected = count - 1;
        palette.next_item();
        assert_eq!(palette.selected, 0); // Wrapped to 0

        // Go to first item then prev
        palette.selected = 0;
        palette.prev_item();
        assert_eq!(palette.selected, count - 1); // Wrapped to last
    }

    #[test]
    fn test_actions_depend_on_object() {
        let mut palette = make_palette();

        // Select "file"
        palette.selected = 0;
        palette.confirm();
        assert!(palette.actions.iter().any(|a| a.id == "read"));
        assert!(palette.actions.iter().any(|a| a.id == "edit"));

        // Reset and select "agent"
        palette.reset();
        palette.selected = 1; // agent
        palette.confirm();
        assert!(palette.actions.iter().any(|a| a.id == "start"));
        assert!(palette.actions.iter().any(|a| a.id == "stop"));
        assert!(!palette.actions.iter().any(|a| a.id == "read")); // no read for agent
    }

    #[test]
    fn test_execute_read_file_command() {
        let mut palette = make_palette();

        // Object -> Action -> Arguments flow for Read File
        palette.selected = 0; // File
        palette.confirm();

        palette.selected = 0; // Read action
        palette.confirm();

        palette.argument_input = "test.txt".to_string();
        let result = palette.confirm();

        match result {
            Some(PaletteCommand::ReadFile { path }) => {
                assert_eq!(path, "test.txt");
            }
            _ => panic!("Expected ReadFile command"),
        }
    }

    #[test]
    fn test_insert_and_backspace() {
        let mut palette = make_palette();
        palette.insert_char('f');
        assert_eq!(palette.query, "f");
        palette.insert_char('i');
        assert_eq!(palette.query, "fi");
        palette.backspace();
        assert_eq!(palette.query, "f");
        palette.backspace();
        assert_eq!(palette.query, "");
    }

    #[test]
    fn test_clear_query() {
        let mut palette = make_palette();
        palette.query = "test".to_string();
        palette.clear_query();
        assert_eq!(palette.query, "");
        palette.filter();
        // All objects should be visible after clearing
        assert_eq!(palette.filtered_objects.len(), palette.objects.len());
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut palette = make_palette();

        // Advance to Action step
        palette.selected = 0;
        palette.confirm();
        assert_eq!(palette.step, PaletteStep::Action);

        // Reset
        palette.reset();
        assert_eq!(palette.step, PaletteStep::Object);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected, 0);
        assert!(palette.selected_object.is_none());
        assert!(palette.selected_action.is_none());
    }
}