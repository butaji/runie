//! Model — Application State (mutable borrow, no cloning per event)
use crate::ui::elements::Element;

// Animation constants
const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
const SPINNER_FRAMES: u32 = 12;

pub const PANEL_CHAT: &str = " Chat ";
pub const PANEL_INPUT: &str = " Input ";

#[derive(Clone)]
pub struct AppState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub streaming: bool,
    pub scroll: usize,
    pub thinking_started_at: Option<std::time::Instant>,
    pub request_queue: Vec<(String, String)>,  // (content, id)
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
    element_count: usize,
    /// Cached elements — rebuilt lazily via ensure_fresh()
    elements_cache: Vec<Element>,
    /// Dirty flag — true when cache needs rebuild
    dirty: bool,
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

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Rebuild cache only when dirty — O(n) but gated
    pub fn ensure_fresh(&mut self) {
        if self.dirty {
            use crate::ui::transform::LazyCache;
            self.elements_cache = LazyCache::rebuild(self);
            self.element_count = self.elements_cache.len();
            self.dirty = false;
        }
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
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: f64,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Cyan, Green, Yellow, DarkGray, White, Magenta,
}
