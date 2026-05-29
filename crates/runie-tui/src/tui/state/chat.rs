use crate::components::MessageItem;
use super::ScrollState;

/// ChatState contains user input and message-related fields.
/// All fields are cloned for RenderState so they must implement Clone.
#[derive(Clone)]
pub struct ChatState {
    pub messages: Vec<MessageItem>,
    pub textarea: ratatui_textarea::TextArea<'static>,
    pub input_right_info: String,
    pub scroll: ScrollState,
    pub input_history: Vec<String>,
    pub input_history_index: Option<usize>,
    pub input_draft: String,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            textarea: ratatui_textarea::TextArea::default(),
            input_right_info: String::new(),
            scroll: ScrollState::default(),
            input_history: Vec::new(),
            input_history_index: None,
            input_draft: String::new(),
        }
    }
}
