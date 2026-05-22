mod render;

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

#[derive(Debug, Clone, PartialEq)]
pub enum PaletteCommand {
    ReadFile { path: String },
    EditFile { path: String, prompt: String },
    RunAgent { name: String },
    SwitchModel { model: String },
    LoadSession { id: String },
    SaveSession { name: String },
    Cancel,
}

#[derive(Clone)]
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

        let actions = vec![];
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
                self.filtered_objects = Self::filter_items(&self.objects, &self.query);
                self.clamp_selection(self.filtered_objects.len());
            }
            PaletteStep::Action => {
                self.filtered_actions = Self::filter_items(&self.actions, &self.query);
                self.clamp_selection(self.filtered_actions.len());
            }
            PaletteStep::Arguments => {}
        }
    }

    fn filter_items(items: &[PaletteItem], query: &str) -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| Self::item_matches(query, item))
            .map(|(i, _)| i)
            .collect()
    }

    fn item_matches(query: &str, item: &PaletteItem) -> bool {
        if query.is_empty() {
            true
        } else {
            let q = &query.to_lowercase();
            item.label.to_lowercase().contains(q) || item.id.to_lowercase().contains(q)
        }
    }

    fn clamp_selection(&mut self, len: usize) {
        if self.selected >= len {
            self.selected = len.saturating_sub(1);
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

                if action.id == "list" || action.id == "view" || action.id == "info" {
                    return self.execute_command();
                }
                None
            }
            PaletteStep::Arguments => self.execute_command(),
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
            ("file", "delete") => Some(PaletteCommand::ReadFile { path: arg.to_string() }),
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
        render::render(self, area, buf, theme);
    }
}

#[allow(clippy::unwrap_used)]
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

        palette.selected = count - 1;
        palette.next_item();
        assert_eq!(palette.selected, 0);

        palette.selected = 0;
        palette.prev_item();
        assert_eq!(palette.selected, count - 1);
    }

    #[test]
    fn test_actions_depend_on_object() {
        let mut palette = make_palette();

        palette.selected = 0;
        palette.confirm();
        assert!(palette.actions.iter().any(|a| a.id == "read"));
        assert!(palette.actions.iter().any(|a| a.id == "edit"));

        palette.reset();
        palette.selected = 1;
        palette.confirm();
        assert!(palette.actions.iter().any(|a| a.id == "start"));
        assert!(palette.actions.iter().any(|a| a.id == "stop"));
        assert!(!palette.actions.iter().any(|a| a.id == "read"));
    }

    #[test]
    fn test_execute_read_file_command() {
        let mut palette = make_palette();

        palette.selected = 0;
        palette.confirm();

        palette.selected = 0;
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
        assert_eq!(palette.filtered_objects.len(), palette.objects.len());
    }

    #[test]
    fn test_reset_restores_initial_state() {
        let mut palette = make_palette();

        palette.selected = 0;
        palette.confirm();
        assert_eq!(palette.step, PaletteStep::Action);

        palette.reset();
        assert_eq!(palette.step, PaletteStep::Object);
        assert!(palette.query.is_empty());
        assert_eq!(palette.selected, 0);
        assert!(palette.selected_object.is_none());
        assert!(palette.selected_action.is_none());
    }
}
