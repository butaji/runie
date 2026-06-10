use std::collections::VecDeque;
use std::sync::Arc;

use crate::keybindings::default_keybindings;
use crate::message::{ChatMessage, now};
use crate::model::{QueuedMessage, ThinkingLevel};
use crate::path_complete::PathCompletion;
use crate::scoped_model::ScopedModel;
use crate::session_tree::SessionTree;
use crate::ui::elements::Element;

#[derive(Clone)]
pub struct InputState {
    pub input: String,
    pub cursor_pos: usize,
    pub(crate) undo_stack: Vec<(String, usize)>,
    pub(crate) redo_stack: Vec<(String, usize)>,
    pub(crate) history_pos: Option<usize>,
    pub input_flash: u8,
    pub placeholder: String,
    /// Ghost completion suffix shown in gray after the cursor.
    pub ghost_completion: Option<String>,
    /// Tab-completion state stored as raw fields (avoid circular dep).
    pub tab_complete_prefix: Option<String>,
    pub tab_complete_matches: Vec<String>,
    pub tab_complete_index: usize,
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
        }
    }
}

#[derive(Clone, Default)]
pub struct AgentState {
    pub request_queue: VecDeque<(String, String)>,
    pub message_queue: Vec<QueuedMessage>,
    pub current_request_id: Option<String>,
    pub turn_started_at: Option<std::time::Instant>,
    pub turn_active: bool,
    pub inflight: usize,
    pub current_tool_name: Option<String>,
    pub tool_started_at: Option<std::time::Instant>,
}

#[derive(Clone)]
pub struct ViewState {
    pub scroll: usize,
    pub elements_cache: Arc<[Element]>,
    pub line_counts: Arc<[usize]>,
    pub total_lines: usize,
    pub dirty: bool,
    pub cached_gen: u64,
    pub message_gen: u64,
    pub element_count: usize,
}

impl ViewState {
    pub fn elements_cache(&self) -> &[Element] {
        self.elements_cache.as_ref()
    }

    pub fn line_counts(&self) -> &[usize] {
        self.line_counts.as_ref()
    }

    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub fn element_count(&self) -> usize {
        self.element_count
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            scroll: 0,
            elements_cache: Arc::new([]),
            line_counts: Arc::new([]),
            total_lines: 0,
            dirty: true,
            cached_gen: 0,
            message_gen: 1,
            element_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct SessionState {
    pub messages: Vec<ChatMessage>,
    pub session_tree: Option<SessionTree>,
    pub session_display_name: Option<String>,
    pub session_created_at: f64,
    pub session_updated_at: f64,
}

impl Default for SessionState {
    fn default() -> Self {
        let t = now();
        Self {
            messages: Vec::new(),
            session_tree: None,
            session_display_name: None,
            session_created_at: t,
            session_updated_at: t,
        }
    }
}

#[derive(Clone)]
pub struct ConfigState {
    pub current_provider: String,
    pub current_model: String,
    pub config_provider: String,
    pub config_model: String,
    pub keybindings: std::collections::HashMap<String, String>,
    pub theme_name: String,
    pub thinking_level: ThinkingLevel,
    pub read_only: bool,
    pub scoped_models: Vec<ScopedModel>,
    pub scoped_index: usize,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            current_provider: "mock".into(),
            current_model: "echo".into(),
            config_provider: "mock".into(),
            config_model: "echo".into(),
            keybindings: default_keybindings(),
            theme_name: "runie".into(),
            thinking_level: ThinkingLevel::Off,
            read_only: false,
            scoped_models: Vec::new(),
            scoped_index: 0,
        }
    }
}

#[derive(Clone, Default)]
pub struct CompletionState {
    pub path_suggestions: Option<Vec<PathCompletion>>,
    pub path_selected: Option<usize>,
    pub at_suggestions: Option<Vec<String>>,
    pub at_selected: Option<usize>,
    pub last_at_query: Option<String>,
}


