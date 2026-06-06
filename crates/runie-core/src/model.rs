//! Model — Application State
//!
//! Cache Architecture (lazy invalidation pattern):
//!
//! ```text
//! Messages (source of truth)
//!    │
//!    │ update() — push/delete/modify
//!    ▼
//! dirty = true ───────────────────────┐
//!    │                                  │
//!    │                                  │ ensure_fresh() — O(n) rebuild
//!    ▼                                  ▼
//! ┌─────────────────────────────────────────────┐
//! │  elements_cache: Vec<Element>               │
//! │  element_count: usize                       │
//! │  dirty: bool                                │
//! └─────────────────────────────────────────────┘
//!    │
//!    │ visible(skip, take) — O(1) slice
//!    ▼
//! UI renders visible viewport only
//! ```
//!
//! Invariants:
//! - `dirty == false` → `elements_cache` and `element_count` are valid
//! - `dirty == true`  → cache is stale, must call `ensure_fresh()` before read
//! - All mutations go through `update()` which sets `dirty = true`
//!
//! Performance:
//! - Write: O(1) amortized (push to Vec + set dirty flag)
//! - Read:  O(1) for count, O(take) for visible slice
//! - Rebuild: O(n) but only when dirty, batched between frames

use crate::ui::elements::Element;

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub streaming: bool,
    pub scroll: usize,
    pub thinking_started_at: Option<std::time::Instant>,
    pub request_queue: Vec<(String, String)>,
    pub next_id: u64,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
    pub has_intermediate_steps: bool,
    pub animation_frame: u32,
    pub turn_active: bool,
    pub current_action: Option<String>,
    /// Cached element count — O(1) access when not dirty
    pub element_count: usize,
    /// Cached elements — rebuilt lazily via ensure_fresh()
    pub elements_cache: Vec<Element>,
    /// Dirty flag — true when messages changed, cache needs rebuild
    pub dirty: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            streaming: false,
            scroll: 0,
            thinking_started_at: None,
            request_queue: Vec::new(),
            next_id: 0,
            current_request_id: None,
            turn_started_at: None,
            current_tool_name: None,
            tool_started_at: None,
            has_intermediate_steps: false,
            animation_frame: 0,
            turn_active: false,
            current_action: None,
            element_count: 0,
            elements_cache: Vec::new(),
            dirty: true,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.thinking_started_at
            .map(|start| start.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.turn_started_at.map(|start| start.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.tool_started_at.map(|start| start.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle)
    pub fn spinner_frame(&self) -> char {
        const SPINNERS: &[char] =
            &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
        SPINNERS[(self.animation_frame % 12) as usize]
    }

    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Mark cache dirty — must call ensure_fresh() before read
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Rebuild cache if dirty — O(n) but only when needed
    /// After this: count() and visible() are O(1)
    pub fn ensure_fresh(&mut self) {
        if self.dirty {
            use crate::ui::transform::LazyCache;
            self.elements_cache = LazyCache::rebuild(self);
            self.element_count = self.elements_cache.len();
            self.dirty = false;
        }
    }

    /// Get visible elements as slice — zero allocation
    /// Panic-free: returns empty if cache is stale
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.elements_cache.is_empty() {
            return &[];
        }
        let start = skip.min(self.element_count).min(self.elements_cache.len());
        let end = (skip + take)
            .min(self.element_count)
            .min(self.elements_cache.len());
        &self.elements_cache[start..end]
    }

    /// O(1) element count — valid when not dirty
    pub fn count(&self) -> usize {
        self.element_count.max(self.elements_cache.len())
    }
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
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

pub const PANEL_CHAT: &str = "Chat";
pub const PANEL_INPUT: &str = "Input";
