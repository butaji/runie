use crate::tui::view_models::InputBarViewModel;
use ratatui_textarea::TextArea;

pub(crate) struct InputBuilder {
    textarea: TextArea<'static>,
    prompt: String,
    right_info: String,
}

impl InputBuilder {
    pub(crate) fn new() -> Self {
        Self {
            textarea: TextArea::default(),
            prompt: format!("{ch} ", ch = crate::glyphs::CHEVRON),
            right_info: String::new(),
        }
    }

    pub(crate) fn placeholder(mut self, text: &str) -> Self {
        self.textarea = TextArea::new(vec![text.to_string()]);
        self
    }

    pub(crate) fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    pub(crate) fn info(mut self, info: &str) -> Self {
        self.right_info = info.to_string();
        self
    }

    pub(crate) fn text(mut self, text: &str) -> Self {
        self.textarea = TextArea::new(vec![text.to_string()]);
        self
    }

    pub(crate) fn build(self) -> InputBarViewModel {
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
