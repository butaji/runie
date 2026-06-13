//! Model — Application State (mutable borrow, no cloning per event)
pub use crate::message::{now, ChatMessage, Role};
use crate::snapshot::{compute_current_top_element, Snapshot};
use crate::ui::elements::Element;
use std::sync::Arc;

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
const SPINNER_FRAMES: u32 = 12;

/// Detect git repo name and current branch from the given directory.
/// Walks up the tree looking for `.git` (dir or file with `gitdir:` pointer).
pub fn detect_git_info(start: &std::path::Path) -> Option<crate::snapshot::GitInfo> {
    let mut current = Some(start);
    while let Some(dir) = current {
        let git_path = dir.join(".git");
        if git_path.is_dir() {
            return read_git_info(&git_path);
        }
        if git_path.is_file() {
            // Worktree: `.git` file contains `gitdir: <path>`
            let gitdir = std::fs::read_to_string(&git_path).ok().and_then(|content| {
                content
                    .trim()
                    .strip_prefix("gitdir:")
                    .map(|s| std::path::PathBuf::from(s.trim()))
            });
            if let Some(worktree_gitdir) = gitdir {
                // HEAD is in the worktree gitdir; config is in the parent repo
                let head_path = worktree_gitdir.join("HEAD");
                let branch = std::fs::read_to_string(&head_path)
                    .ok()
                    .and_then(|content| {
                        content
                            .trim()
                            .strip_prefix("ref: refs/heads/")
                            .map(|b| b.to_string())
                    });
                // Config is one level up from worktrees/<name>
                let config_path = worktree_gitdir
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("config"));
                let repo_name = config_path.and_then(|p| read_origin_repo_name(&p));
                return Some(crate::snapshot::GitInfo { repo_name, branch });
            }
        }
        current = dir.parent();
    }
    None
}

fn read_git_info(git_dir: &std::path::Path) -> Option<crate::snapshot::GitInfo> {
    let head_path = git_dir.join("HEAD");
    let branch = std::fs::read_to_string(&head_path)
        .ok()
        .and_then(|content| {
            content
                .trim()
                .strip_prefix("ref: refs/heads/")
                .map(|b| b.to_string())
        });
    let config_path = git_dir.join("config");
    let repo_name = read_origin_repo_name(&config_path);
    Some(crate::snapshot::GitInfo { repo_name, branch })
}

fn read_origin_repo_name(config_path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(config_path)
        .ok()
        .and_then(|config| {
            config
                .lines()
                .skip_while(|line| !line.contains("[remote \"origin\"]"))
                .skip(1)
                .find(|line| line.trim().starts_with("url"))
                .and_then(|url_line| {
                    let url = url_line.split('=').nth(1)?;
                    let url = url.trim();
                    url.rsplit('/')
                        .next()
                        .map(|name| name.trim_end_matches(".git").to_string())
                })
        })
}

/// Get the current working directory name.
pub fn current_dir_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default()
}

/// Approximate token count from text (4 chars ≈ 1 token).
pub fn count_tokens(text: &str) -> usize {
    text.chars().count() / 4
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueuedMessageKind {
    Steering,
    FollowUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DeliveryMode {
    /// Each message triggers a separate LLM call
    #[default]
    OneAtATime,
    /// All queued messages delivered together in one LLM call
    All,
}

/// Thinking level for reasoning-intensive tasks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ThinkingLevel {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

impl ThinkingLevel {
    /// All thinking levels in cycle order (low → high).
    /// Single source of truth for UI selectors.
    pub const ALL: &'static [ThinkingLevel] = &[
        ThinkingLevel::Off,
        ThinkingLevel::Low,
        ThinkingLevel::Medium,
        ThinkingLevel::High,
    ];

    /// All thinking levels in cycle order. See [`ALL`](Self::ALL).
    pub fn all() -> &'static [ThinkingLevel] {
        Self::ALL
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }

    pub fn prompt_suffix(&self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Low => "\nThink briefly before responding.",
            Self::Medium => "\nThink step by step before responding.",
            Self::High => "\nThink deeply and thoroughly. Consider edge cases and alternatives.",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Returns the I Ching hexagram for this thinking level.
    /// Maps to 3-bit representation: 000=earth, 111=heaven.
    pub fn hexagram(&self) -> &'static str {
        match self {
            Self::Off => "☷",    // 000 - earth (no thinking)
            Self::Low => "☵",    // 010 - water (minimal thinking)
            Self::Medium => "☳", // 100 - thunder (moderate thinking)
            Self::High => "☰",   // 111 - heaven (deep thinking)
        }
    }
}

impl std::str::FromStr for ThinkingLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(format!("Unknown thinking level: {s}")),
        }
    }
}
/// Initialize git info and cwd name once at startup.
pub fn init_git_and_cwd() -> (Option<crate::snapshot::GitInfo>, String) {
    let cwd = std::env::current_dir().ok();
    let cwd_name = cwd
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let git_info = cwd.as_ref().and_then(|p| detect_git_info(p));
    (git_info, cwd_name)
}

pub use crate::scoped_model::ScopedModel;

pub use crate::model_catalog::{
    build_model_selector_items, filter_models, model_catalog, ModelInfo,
};
#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub content: String,
    pub kind: QueuedMessageKind,
}

#[derive(Clone)]
pub struct AppState {
    // 6 inner state structs (factored domain state)
    pub session: crate::state::SessionState,
    pub input: crate::state::InputState,
    pub agent: crate::state::AgentState,
    pub view: crate::state::ViewState,
    pub config: crate::state::ConfigState,
    pub completion: crate::state::CompletionState,

    // Singleton UI/control flags (don't fit a single domain)
    /// Quit flag read by the main event loop
    pub should_quit: bool,
    /// Currently open overlay dialog (palette, model selector, etc.)
    pub open_dialog: Option<crate::commands::DialogState>,
    /// Stack for nested dialog navigation (Esc pops, restoring parent)
    pub dialog_back_stack: Vec<crate::commands::DialogState>,
    /// Active login/auth flow overlay
    pub login_flow: Option<crate::login_flow::LoginFlowState>,
    /// Command registry (loaded once, immutable per session)
    pub registry: crate::commands::CommandRegistry,
    /// Loaded skill definitions
    pub skills: Vec<crate::skills::Skill>,
    /// Loaded prompt templates
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    /// Transient notification message (cleared after timeout)
    pub transient_message: Option<String>,
    pub transient_until: Option<std::time::Instant>,
    pub transient_level: Option<crate::event::TransientLevel>,
    /// Git info detected at startup (repo name, branch)
    pub git_info: Option<crate::snapshot::GitInfo>,
    /// Current working directory name (detected at startup)
    pub cwd_name: String,
    /// Command input history (persistent across sessions)
    pub input_history: Vec<String>,
    /// True while the user is in vim feed-navigation mode (j/k/g/G etc.).
    /// Only meaningful when `config.vim_mode` is enabled.
    pub vim_nav_mode: bool,
    /// When vim_mode Esc was used to abort a turn, the next Esc enters
    /// nav mode. Cleared once consumed or when a turn is no longer active.
    pub vim_nav_pending: bool,
    /// Backup of input state before opening file picker:
    /// (original input, insert position, needs brackets for @ references).
    pub file_picker_backup: Option<(String, usize, bool)>,
}

impl Default for AppState {
    fn default() -> Self {
        let (git_info, cwd_name) = crate::model::init_git_and_cwd();
        Self {
            session: crate::state::SessionState::default(),
            input: crate::state::InputState::default(),
            agent: crate::state::AgentState::default(),
            view: crate::state::ViewState::default(),
            config: crate::state::ConfigState::default(),
            completion: crate::state::CompletionState::default(),
            should_quit: false,
            open_dialog: None,
            dialog_back_stack: Vec::new(),
            login_flow: None,
            registry: crate::commands::CommandRegistry::new(),
            skills: Vec::new(),
            prompts: Vec::new(),
            transient_message: None,
            transient_until: None,
            transient_level: None,
            git_info,
            cwd_name,
            input_history: Vec::new(),
            vim_nav_mode: false,
            vim_nav_pending: false,
            file_picker_backup: None,
        }
    }
}

impl AppState {
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .thinking_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .turn_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .tool_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle)
    pub fn spinner_frame(&self) -> char {
        SPINNER_CHARS[(self.view.animation_frame % SPINNER_FRAMES) as usize]
    }

    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.agent.next_id);
        self.agent.next_id += 1;
        id
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.view.dirty = true;
    }

    pub fn messages_changed(&mut self) {
        self.view.message_gen = self.view.message_gen.wrapping_add(1);
        self.session.session_updated_at = now();
        self.view.dirty = true;
    }

    /// Record the height of the message viewport. Called by the render
    /// actor on each draw. Used by vim nav mode for element-level jumps.
    pub fn set_last_visible_height(&mut self, height: u16) {
        self.view.last_visible_height = height;
    }

    fn palette_items(&mut self) -> Arc<[(String, String, String)]> {
        let filter = match &self.open_dialog {
            Some(d) => d
                .panel_stack()
                .current()
                .map(|p| p.filter.clone())
                .unwrap_or_default(),
            _ => {
                self.view.cached_palette_filter = None;
                if self.view.cached_palette_items.is_empty() {
                    return Arc::clone(&self.view.cached_palette_items);
                }
                self.view.cached_palette_items = Arc::new([]);
                return Arc::clone(&self.view.cached_palette_items);
            }
        };
        if Some(&filter) != self.view.cached_palette_filter.as_ref() {
            self.view.cached_palette_filter = Some(filter.clone());
            let mut items: Vec<_> = crate::commands::filter_commands(&self.registry, &filter)
                .into_iter()
                .map(|cmd| {
                    (
                        cmd.name.clone(),
                        cmd.desc.clone(),
                        cmd.category.as_str().to_string(),
                    )
                })
                .collect();
            let f = filter.to_lowercase();
            for skill in &self.skills {
                if skill.user_invocable
                    && (f.is_empty()
                        || skill.name.to_lowercase().contains(&f)
                        || skill.description.to_lowercase().contains(&f))
                {
                    items.push((
                        skill.name.clone(),
                        skill.description.clone(),
                        "Skill".to_string(),
                    ));
                }
            }
            self.view.cached_palette_items = items.into();
        }
        Arc::clone(&self.view.cached_palette_items)
    }

    fn session_tree_items(&mut self) -> Arc<[(usize, String)]> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::SessionTree(_)) => {
                crate::session_tree::SessionTreeFilter::All
            }
            _ => {
                self.view.cached_session_tree_valid = false;
                if self.view.cached_session_tree_items.is_empty() {
                    return Arc::clone(&self.view.cached_session_tree_items);
                }
                self.view.cached_session_tree_items = Arc::new([]);
                return Arc::clone(&self.view.cached_session_tree_items);
            }
        };
        if !self.view.cached_session_tree_valid {
            self.view.cached_session_tree_items = match self.session.session_tree.as_ref() {
                Some(tree) => tree
                    .filtered_walk(filter)
                    .into_iter()
                    .map(|(depth, node)| {
                        let preview = format!(
                            "[{}] {}",
                            node.message.role.as_str(),
                            node.message.content.chars().take(60).collect::<String>()
                        );
                        (depth, preview)
                    })
                    .collect::<Vec<_>>()
                    .into(),
                None => Arc::new([]),
            };
            self.view.cached_session_tree_valid = true;
        }
        Arc::clone(&self.view.cached_session_tree_items)
    }

    fn model_selector_items(&mut self) -> Arc<[(String, String, String, bool, bool)]> {
        let filter = match &self.open_dialog {
            Some(d) => d
                .panel_stack()
                .current()
                .map(|p| p.filter.clone())
                .unwrap_or_default(),
            _ => {
                self.view.cached_model_filter = None;
                if self.view.cached_model_items.is_empty() {
                    return Arc::clone(&self.view.cached_model_items);
                }
                self.view.cached_model_items = Arc::new([]);
                return Arc::clone(&self.view.cached_model_items);
            }
        };
        if Some(&filter) != self.view.cached_model_filter.as_ref() {
            self.view.cached_model_filter = Some(filter.clone());
            self.view.cached_model_items = build_model_selector_items(
                &model_catalog(),
                &self.config.recent_models,
                &filter,
                &self.config.current_provider,
                &self.config.current_model,
            )
            .into();
        }
        Arc::clone(&self.view.cached_model_items)
    }

    fn settings_items(&mut self) -> Arc<[crate::settings::SettingItem]> {
        if !self.view.cached_settings_valid {
            self.view.cached_settings_items =
                crate::update::settings_dialog::build_setting_items(self).into();
            self.view.cached_settings_valid = true;
        }
        Arc::clone(&self.view.cached_settings_items)
    }

    fn auth_providers(&mut self) -> Arc<[String]> {
        if !self.view.cached_auth_valid {
            let providers: Vec<String> = crate::auth::AuthStorage::load()
                .tokens
                .keys()
                .cloned()
                .collect();
            self.view.cached_auth_providers = providers.into();
            self.view.cached_auth_valid = true;
        }
        Arc::clone(&self.view.cached_auth_providers)
    }

    /// Record a model selection in recent history (max 5, no duplicates).
    pub fn record_model_usage(&mut self, provider: &str, model: &str) {
        let full = format!("{}/{}", provider, model);
        self.config.recent_models.retain(|m| m != &full);
        self.config.recent_models.push(full);
        if self.config.recent_models.len() > 5 {
            self.config.recent_models.remove(0);
        }
    }

    pub fn cache_generation(&self) -> u64 {
        self.view.message_gen
    }

    /// Rebuild cache only when messages changed — O(n) but gated
    pub fn ensure_fresh(&mut self) {
        if self.view.dirty && self.view.message_gen != self.view.cached_gen {
            let feed = crate::ui::LazyCache::feed(self);
            self.view.element_count = feed.elements.len();
            let line_counts: Vec<usize> = feed.elements.iter().map(|e| e.line_count()).collect();
            self.view.total_lines = line_counts.iter().sum();
            self.view.line_counts = line_counts.into();
            self.view.elements_cache = feed.elements.into();
            self.view.posts = feed.posts.into();
            self.view.cached_gen = self.view.message_gen;
        }
        // Keep the nav-mode selection valid after the feed changes.
        if let Some(sel) = self.view.selected_post {
            let max = self.view.posts.len().saturating_sub(1);
            self.view.selected_post = Some(sel.min(max));
        }
        self.view.dirty = false;
    }

    /// Visible elements slice — O(1), zero allocation
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.view.elements_cache.is_empty() {
            return &[];
        }
        let start = skip
            .min(self.view.element_count)
            .min(self.view.elements_cache.len());
        let end = (start + take)
            .min(self.view.element_count)
            .min(self.view.elements_cache.len());
        &self.view.elements_cache[start..end]
    }

    pub fn count(&self) -> usize {
        self.view.element_count.max(self.view.elements_cache.len())
    }

    pub fn element_count(&self) -> usize {
        self.view.element_count
    }

    pub fn total_lines(&self) -> usize {
        self.view.total_lines
    }

    pub fn elements_cache(&self) -> &[Element] {
        self.view.elements_cache.as_ref()
    }

    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.agent.turn_active {
            self.view.animation_frame = self.view.animation_frame.wrapping_add(1);
            self.update_speed();
            changed = true;
        }
        if self.input.input_flash > 0 {
            self.input.input_flash -= 1;
            changed = true;
        }
        if self.clear_expired_transient() {
            changed = true;
        }
        // Animate token counters toward their target values
        if self.animate_tokens() {
            changed = true;
        }
        if changed {
            self.view.dirty = true;
        }
    }

    /// Animate token display values toward their actual values.
    /// Returns true if the display values changed.
    fn animate_tokens(&mut self) -> bool {
        // Track changes in actual values
        if self.agent.tokens_in != self.agent.tokens_in_prev {
            self.agent.tokens_in_prev = self.agent.tokens_in;
        }
        if self.agent.tokens_out != self.agent.tokens_out_prev {
            self.agent.tokens_out_prev = self.agent.tokens_out;
        }
        // Ease-out interpolation: 15% of remaining per tick
        let t_in = self.agent.tokens_in as f64;
        let t_out = self.agent.tokens_out as f64;
        let d_in = t_in - self.agent.tokens_in_display;
        let d_out = t_out - self.agent.tokens_out_display;
        let c1 = if d_in.abs() < 0.5 {
            let n = self.agent.tokens_in_display.round() as usize != t_in as usize;
            if n {
                self.agent.tokens_in_display = t_in;
            }
            n
        } else {
            self.agent.tokens_in_display += d_in * 0.15;
            true
        };
        let c2 = if d_out.abs() < 0.5 {
            let n = self.agent.tokens_out_display.round() as usize != t_out as usize;
            if n {
                self.agent.tokens_out_display = t_out;
            }
            n
        } else {
            self.agent.tokens_out_display += d_out * 0.15;
            true
        };
        c1 || c2
    }

    /// Update streaming speed using rolling window of last 1000 tokens.
    /// Called every animation tick (~200ms).
    pub fn update_speed(&mut self) {
        let now = std::time::Instant::now();
        let last = self.agent.last_speed_update.get_or_insert(now);
        let elapsed = now.duration_since(*last).as_secs_f64();

        if elapsed < 0.05 {
            return; // Too soon, wait for next tick
        }

        let prev_tokens = self.agent.tokens_at_last_speed;
        let delta_tokens = self.agent.tokens_out.saturating_sub(prev_tokens);

        if delta_tokens > 0 {
            // Record new tokens in rolling window
            self.agent.speed_window.record(self.agent.tokens_out);
            self.agent.tokens_at_last_speed = self.agent.tokens_out;
            // Calculate speed from rolling window
            self.agent.speed_tps = self.agent.speed_window.speed();
            *last = now;
        } else if elapsed > 1.0 {
            // No new tokens for 1s+ — decay speed toward 0
            self.agent.speed_tps *= 0.5;
            if self.agent.speed_tps < 0.1 {
                self.agent.speed_tps = 0.0;
            }
        }
    }

    fn clear_expired_transient(&mut self) -> bool {
        if let Some(until) = self.transient_until {
            if std::time::Instant::now() > until {
                self.transient_message = None;
                self.transient_until = None;
                self.transient_level = None;
                return true;
            }
        }
        false
    }

    /// Build an immutable Snapshot for the render actor.
    /// The event loop calls this after ensure_fresh(); the render
    /// actor receives it via channel and draws without touching state.
    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            elements: Arc::clone(&self.view.elements_cache),
            line_counts: Arc::clone(&self.view.line_counts),
            total_lines: self.view.total_lines,
            input: self.input.input.clone(),
            cursor_pos: self.input.cursor_pos,
            hint_text: self.hint_text(),
            path_suggestions: self.completion.path_suggestions.clone(),
            path_selected: self.completion.path_selected,
            turn_active: self.agent.turn_active,
            input_flash: self.input.input_flash,
            vim_nav_mode: self.vim_nav_mode,
            placeholder: self.input.placeholder.clone(),
            ghost_completion: self.input.ghost_completion.clone(),
            spinner_frame: self.spinner_frame(),
            scroll: self.view.scroll,
            turn_elapsed_secs: self.turn_elapsed_secs(),
            provider: self.config.current_provider.clone(),
            model: self.config.current_model.clone(),
            theme_name: self.config.theme_name.clone(),
            thinking_level: self.config.thinking_level,
            read_only: self.config.read_only,
            queue_count: self.agent.message_queue.len() + self.agent.request_queue.len(),
            dialog: self.open_dialog.clone(),
            palette_items: self.palette_items(),
            model_selector_items: self.model_selector_items(),
            pending_edits: self.session.pending_edits.clone(),
            scoped_models: self.config.scoped_models.clone(),
            settings_items: self.settings_items(),
            session_tree_items: self.session_tree_items(),
            image_attachments: self.session.image_attachments.clone(),
            auth_providers: self.auth_providers(),
            transient_message: self.transient_message.clone(),
            transient_level: self.transient_level,
            tokens_in: self.agent.tokens_in,
            tokens_out: self.agent.tokens_out,
            speed_tps: self.agent.speed_tps,
            tokens_in_display: self.agent.tokens_in_display,
            tokens_out_display: self.agent.tokens_out_display,
            git_info: self.git_info.clone(),
            cwd_name: self.cwd_name.clone(),
            input_scroll: self.input.input_scroll,
            last_visible_height: self.view.last_visible_height,
            current_top_element: compute_current_top_element(
                &self.view.elements_cache,
                &self.view.line_counts,
                self.view.total_lines,
                self.view.scroll,
                self.view.last_visible_height,
            ),
            posts: Arc::clone(&self.view.posts),
            selected_post: self.view.selected_post,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.view.dirty
    }

    pub fn total_tokens(&self) -> usize {
        self.session
            .messages
            .iter()
            .map(|m| crate::tokens::estimate_tokens(&m.content))
            .sum()
    }

    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        let total = self.total_tokens();
        if total <= keep_recent_tokens {
            return format!("Session has {} tokens, no compaction needed", total);
        }
        let mut accumulated = 0usize;
        let mut cut_idx = 0usize;
        for (i, msg) in self.session.messages.iter().enumerate().rev() {
            accumulated += crate::tokens::estimate_tokens(&msg.content);
            if accumulated >= keep_recent_tokens {
                cut_idx = i;
                break;
            }
        }
        while cut_idx < self.session.messages.len() {
            match self.session.messages[cut_idx].role {
                Role::User | Role::Assistant => break,
                _ => cut_idx += 1,
            }
        }
        if cut_idx == 0 {
            return "Cannot compact: all messages are recent".to_string();
        }
        let removed_count = cut_idx;
        self.session.messages.drain(..cut_idx);
        let summary = format!(
            "[Compacted: {} earlier messages removed, keeping ~{} tokens]",
            removed_count, keep_recent_tokens
        );
        self.session.messages.insert(
            0,
            ChatMessage {
                role: Role::System,
                content: summary.clone(),
                timestamp: now(),
                id: "compaction".to_string(),
                ..Default::default()
            },
        );
        self.messages_changed();
        summary
    }
}
