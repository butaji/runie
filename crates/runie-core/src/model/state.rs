//! Core application state types and simple accessors.


use crate::state::CommandUsage;
use crate::ui::elements::Element;

/// A file entry from the FFF indexer for the file picker.
#[derive(Clone, Debug)]
pub struct FffFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub score: f64,
    pub git_status: Option<String>,
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
    /// Sidebar state (Team mode subagent panel).
    pub sidebar: crate::state::SidebarState,
    /// Current orchestrator state (Team mode) for status bar display.
    pub orchestrator_state: crate::orchestrator_actor::OrchestratorState,

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
    /// FFF search results for the current file picker query.
    /// Set when FFF indexer returns results (populated asynchronously).
    pub fff_file_results: Vec<FffFileEntry>,
    /// Counter incremented each time the user types in the file picker.
    /// Used to detect stale FFF results (result counter != current counter means ignore).
    pub fff_debounce: u32,
    pub pending_agent_edit: Option<crate::agent_profiles::AgentProfile>,
    /// Multi-agent registry for Team mode.
    pub multi_agent: crate::multi_agent::AgentRegistry,
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
            sidebar: crate::state::SidebarState::default(),
            orchestrator_state: crate::orchestrator_actor::OrchestratorState::default(),
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
            fff_file_results: Vec::new(),
            fff_debounce: 0,
            pending_agent_edit: None,
            multi_agent: crate::multi_agent::AgentRegistry::default(),
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
        const SPINNER_CHARS: &[char] =
            &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
        const SPINNER_FRAMES: u32 = 12;
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
        self.session.session_updated_at = crate::message::now();
        self.view.dirty = true;
    }

    /// Record the height of the message viewport. Called by the render
    /// actor on each draw. Used by vim nav mode for element-level jumps.
    pub fn set_last_visible_height(&mut self, height: u16) {
        self.view.last_visible_height = height;
    }

    /// Record the width of the message content area. Called by the render
    /// actor on each draw. Used to keep core scroll math consistent with
    /// the actual wrapped Ratatui output.
    pub fn set_last_content_width(&mut self, width: u16) {
        self.view.last_content_width = width;
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

    pub fn is_dirty(&self) -> bool {
        self.view.dirty
    }

    /// Record that a command was invoked for palette ranking.
    pub fn record_command_usage(&mut self, name: &str) {
        let now = crate::update::now();
        let entry = self.config.command_usage.entry(name.to_string()).or_insert_with(|| CommandUsage {
            count: 0,
            last_used: now,
        });
        entry.count += 1;
        entry.last_used = now;
    }

    /// Rank commands by fuzzy match score, recency boost, and usage count.
    /// Returns commands in ranked order, limited to `limit`.
    pub fn rank_commands(&self, query: &str, limit: usize) -> Vec<(&crate::commands::CommandDef, i32)> {
        let all: Vec<_> = self.registry.list();
        if query.is_empty() {
            // No query: sort by usage + recency (most recently used first), then category/name
            let mut ranked: Vec<_> = all
                .iter()
                .map(|cmd| {
                    let usage = self.config.command_usage.get(&cmd.name);
                    let score = compute_ranking_score(query, cmd, usage);
                    (cmd, score)
                })
                .collect();
            ranked.sort_by_key(|(cmd, score)| {
                (std::cmp::Reverse(*score), &cmd.category, &cmd.name)
            });
            return ranked.into_iter().take(limit).map(|(&cmd, s)| (cmd, s)).collect();
        }

        let mut ranked: Vec<_> = all
            .iter()
            .filter_map(|cmd| {
                let base = crate::fuzzy::fuzzy_match(query, &cmd.name)
                    .or_else(|| crate::fuzzy::fuzzy_match(query, &cmd.desc))?;
                let usage = self.config.command_usage.get(&cmd.name);
                let score = compute_ranking_score(query, cmd, usage) + base * 100;
                Some((cmd, score))
            })
            .collect();
        ranked.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        ranked.into_iter().take(limit).map(|(&cmd, s)| (cmd, s)).collect()
    }

    /// Extract plain text from the currently selected post for `y` (copy).
    /// Returns None if no post is selected or if the selection is empty.
    pub fn copy_selected_post_text(&self) -> Option<String> {
        let post_idx = self.view.selected_post?;
        let post = self.view.posts.get(post_idx)?;
        let elements = &self.view.elements_cache;
        let mut lines = Vec::new();
        for i in post.start..post.end {
            if let Some(elem) = elements.get(i) {
                if let Some(text) = element_text(elem) {
                    lines.push(text);
                }
            }
        }
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    /// Extract metadata from the currently selected post for `Y` (copy metadata).
    pub fn copy_selected_post_metadata(&self) -> Option<String> {
        let post_idx = self.view.selected_post?;
        let post = self.view.posts.get(post_idx)?;
        let elements = &self.view.elements_cache;
        let mut parts = Vec::new();
        for i in post.start..post.end.min(elements.len()) {
            if let Some(elem) = elements.get(i) {
                if let Some(meta) = element_metadata(elem) {
                    parts.push(meta);
                }
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
    }

    /// Restore application state from a JSON session snapshot.
    pub fn restore_session(&mut self, session: &crate::session::Session) {
        self.session.messages = session.messages.clone();
        self.config.current_provider = session.provider.clone();
        self.config.current_model = session.model.clone();
        self.config.theme_name = session.theme_name.clone();
        self.config.thinking_level = session.thinking_level;
        self.config.read_only = session.read_only;
        self.session.session_display_name =
            session.display_name.clone().or(Some(session.name.clone()));
        self.session.session_created_at = session.created_at;
        self.session.session_updated_at = session.updated_at;
        self.session.session_tree = session.session_tree.clone();
        self.configure_token_tracker();
        self.messages_changed();
    }
}

/// Compute a ranking score boost from usage count and recency.
/// Score is scaled so it doesn't dominate the fuzzy score.
fn compute_ranking_score(
    _query: &str,
    _cmd: &crate::commands::CommandDef,
    usage: Option<&CommandUsage>,
) -> i32 {
    let usage_boost = usage.map(|u| u.count as i32).unwrap_or(0);
    // Recency: commands used in the last 5 minutes get a bonus that decays.
    // Stored as Unix timestamp; compare against current time.
    let now = crate::update::now();
    let recency_boost = usage.map(|u| {
        let age = now - u.last_used;
        if age < 300.0 {
            // Exponential decay: max bonus at t=0, zero at t=300s
            ((300.0 - age) / 300.0 * 10.0) as i32
        } else {
            0
        }
    }).unwrap_or(0);

    // Usage boosts are small (1–10), recency boosts also small.
    // This is added to fuzzy score * 100, so it only breaks ties.
    usage_boost + recency_boost
}

// ── Post copy helpers ───────────────────────────────────────────────────────────

/// Extract plain text from an Element for `y` copy.
fn element_text(elem: &Element) -> Option<String> {
    match elem {
        Element::UserMessage { content, .. } => Some(content.clone()),
        Element::AgentMessage { content, .. } => Some(content.clone()),
        Element::ThoughtSummary { content, .. } => Some(content.clone()),
        Element::ThoughtMarker { content, .. } => Some(content.clone()),
        Element::ToolRunning { name, args, .. } => {
            if args.is_empty() {
                Some(name.clone())
            } else {
                Some(format!("{} {}", name, args))
            }
        }
        Element::ToolDone { name, args, output, .. } => {
            let head = if args.is_empty() {
                name.clone()
            } else {
                format!("{} {}", name, args)
            };
            if output.is_empty() {
                Some(head)
            } else {
                Some(format!("{} {}\n{}", head, output,
                    if output.ends_with('\n') { "" } else { "\n" }))
            }
        }
        _ => None,
    }
}

/// Extract short metadata string from an Element for `Y` (copy metadata).
fn element_metadata(elem: &Element) -> Option<String> {
    match elem {
        Element::UserMessage { timestamp, .. } => {
            Some(format!("user {:.0}s", timestamp))
        }
        Element::AgentMessage { provider, timestamp, .. } => {
            Some(format!("{} {:.0}s", provider, timestamp))
        }
        Element::Thinking { timestamp, .. } => {
            Some(format!("thinking {:.0}s", timestamp))
        }
        Element::ThoughtSummary { duration_secs, timestamp, .. } => {
            Some(format!("thought {:.0}s → {:.1}s", timestamp, duration_secs))
        }
        Element::ToolRunning { name, timestamp, .. } => {
            Some(format!("{} running at {:.0}s", name, timestamp))
        }
        Element::ToolDone { name, duration_secs, timestamp, .. } => {
            Some(format!("{} done in {:.1}s at {:.0}s", name, duration_secs, timestamp))
        }
        Element::ToolSummary { name, duration_secs, timestamp, .. } => {
            Some(format!("{} {:.1}s at {:.0}s", name, duration_secs, timestamp))
        }
        Element::TurnComplete { duration_secs, timestamp, .. } => {
            Some(format!("turn {:.1}s at {:.0}s", duration_secs, timestamp))
        }
        _ => None,
    }
}
