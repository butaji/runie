//! Renderer-independent prompt state vocabulary.

use runie_core::types::ThemeKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    Submitted(String),
    Edited,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Alternate,
    Plan,
    FileSearch,
    FileViewer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSnapshot {
    pub text: String,
    pub focused: bool,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub history_search: bool,
    pub mode: InputMode,
    pub model_caption: String,
    pub show_placeholder: bool,
    pub file_candidates: Vec<String>,
    pub file_candidate_index: usize,
    pub selected_file: Option<String>,
    pub viewer_lines: Vec<String>,
    pub theme: ThemeKind,
}
