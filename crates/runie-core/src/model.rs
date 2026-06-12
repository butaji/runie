//! Model — Application State (mutable borrow, no cloning per event)

pub mod app_state;

pub use crate::message::{now, ChatMessage, Role};

pub(crate) const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠹', '⠸', '⠴', '⠼'];
pub(crate) const SPINNER_FRAMES: u32 = 12;

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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
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
    pub session: crate::state::SessionState,
    pub input: crate::state::InputState,
    pub agent: crate::state::AgentState,
    pub view: crate::state::ViewState,
    pub config: crate::state::ConfigState,
    pub completion: crate::state::CompletionState,

    pub streaming: bool,
    pub thinking_started_at: Option<std::time::Instant>,
    pub steering_mode: DeliveryMode,
    pub follow_up_mode: DeliveryMode,
    pub next_id: u64,
    pub intermediate_step_count: usize,
    pub animation_frame: u32,
    pub current_action: Option<String>,
    pub registry: crate::commands::CommandRegistry,
    pub should_quit: bool,
    pub open_dialog: Option<crate::commands::DialogState>,
    pub login_flow: Option<crate::login_flow::LoginFlowState>,
    pub recent_models: Vec<String>,
    pub pending_edits: Vec<crate::edit_preview::EditPreview>,
    pub skills: Vec<crate::skills::Skill>,
    pub telemetry: crate::telemetry::Telemetry,
    pub prompts: Vec<crate::prompts::PromptTemplate>,
    pub current_prompt: String,
    pub image_attachments: Vec<String>,
    pub all_collapsed: bool,
    pub(crate) last_assistant_index: Option<usize>,
    pub(crate) thought_seq: u64,
    pub(crate) input_history: Vec<String>,
    pub transient_message: Option<String>,
    pub transient_until: Option<std::time::Instant>,
    pub transient_level: Option<crate::event::TransientLevel>,
    pub git_info: Option<crate::snapshot::GitInfo>,
    pub cwd_name: String,
    cached_palette_items: Vec<(String, String, String)>,
    cached_palette_filter: Option<String>,
    cached_model_items: Vec<(String, String, String, bool, bool)>,
    cached_model_filter: Option<String>,
}

pub(crate) fn init_git_and_cwd() -> (Option<crate::snapshot::GitInfo>, String) {
    let cwd = std::env::current_dir().ok();
    let cwd_name = cwd
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let git_info = cwd.as_ref().and_then(|p| detect_git_info(p));
    (git_info, cwd_name)
}

