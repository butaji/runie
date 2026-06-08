//! Model — Application State (mutable borrow, no cloning per event)
use crate::snapshot::Snapshot;
use crate::ui::elements::Element;


const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
const SPINNER_FRAMES: u32 = 12;

/// A viewport into the element cache — elements plus how many
/// lines to skip from the top of the first element.
#[derive(Clone, Copy)]
pub struct VisibleRegion<'a> {
    pub elements: &'a [Element],
    pub skip_lines: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedMessageKind {
    Steering,
    FollowUp,
}

#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub content: String,
    pub kind: QueuedMessageKind,
}

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    /// Cursor position in input (0 = before first char)
    pub cursor_pos: usize,
    pub streaming: bool,
    pub scroll: usize,
    pub thinking_started_at: Option<std::time::Instant>,
    pub request_queue: std::collections::VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    pub next_id: u64,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    pub intermediate_step_count: usize,
    pub animation_frame: u32,
    pub turn_active: bool,
    pub current_action: Option<String>,
    pub current_provider: String,
    pub current_model: String,
    /// Index of last tool message — avoids O(n) reverse search in end_tool()
    pub(crate) last_tool_index: Option<usize>,
    /// Number of commands sent to agent but not yet completed
    pub inflight: usize,
    /// Monotonic counter — increments on every snapshot sent to render actor
    pub render_generation: u64,
    /// @-ref file lookup suggestions
    pub at_suggestions: Option<Vec<String>>,
    /// Selected index in @-ref suggestions
    pub at_selected: Option<usize>,
    /// Last @-ref query to avoid redundant filesystem calls
    pub last_at_query: Option<String>,
    /// Global collapse flag — when true, ALL thoughts/tools render collapsed.
    /// New elements automatically respect this setting.
    pub all_collapsed: bool,
    pub(crate) thought_seq: u64,
    pub(crate) input_history: Vec<String>,
    pub(crate) history_pos: Option<usize>,
    pub(crate) undo_stack: Vec<(String, usize)>,
    pub(crate) redo_stack: Vec<(String, usize)>,
    pub input_flash: u8,
    pub placeholder: String,
    element_count: usize,
    elements_cache: Vec<Element>,
    line_counts: Vec<usize>,
    total_lines: usize,
    dirty: bool,
    message_gen: u64,
    cached_gen: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(), input: String::new(), cursor_pos: 0,
            streaming: false, scroll: 0, thinking_started_at: None,
            request_queue: std::collections::VecDeque::new(),
            message_queue: Vec::new(), next_id: 0,
            current_request_id: None, turn_started_at: None,
            current_tool_name: None, tool_started_at: None,
            intermediate_step_count: 0, animation_frame: 0,
            turn_active: false, current_action: None,
            current_provider: "mock".into(), current_model: "echo".into(),
            last_tool_index: None, inflight: 0, render_generation: 0,
            at_suggestions: None, at_selected: None, last_at_query: None,
            all_collapsed: false, thought_seq: 0,
            input_history: Vec::new(), history_pos: None,
            undo_stack: Vec::new(), redo_stack: Vec::new(),
            input_flash: 0, placeholder: "Type a message to start...".into(),
            element_count: 0, elements_cache: Vec::new(),
            line_counts: Vec::new(), total_lines: 0,
            dirty: true, message_gen: 1, cached_gen: 0,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.turn_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.tool_started_at.map(|t| t.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle)
    pub fn spinner_frame(&self) -> char {
        SPINNER_CHARS[(self.animation_frame % SPINNER_FRAMES) as usize]
    }

    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        id
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn messages_changed(&mut self) {
        self.message_gen = self.message_gen.wrapping_add(1);
        self.dirty = true;
    }

    pub fn cache_generation(&self) -> u64 {
        self.message_gen
    }

    /// Rebuild cache only when messages changed — O(n) but gated
    pub fn ensure_fresh(&mut self) {
        if self.dirty && self.message_gen != self.cached_gen {
            self.elements_cache = crate::ui::LazyCache::rebuild(self);
            self.element_count = self.elements_cache.len();
            self.line_counts = self.elements_cache.iter().map(|e| e.line_count()).collect();
            self.total_lines = self.line_counts.iter().sum();
            self.cached_gen = self.message_gen;
        }
        self.dirty = false;
    }

    /// Visible elements slice — O(1), zero allocation
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.elements_cache.is_empty() {
            return &[];
        }
        let start = skip.min(self.element_count).min(self.elements_cache.len());
        let end = (start + take).min(self.element_count).min(self.elements_cache.len());
        &self.elements_cache[start..end]
    }

    pub fn count(&self) -> usize {
        self.element_count.max(self.elements_cache.len())
    }

    pub fn element_count(&self) -> usize {
        self.element_count
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub fn elements_cache(&self) -> &[Element] {
        &self.elements_cache
    }

    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.turn_active {
            self.animation_frame = self.animation_frame.wrapping_add(1);
            changed = true;
        }
        if self.input_flash > 0 {
            self.input_flash -= 1;
            changed = true;
        }
        if changed {
            self.dirty = true;
        }
    }

    /// Build an immutable Snapshot for the render actor.
    /// The event loop calls this after ensure_fresh(); the render
    /// actor receives it via channel and draws without touching state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            elements: self.elements_cache.clone(),
            line_counts: self.line_counts.clone(),
            total_lines: self.total_lines,
            input: self.input.clone(),
            cursor_pos: self.cursor_pos,
            hint_text: self.hint_text(),
            at_suggestions: self.at_suggestions.clone(),
            at_selected: self.at_selected,
            turn_active: self.turn_active,
            input_flash: self.input_flash,
            placeholder: self.placeholder.clone(),
            spinner_frame: self.spinner_frame(),
            scroll: self.scroll,
            turn_elapsed_secs: self.turn_elapsed_secs(),
            provider: self.current_provider.clone(),
            model: self.current_model.clone(),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn scroll_offset(&self, visible_height: usize) -> u16 {
        let max_scroll = self.total_lines.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);
        max_scroll.saturating_sub(scroll).min(u16::MAX as usize) as u16
    }

    pub fn scrollbar_metrics(&self, visible_height: usize) -> (usize, usize) {
        let total = self.total_lines;
        if total <= visible_height || visible_height == 0 {
            return (0, 0);
        }
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);
        let position = max_scroll.saturating_sub(scroll);
        let track = visible_height;
        let thumb = (visible_height * visible_height / total).max(1);
        let thumb_offset = if max_scroll > 0 {
            position * (track - thumb) / max_scroll
        } else {
            0
        };
        (thumb, thumb_offset)
    }

    pub fn visible_scroll(&self, visible_height: usize) -> VisibleRegion<'_> {
        if self.elements_cache.is_empty() || visible_height == 0 {
            return VisibleRegion { elements: &[], skip_lines: 0 };
        }

        let total = self.total_lines;
        let max_scroll = total.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);

        let viewport_end = total.saturating_sub(scroll);
        let viewport_start = viewport_end.saturating_sub(visible_height);

        let mut cum = 0usize;
        let mut start_idx = 0;
        let mut skip_lines = 0;

        for (i, count) in self.line_counts.iter().enumerate() {
            let next_cum = cum + count;
            if next_cum > viewport_start {
                start_idx = i;
                skip_lines = viewport_start.saturating_sub(cum);
                break;
            }
            cum = next_cum;
        }

        let mut end_idx = self.elements_cache.len();
        cum = 0;
        for (i, count) in self.line_counts.iter().enumerate() {
            cum += count;
            if cum >= viewport_end {
                end_idx = i + 1;
                break;
            }
        }

        VisibleRegion {
            elements: &self.elements_cache[start_idx..end_idx.min(self.elements_cache.len())],
            skip_lines,
        }
    }

    pub fn total_tokens(&self) -> usize {
        self.messages.iter().map(|m| crate::tokens::estimate_tokens(&m.content)).sum()
    }

    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        let total = self.total_tokens();
        if total <= keep_recent_tokens {
            return format!("Session has {} tokens, no compaction needed", total);
        }
        let mut accumulated = 0usize;
        let mut cut_idx = 0usize;
        for (i, msg) in self.messages.iter().enumerate().rev() {
            accumulated += crate::tokens::estimate_tokens(&msg.content);
            if accumulated >= keep_recent_tokens {
                cut_idx = i;
                break;
            }
        }
        while cut_idx < self.messages.len() {
            match self.messages[cut_idx].role {
                Role::User | Role::Assistant => break,
                _ => cut_idx += 1,
            }
        }
        if cut_idx == 0 {
            return "Cannot compact: all messages are recent".to_string();
        }
        let removed_count = cut_idx;
        self.messages.drain(..cut_idx);
        let summary = format!("[Compacted: {} earlier messages removed, keeping ~{} tokens]", removed_count, keep_recent_tokens);
        self.messages.insert(0, ChatMessage {
            role: Role::System,
            content: summary.clone(),
            timestamp: now(),
            id: "compaction".to_string(),
        });
        self.messages_changed();
        summary
    }
}

pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    User,
    Thought,
    Assistant,
    Tool,
    TurnComplete,
    System,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Thought => "thought",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::TurnComplete => "turn_complete",
            Role::System => "system",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub timestamp: f64,
    pub id: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    User(String),
    Assistant(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Cyan,
    Green,
    Yellow,
    DarkGray,
    White,
    Magenta,
}
