//! State types for AppState and related structs.



/// Animation configuration from config file
#[derive(Clone)]
pub struct AnimationConfig {
    pub fps: u8,        // 1-60, default 30
    pub wave_rows: u16, // default 32
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            wave_rows: 32,
        }
    }
}

/// P1-REMAINING-1 FIX: Track Ctrl+C double-tap to prevent accidental text loss
#[derive(Clone)]
pub struct ClearInputConfirm {
    pub pending: bool,
    pub last_press: Option<std::time::Instant>,
}

impl Default for ClearInputConfirm {
    fn default() -> Self {
        Self {
            pending: false,
            last_press: None,
        }
    }
}

impl ClearInputConfirm {
    /// Check if the user wants to clear input (requires double-tap within 2 seconds)
    pub fn wants_clear(&mut self) -> bool {
        let now = std::time::Instant::now();
        const CLEAR_CONFIRM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

        if self.pending {
            if let Some(last) = self.last_press {
                if now.duration_since(last) < CLEAR_CONFIRM_TIMEOUT {
                    self.pending = false;
                    self.last_press = None;
                    return true;
                }
            }
            self.pending = false;
        }

        self.pending = true;
        self.last_press = Some(now);
        false
    }

    /// Check if there's a pending clear request (for showing hint)
    pub fn is_pending(&self) -> bool {
        self.pending
    }
}

#[derive(Clone)]
pub struct AnimationState {
    pub braille_frame: usize,
    pub rewind_braille_frame: usize,
    pub streaming_cursor_visible: bool,
    pub interrupt_fade_start: Option<std::time::Instant>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            braille_frame: 0,
            rewind_braille_frame: 0,
            streaming_cursor_visible: true,
            interrupt_fade_start: None,
        }
    }
}

#[derive(Clone)]
pub struct TopBarState {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub model: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub agent_count: Option<usize>,
    pub context_badges: Vec<String>,
    pub context_pct: Option<f32>,
    pub context_bar_pct: Option<f32>,
    pub context_window: Option<usize>,
    pub estimated_tokens: Option<usize>,
}

impl Default for TopBarState {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            checks_passed: None,
            checks_total: None,
            percentage: None,
            agent_count: None,
            context_badges: Vec::new(),
            context_pct: None,
            context_bar_pct: None,
            context_window: Some(128_000),
            estimated_tokens: Some(0),
        }
    }
}

/// ContextState holds git info and context badges for global tags display.
#[derive(Clone)]
pub struct ContextState {
    pub repo: String,
    pub branch: String,
    pub path: String,
    pub checks_passed: Option<usize>,
    pub checks_total: Option<usize>,
    pub percentage: Option<f32>,
    pub context_badges: Vec<String>,
    pub context_pct: Option<f32>,
    pub context_bar_pct: Option<f32>,
}

impl Default for ContextState {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            checks_passed: None,
            checks_total: None,
            percentage: None,
            context_badges: Vec::new(),
            context_pct: None,
            context_bar_pct: None,
        }
    }
}

/// Pending permission request (queued when in blocking mode)
#[derive(Clone, Debug)]
pub struct PendingPermission {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_args: String,
}

#[derive(Clone)]
pub struct PermissionModalState {
    pub tool: Option<String>,
    pub args: Option<String>,
    pub desc: Option<String>,
    pub tool_call_id: Option<String>,
    pub timeout_start: Option<std::time::Instant>,
    pub timed_out: bool,
    pub pending_queue: Vec<PendingPermission>,
    pub show_advanced: bool,
}

impl Default for PermissionModalState {
    fn default() -> Self {
        Self {
            tool: None,
            args: None,
            desc: None,
            tool_call_id: None,
            timeout_start: None,
            timed_out: false,
            pending_queue: Vec::new(),
            show_advanced: false,
        }
    }
}

#[derive(Clone)]
pub struct CommandPaletteState {
    pub open: bool,
    pub filter: String,
    pub selected: usize,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            open: false,
            filter: String::new(),
            selected: 0,
        }
    }
}

#[derive(Clone)]
pub struct ScrollState {
    pub feed_offset: usize,
    pub diff_offset: usize,
    pub tree_offset: usize,
    pub user_scrolled_up: bool,
    pub scroll_focused: bool,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            feed_offset: 0,
            diff_offset: 0,
            tree_offset: 0,
            user_scrolled_up: false,
            scroll_focused: false,
        }
    }
}
