#![warn(clippy::all)]

//! Runie Core — State, Events, Update, UI Architecture
//!
//! Architecture (three layers):
//!   model     :: AppState, ChatMessage (source of truth)
//!   event     :: Event enum (all possible state transitions)
//!   update    :: State transitions (pure functions)
//!   ui        :: Elements, Transform (view layer)
//!   labels    :: Static text constants

pub mod auth;
pub mod clipboard_image;
pub mod commands;
pub mod config_migrate;
pub mod config_reload;
pub mod dialog;
#[cfg(test)]
pub mod dsl;
pub mod edit_preview;
pub mod event;
pub mod file_refs;
pub mod fuzzy;
pub mod input_history;
pub mod keybindings;
pub mod labels;
pub mod login_config;
pub mod login_flow;
pub mod message;
pub mod model;
pub mod model_catalog;
pub mod model_scroll;
pub mod notification;
pub mod path_complete;
pub mod prompts;
pub mod provider;
pub mod provider_registry;
pub mod providers_dialog;
pub mod scoped_model;
pub mod session;
pub mod session_tree;
pub mod settings;
pub mod skills;
pub mod snapshot;
pub mod state;
pub mod telemetry;
pub mod themes;
pub mod tokens;
pub mod tool_markers;
pub mod trust;
pub mod ui;
pub mod update;

#[cfg(test)]
mod file_refs_lookup_tests;
#[cfg(test)]
mod tests;

pub use auth::{AuthStorage, AuthToken};
pub use clipboard_image::read_clipboard_image;
pub use edit_preview::EditPreview;
pub use event::Event;
pub use file_refs::{find_files, is_image_file, read_file_ref, FileRef};
pub use input_history::{filter_history, load_history, save_history, search_history};
pub use keybindings::{
    default_keybindings, event_from_name, load_keybindings, parse_keybindings_json,
};
pub use labels::{format_timestamp, thinking_with_time, thought_with_time, THINKING_LOADING};
pub use login_config::{
    config_path as login_config_path, list_configured_providers, remove_provider_config,
    save_provider_config,
};
pub use login_flow::{
    build_key_input, build_login_root, build_model_selector, build_provider_picker, LoginFlowState,
    LoginStep,
};
pub use model::{now, AppState, ChatMessage, Role};
pub use prompts::{
    build_system_prompt, load_prompts, PromptSource, PromptTemplate, DEFAULT_PROMPT,
};
pub use provider::{Message, Provider, ProviderError, ResponseChunk};
pub use provider_registry::{
    display_name, find_provider, find_provider_by_env_var, is_known_provider, known_providers,
    ProviderApiType, ProviderMeta,
};
pub use session::{delete, format_as_markdown, list, load, save, Session};
pub use session_tree::{SessionTree, SessionTreeFilter, TreeNode};
pub use skills::{build_skills_context, load_all, load_from_dir, Skill};
pub use snapshot::{GitInfo, Snapshot};
pub use telemetry::Telemetry;
pub use tokens::{estimate_tokens, TokenTracker};
pub use trust::{TrustDecision, TrustManager};
pub use ui::{Element, Feed, LazyCache};
