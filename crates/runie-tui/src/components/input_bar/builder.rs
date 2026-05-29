use crate::tui::view_models::InputBarViewModel;
use ratatui_textarea::TextArea;

pub struct InputBuilder {
    textarea: TextArea<'static>,
    prompt: String,
    right_info: String,
}

impl InputBuilder {
    pub fn new() -> Self {
        Self {
            textarea: TextArea::default(),
            prompt: "\u{276F} ".to_string(),
            right_info: String::new(),
        }
    }

    pub fn placeholder(mut self, text: &str) -> Self {
        self.textarea = TextArea::new(vec![text.to_string()]);
        self
    }

    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    pub fn info(mut self, info: &str) -> Self {
        self.right_info = info.to_string();
        self
    }

    pub fn text(mut self, text: &str) -> Self {
        self.textarea = TextArea::new(vec![text.to_string()]);
        self
    }

    pub fn build(self) -> InputBarViewModel {
        InputBarViewModel {
            textarea: self.textarea,
            prompt: self.prompt,
            right_info: self.right_info,
        }
    }
}

impl Default for InputBuilder {
    fn default() -> Self {
        Self::new()
    }
}
