use std::sync::Arc;

use crate::model::view_cache::ViewCache;
use crate::model::InputReceiver;
use crate::model::ModelSelectorItem;

/// View state — scroll, animation, and UI dimensions.
///
/// NOTE: View cache (elements, posts) has been moved out of `ViewState` into
/// `Snapshot` and `UiActor`. This eliminates the coupling between domain state
/// and view AST. However, scroll-computed values (total_lines, line_counts) are
/// kept here for the `ViewActor` to use in scroll calculations.
#[derive(Clone, Debug)]
pub struct ViewState {
    pub scroll: usize,
    pub dirty: bool,
    pub message_gen: u64,
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
    /// Posts individually expanded with Enter in feed navigation. Thoughts
    /// are collapsed to one-line summaries by default (grok parity), so this
    /// set applies in both global modes. Ephemeral UI state — cleared
    /// whenever the global collapse flag is toggled.
    pub expanded_posts: std::collections::HashSet<usize>,
    /// Total rendered lines in the feed. Updated when feed cache rebuilds.
    /// Used for scroll bound calculations.
    pub total_lines: usize,
    /// Cumulative line counts per element. Updated when feed cache rebuilds.
    /// Used for element-level scroll jumps.
    pub line_counts: Arc<[usize]>,
    // Cached palette items (for command palette dialog)
    pub cached_palette_items: Arc<[(String, String, String)]>,
    pub cached_palette_filter: Option<String>,
    // Cached model selector items
    pub cached_model_items: Arc<[ModelSelectorItem]>,
    pub cached_model_filter: Option<String>,
    // Cached settings items
    pub cached_settings_items: Arc<[crate::settings::SettingItem]>,
    pub cached_settings_valid: bool,
    // Cached session tree items
    pub cached_session_tree_items: Arc<[(usize, String)]>,
    pub cached_session_tree_valid: bool,
    // Cached auth provider names
    pub cached_auth_providers: Arc<[String]>,
    pub cached_auth_valid: bool,
    /// Vim-style scrollback navigation active.
    pub vim_nav_mode: bool,
    /// Reusable feed cache. Populated by `ensure_fresh()`; reused by
    /// `snapshot_feed()` when `message_gen` matches `cached_gen`.
    pub(crate) cached_feed: Option<ViewCache>,
    /// When vim_mode Esc was used to abort a turn, the next Esc enters
    /// nav mode. Cleared once consumed or when a turn is no longer active.
    pub vim_nav_pending: bool,
    /// Identifies which component is currently receiving keyboard input.
    /// Used to determine how Esc should behave (e.g., close dialog vs enter vim-nav).
    pub input_receiver: InputReceiver,
    /// Plan mode active — blocks write tools until plan is approved.
    pub plan_mode: bool,
    /// Auto-approve mode active — read, edit and shell tools run without
    /// confirmation (sensitive paths still ask). Session-scoped; never
    /// persisted across restarts.
    pub auto_mode: bool,
    /// Content of the active plan (markdown).
    pub active_plan_content: String,
    /// ID of the active plan file.
    pub active_plan_id: Option<String>,
    /// Grok-style tasks pane visibility.
    pub tasks_pane_visible: bool,
    /// Show completed workers in the tasks pane (true when no workers are running).
    pub tasks_pane_show_done: bool,
    /// Open subagent detail overlay state.
    pub subagent_detail: Option<crate::model::SubagentDetail>,
}

impl PartialEq for ViewState {
    fn eq(&self, other: &Self) -> bool {
        // Only compare essential navigation state, not caches (caches are runtime-only)
        self.scroll == other.scroll
            && self.dirty == other.dirty
            && self.message_gen == other.message_gen
            && self.all_collapsed == other.all_collapsed
            && self.last_visible_height == other.last_visible_height
            && self.last_content_width == other.last_content_width
            && self.selected_post == other.selected_post
            && self.expanded_posts == other.expanded_posts
            && self.vim_nav_mode == other.vim_nav_mode
    }
}

impl ViewState {
    // Mutable accessors for tests
    pub fn scroll_mut(&mut self) -> &mut usize {
        &mut self.scroll
    }

    pub fn dirty_mut(&mut self) -> &mut bool {
        &mut self.dirty
    }

    pub fn all_collapsed_mut(&mut self) -> &mut bool {
        &mut self.all_collapsed
    }

    pub fn last_visible_height_mut(&mut self) -> &mut u16 {
        &mut self.last_visible_height
    }

    pub fn vim_nav_mode_mut(&mut self) -> &mut bool {
        &mut self.vim_nav_mode
    }

    pub fn plan_mode_mut(&mut self) -> &mut bool {
        &mut self.plan_mode
    }

    pub fn auto_mode_mut(&mut self) -> &mut bool {
        &mut self.auto_mode
    }

    pub fn active_plan_content_mut(&mut self) -> &mut String {
        &mut self.active_plan_content
    }

    pub fn active_plan_id_mut(&mut self) -> &mut Option<String> {
        &mut self.active_plan_id
    }
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            scroll: 0,
            dirty: true,
            message_gen: 1,
            animation_frame: 0,
            all_collapsed: false,
            last_visible_height: 20,
            last_content_width: 82, // area width; rendering subtracts 2 for glyph margins
            selected_post: None,
            expanded_posts: std::collections::HashSet::new(),
            total_lines: 0,
            line_counts: Arc::new([]),
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
            vim_nav_mode: false,
            cached_feed: None,
            vim_nav_pending: false,
            input_receiver: InputReceiver::default(),
            plan_mode: false,
            auto_mode: false,
            active_plan_content: String::new(),
            active_plan_id: None,
            tasks_pane_visible: false,
            tasks_pane_show_done: false,
            subagent_detail: None,
        }
    }
}
