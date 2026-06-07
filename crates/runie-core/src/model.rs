//! Model — Application State (mutable borrow, no cloning per event)
use crate::ui::elements::Element;
use std::collections::HashSet;


const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
const SPINNER_FRAMES: u32 = 12;

pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

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
    /// Collapsed element ids (hidden in TUI) — thoughts, tools, etc.
    pub collapsed: HashSet<String>,
    element_count: usize,
    elements_cache: Vec<Element>,
    dirty: bool,
    message_gen: u64,
    cached_gen: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            streaming: false,
            scroll: 0,
            thinking_started_at: None,
            request_queue: std::collections::VecDeque::new(),
            message_queue: Vec::new(),
            next_id: 0,
            current_request_id: None,
            turn_started_at: None,
            current_tool_name: None,
            tool_started_at: None,
            intermediate_step_count: 0,
            animation_frame: 0,
            turn_active: false,
            current_action: None,
            current_provider: "mock".to_string(),
            current_model: "echo".to_string(),
            last_tool_index: None,
            inflight: 0,
            render_generation: 0,
            at_suggestions: None,
            at_selected: None,
            last_at_query: None,
            collapsed: HashSet::new(),
            element_count: 0,
            elements_cache: Vec::new(),
            dirty: true,
            message_gen: 1,
            cached_gen: 0,
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

    pub fn elements_cache(&self) -> &[Element] {
        &self.elements_cache
    }

    pub fn tick_animation(&mut self) {
        if self.turn_active {
            self.animation_frame = self.animation_frame.wrapping_add(1);
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
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

fn now() -> f64 {
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
