use crate::model::{AppState, ChatMessage, Role};

pub(crate) fn extract_path_prefix(input: &str, cursor: usize) -> String {
    let before = &input[..cursor.min(input.len())];
    let start = before.rfind(' ').map(|i| i + 1).unwrap_or(0);
    before[start..].to_string()
}

impl AppState {
    pub(crate) fn toggle_path_completion(&mut self) {
        let partial = extract_path_prefix(&self.input, self.cursor_pos);
        let cwd = std::env::current_dir().unwrap_or_default();
        let suggestions = crate::path_complete::complete_path(&partial, &cwd);
        if suggestions.is_empty() {
            self.input_flash = 3;
            return;
        }
        self.path_suggestions = Some(suggestions);
        self.path_selected = Some(0);
        self.mark_dirty();
    }

    pub(crate) fn path_completion_up(&mut self) {
        if let Some(ref items) = self.path_suggestions {
            let sel = self.path_selected.unwrap_or(0);
            self.path_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
            self.mark_dirty();
        }
    }

    pub(crate) fn path_completion_down(&mut self) {
        if let Some(ref items) = self.path_suggestions {
            let sel = self.path_selected.unwrap_or(0);
            self.path_selected = Some((sel + 1) % items.len());
            self.mark_dirty();
        }
    }

    pub(crate) fn path_completion_select(&mut self) {
        let (path, is_dir) = match (&self.path_suggestions, self.path_selected) {
            (Some(items), Some(sel)) if sel < items.len() => {
                (items[sel].path.clone(), items[sel].is_dir)
            }
            _ => return,
        };
        let prefix = extract_path_prefix(&self.input, self.cursor_pos);
        let replacement = if is_dir { format!("{}/", path) } else { path };
        self.replace_path_prefix(&prefix, &replacement);
        self.path_suggestions = None;
        self.path_selected = None;
        self.mark_dirty();
    }

    pub(crate) fn path_completion_close(&mut self) {
        self.path_suggestions = None;
        self.path_selected = None;
        self.mark_dirty();
    }

    fn replace_path_prefix(&mut self, prefix: &str, replacement: &str) {
        let before = &self.input[..self.cursor_pos];
        if let Some(pos) = before.rfind(prefix) {
            let mut new_input = String::with_capacity(self.input.len() - prefix.len() + replacement.len());
            new_input.push_str(&self.input[..pos]);
            new_input.push_str(replacement);
            new_input.push_str(&self.input[self.cursor_pos..]);
            self.cursor_pos = pos + replacement.len();
            self.input = new_input;
        }
    }
}
