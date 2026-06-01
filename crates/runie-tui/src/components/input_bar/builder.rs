use crate::tui::view_models::InputBarViewModel;
use ratatui_textarea::TextArea;

pub(crate) struct InputBuilder {
    textarea: TextArea<'static>,
    prompt: String,
    right_info: String,
    placeholder: String,
    mode_indicator: String,
    attached_files: Vec<String>,
    char_count: Option<usize>,
    context_window: Option<usize>,
}

impl InputBuilder {
    pub(crate) fn new() -> Self {
        Self {
            textarea: TextArea::default(),
            prompt: format!("{ch} ", ch = crate::glyphs::CHEVRON),
            right_info: String::new(),
            placeholder: String::new(),
            mode_indicator: "runie".to_string(),
            attached_files: Vec::new(),
            char_count: None,
            context_window: None,
        }
    }

    pub(crate) fn placeholder(mut self, text: &str) -> Self {
        self.placeholder = text.to_string();
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

    pub(crate) fn mode_indicator(mut self, mode: &str) -> Self {
        self.mode_indicator = mode.to_string();
        self
    }

    pub(crate) fn attached_files(mut self, files: Vec<String>) -> Self {
        self.attached_files = files;
        self
    }

    pub(crate) fn char_count(mut self, count: Option<usize>) -> Self {
        self.char_count = count;
        self
    }

    pub(crate) fn context_window(mut self, window: Option<usize>) -> Self {
        self.context_window = window;
        self
    }

    pub(crate) fn build(self) -> InputBarViewModel {
        InputBarViewModel {
            textarea: self.textarea,
            prompt: self.prompt,
            right_info: self.right_info,
            placeholder: self.placeholder,
            mode_indicator: self.mode_indicator,
            attached_files: self.attached_files,
            char_count: self.char_count,
            context_window: self.context_window,
        }
    }
}

impl Default for InputBuilder {
    fn default() -> Self {
        Self::new()
    }
}
