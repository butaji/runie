use serde::{Deserialize, Serialize};

/// Tracks usage count and last-used timestamp for a command.
#[derive(Clone, Debug)]
pub struct CommandUsage {
    pub count: u32,
    pub last_used: f64,
}

/// An atomic region in the input buffer (grok parity).
///
/// - Paste chips (`label: Some("[Pasted: N lines]")`): created when pasting
///   more than 3 lines; the label replaces the buffer text when rendering.
/// - Mention chips (`label: None`): created when picking a file from the `@`
///   picker; the buffer text renders as-is.
///
/// Backspace with the cursor exactly at a chip's `end` deletes the whole
/// chip in one press; editing inside a chip dissolves it.
/// Offsets are byte offsets into `InputState::input`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputChip {
    /// Start byte offset in `input` (inclusive).
    pub start: usize,
    /// End byte offset in `input` (exclusive).
    pub end: usize,
    /// Display label replacing the buffer text; `None` = render buffer text.
    pub label: Option<String>,
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
    /// Atomic regions (paste chips, picked @-mentions) — see `InputChip`.
    #[serde(default)]
    pub chips: Vec<InputChip>,
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
            chips: Vec::new(),
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

    /// Insert pasted text at the cursor: CRLF/CR normalize to LF, tabs to 4
    /// spaces. Pasting more than 3 lines records a `[Pasted: N lines]` chip
    /// (grok parity) — the full text stays in the buffer.
    pub fn insert_paste(&mut self, text: &str) {
        let clean = text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\t', "    ");
        if clean.is_empty() {
            return;
        }
        self.push_undo();
        let start = self.cursor_pos;
        self.adjust_chips_for_replace(start, start, clean.len());
        self.input.insert_str(start, &clean);
        self.cursor_pos += clean.len();
        let n_lines = clean.lines().count();
        if n_lines > 3 {
            self.chips.push(InputChip {
                start,
                end: start + clean.len(),
                label: Some(format!("[Pasted: {} lines]", n_lines)),
            });
        }
        self.redo_stack.clear();
    }

    /// Adjust chip spans after replacing `[start, end)` with `new_len` bytes.
    /// Chips intersecting the replaced range dissolve; chips after it shift.
    pub fn adjust_chips_for_replace(&mut self, start: usize, end: usize, new_len: usize) {
        if self.chips.is_empty() {
            return;
        }
        let delta = new_len as isize - (end - start) as isize;
        self.chips.retain_mut(|chip| {
            if chip.end <= start {
                true
            } else if chip.start >= end {
                chip.start = (chip.start as isize + delta).max(0) as usize;
                chip.end = (chip.end as isize + delta).max(0) as usize;
                true
            } else {
                false
            }
        });
    }

    /// The chip whose `end` is exactly at the cursor, if any.
    pub fn chip_at_cursor_end(&self) -> Option<InputChip> {
        self.chips
            .iter()
            .find(|c| c.end == self.cursor_pos && c.start < c.end)
            .cloned()
    }

    /// `(display_text, display_cursor)` for rendering: labeled chips are
    /// substituted with their label and the cursor is mapped accordingly.
    /// Out-of-range chips (e.g. during the file-picker Clear window) are
    /// skipped so rendering never panics on stale spans.
    pub fn display_view(&self) -> (String, usize) {
        let mut labeled: Vec<&InputChip> = self
            .chips
            .iter()
            .filter(|c| c.label.is_some() && c.start < c.end && c.end <= self.input.len())
            .collect();
        if labeled.is_empty() {
            return (self.input.clone(), self.cursor_pos);
        }
        labeled.sort_by_key(|c| c.start);
        let mut text = self.input.clone();
        let mut display_cursor = self.cursor_pos;
        let mut shift: isize = 0;
        for chip in labeled {
            let label = chip.label.as_ref().unwrap();
            let dstart = (chip.start as isize + shift) as usize;
            let dend = (chip.end as isize + shift) as usize;
            if self.cursor_pos >= chip.end {
                display_cursor =
                    (display_cursor as isize - (chip.end - chip.start) as isize + label.len() as isize) as usize;
            } else if self.cursor_pos > chip.start {
                // Cursor inside a chip: clamp to just after the label.
                display_cursor = dstart + label.len();
            }
            text.replace_range(dstart..dend, label);
            shift += label.len() as isize - (chip.end - chip.start) as isize;
        }
        (text, display_cursor)
    }
}
