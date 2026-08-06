//! Renderer-independent prompt state vocabulary.

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
