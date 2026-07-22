use crate::model::AppState;

pub(crate) fn extract_path_prefix(input: &str, cursor: usize) -> String {
    let before = &input[..cursor.min(input.len())];
    let start = before.rfind(' ').map(|i| i + 1).unwrap_or(0);
    before[start..].to_string()
}

impl AppState {
    pub(crate) fn toggle_path_completion(&mut self) {
        let partial = {
            let input = self.input();
            extract_path_prefix(&input.input, input.cursor_pos)
        };
        let cwd = std::env::current_dir().unwrap_or_default();
        let suggestions = crate::path_complete::complete_path(&partial, &cwd);
        if suggestions.is_empty() {
            self.input_mut().input_flash = 3;
            return;
        }
        self.completion_mut().path_suggestions = Some(suggestions);
        self.completion_mut().path_selected = Some(0);
        self.view_mut().dirty = true;
    }

    pub(crate) fn path_completion_up(&mut self) {
        let completion = self.completion_mut();
        if let Some(ref items) = completion.path_suggestions {
            let sel = completion.path_selected.unwrap_or(0);
            completion.path_selected = Some(if sel == 0 { items.len() - 1 } else { sel - 1 });
            self.view_mut().dirty = true;
        }
    }

    pub(crate) fn path_completion_down(&mut self) {
        let completion = self.completion_mut();
        if let Some(ref items) = completion.path_suggestions {
            let sel = completion.path_selected.unwrap_or(0);
            completion.path_selected = Some((sel + 1) % items.len());
            self.view_mut().dirty = true;
        }
    }

    pub(crate) fn path_completion_select(&mut self) {
        let (path, is_dir) = {
            let completion = self.completion();
            match (&completion.path_suggestions, completion.path_selected) {
                (Some(items), Some(sel)) if sel < items.len() => (items[sel].path.clone(), items[sel].is_dir),
                _ => return,
            }
        };
        let prefix = {
            let input = self.input();
            extract_path_prefix(&input.input, input.cursor_pos)
        };
        let replacement = if is_dir { format!("{}/", path) } else { path };
        self.replace_path_prefix(&prefix, &replacement);
        self.completion_mut().path_suggestions = None;
        self.completion_mut().path_selected = None;
        self.view_mut().dirty = true;
    }

    pub(super) fn path_completion_close(&mut self) {
        self.completion_mut().path_suggestions = None;
        self.completion_mut().path_selected = None;
        self.view_mut().dirty = true;
    }

    fn replace_path_prefix(&mut self, prefix: &str, replacement: &str) {
        let (cursor_pos, input_len, input_str) = {
            let input = self.input();
            (input.cursor_pos, input.input.len(), input.input.clone())
        };
        let before = &input_str[..cursor_pos];
        if let Some(pos) = before.rfind(prefix) {
            let mut new_input = String::with_capacity(input_len - prefix.len() + replacement.len());
            new_input.push_str(&input_str[..pos]);
            new_input.push_str(replacement);
            new_input.push_str(&input_str[cursor_pos..]);
            let input_mut = self.input_mut();
            input_mut.input = new_input;
            input_mut.cursor_pos = pos + replacement.len();
        }
    }
}
