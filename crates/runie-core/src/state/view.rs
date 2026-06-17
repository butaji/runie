use std::sync::Arc;

use crate::model::ModelSelectorItem;
use crate::ui::elements::Element;

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
    // Animation/scroll state
    pub animation_frame: u32,
    pub all_collapsed: bool,
    /// Height of the message viewport in terminal rows, updated by
    /// the render actor on each draw. Used by vim nav mode to compute
    /// element-level jumps for `j`/`k`/arrow keys.
    pub last_visible_height: u16,
    /// Width of the message content area in terminal columns, updated by
    /// the render actor on each draw. Used to compute per-element line
    /// counts so that scroll math matches the actual wrapped output.
    pub last_content_width: u16,
    /// Index of the post currently selected in vim nav mode.
    /// A post is a logical unit in the feed (e.g. a user message, a
    /// thought, a tool call). Independent of scroll; used to highlight
    /// the selected post and to drive post-level navigation.
    pub selected_post: Option<usize>,
    // Cached palette items (for command palette dialog)
    pub(crate) cached_palette_items: Arc<[(String, String, String)]>,
    pub(crate) cached_palette_filter: Option<String>,
    // Cached model selector items
    pub(crate) cached_model_items: Arc<[ModelSelectorItem]>,
    pub(crate) cached_model_filter: Option<String>,
    // Cached settings items
    pub(crate) cached_settings_items: Arc<[crate::settings::SettingItem]>,
    pub(crate) cached_settings_valid: bool,
    // Cached session tree items
    pub(crate) cached_session_tree_items: Arc<[(usize, String)]>,
    pub(crate) cached_session_tree_valid: bool,
    // Cached auth provider names
    pub(crate) cached_auth_providers: Arc<[String]>,
    pub(crate) cached_auth_valid: bool,
    /// Navigable posts in the feed. Rebuilt alongside `elements_cache`.
    pub posts: Arc<[crate::ui::elements::Post]>,
    /// Last known mouse position from `MouseMove` events. Used by the TUI
    /// to compute `MouseTarget` for hover styling and click routing.
    pub mouse_position: Option<(u16, u16)>,
    /// Vim-style scrollback navigation active.
    pub vim_nav_mode: bool,
    /// When vim_mode Esc was used to abort a turn, the next Esc enters
    /// nav mode. Cleared once consumed or when a turn is no longer active.
    pub vim_nav_pending: bool,
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
            animation_frame: 0,
            all_collapsed: false,
            last_visible_height: 20,
            last_content_width: 80,
            cached_palette_items: Arc::new([]),
            cached_palette_filter: None,
            cached_model_items: Arc::new([]),
            cached_model_filter: None,
            cached_settings_items: Arc::new([]),
            cached_settings_valid: false,
            cached_session_tree_items: Arc::new([]),
            cached_session_tree_valid: false,
            cached_auth_providers: Arc::new([]),
            cached_auth_valid: false,
            selected_post: None,
            posts: Arc::new([]),
            mouse_position: None,
            vim_nav_mode: false,
            vim_nav_pending: false,
        }
    }
}
