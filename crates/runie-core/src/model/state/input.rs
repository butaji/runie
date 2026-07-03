use serde::{Deserialize, Serialize};

/// Tracks usage count and last-used timestamp for a command.
#[derive(Clone, Debug)]
pub struct CommandUsage {
    pub count: u32,
    pub last_used: f64,
}

/// Input state — text, cursor, history.
/// Fields are public for test setup; production code should use accessors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputState {
    pub input: String,
    pub cursor_pos: usize,
    pub undo_stack: Vec<(String, usize)>,
    pub redo_stack: Vec<(String, usize)>,
    pub history_pos: Option<usize>,
    pub input_flash: u8,
    pub placeholder: String,
    /// Ghost completion suffix shown in gray after the cursor.
    pub ghost_completion: Option<String>,
    /// Tab-completion state stored as raw fields (avoid circular dep).
    pub tab_complete_prefix: Option<String>,
    pub tab_complete_matches: Vec<String>,
    pub tab_complete_index: usize,
    /// Top visible line index for multi-line input scrolling.
    pub input_scroll: usize,
    /// Command input history (persistent across sessions).
    pub input_history: Vec<String>,
    pub current_prompt: String,
    /// Backup of input state before opening file picker:
    /// (original input, insert position, cursor position, needs brackets for @ references).
    pub file_picker_backup: Option<(String, usize, usize, bool)>,
    /// The `:start-end` range suffix to append when inserting a file reference.
    /// Set when opening the picker from `@path:10-50`.
    pub file_picker_range_suffix: Option<String>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_pos: None,
            input_flash: 0,
            placeholder: "Type a message to start...".into(),
            ghost_completion: None,
            tab_complete_prefix: None,
            tab_complete_matches: Vec::new(),
            tab_complete_index: 0,
            input_scroll: 0,
            input_history: Vec::new(),
            current_prompt: String::new(),
            file_picker_backup: None,
            file_picker_range_suffix: None,
        }
    }
}

impl InputState {
    /// Immutable access to the input text.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Mutable access to the input text.
    pub fn input_mut(&mut self) -> &mut String {
        &mut self.input
    }

    /// Immutable access to cursor position.
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Mutable access to cursor position.
    pub fn cursor_pos_mut(&mut self) -> &mut usize {
        &mut self.cursor_pos
    }

    /// Push current (input, cursor) onto the undo stack.
    pub fn push_undo(&mut self) {
        self.undo_stack.push((self.input.clone(), self.cursor_pos));
    }
}
