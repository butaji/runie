#![warn(clippy::all)]

//! Runie Core — State, Events, Update, View Architecture
//!
//! Architecture (three layers):
//!   model     :: AppState, ChatMessage (source of truth)
//!   event     :: Event enum (all possible state transitions)
//!   update    :: State transitions (pure functions)
//!   view      :: Elements, Transform (view layer — domain projection)
//!   labels    :: Static text constants

extern crate self as runie_core;

pub mod actors;
pub use actors::{
    FffFileItem, FffSearchRequest, FffSearchResult, FffSearchResultPayload, FffSearchState,
    PersistenceActor,
};
// Inner state structs are pub(crate) — accessible within runie-core but not exported externally.
// AppState itself remains pub so it can be used in public DSL signatures.
pub use message::{ChatMessage, Role};
pub use model::{
    AgentState, AppState, CompletionState, ConfigState, DeliveryMode, InputState,
    PermissionRequestState, ScopedModel, SessionState, ThinkingLevel, ViewState,
};
pub mod agent_phase;
pub mod auth;
pub mod bash_safety;
pub mod bus;
pub mod commands;
pub mod config;
pub mod declarative;
pub mod dialog;
pub mod diff;
pub mod resource_loader;
/// Display-cell width helpers for terminal layout.
pub mod display_width;
pub mod dry_run;
pub mod dsl;
pub mod edit_preview;
pub mod error;
pub mod event;
pub mod file_refs;
pub mod headless_runtime;
pub mod hooks;
pub mod input_history;
/// I/O utilities (atomic writes, file locking).
pub mod io;
pub mod keybindings;
// NOTE: mcp module was deleted, keeping reference commented for now
// pub mod mcp;
pub mod mcp;
pub use mcp::{McpConnectionManager, McpTool, SchemaCache};
/// Static labels and text constants.
pub mod labels;
pub mod layout;
pub mod location;
pub mod login_flow;
pub mod markdown;
pub mod message;
/// Wire-protocol types (JSON-RPC envelope, submission-queue types).
pub mod proto;
pub mod provider_event;
pub use message::Part;
pub mod harness_skills;
pub mod model;
pub mod model_catalog;
pub mod notification;
pub mod path;
pub mod path_complete;
pub mod prompts;
pub mod provider;
pub mod sanitize;
pub mod scoped_model;
pub mod session;
pub mod settings;
pub mod shell;
pub mod skills;
pub mod snapshot;
// state types moved to model::state
pub mod streaming_buffer;
pub mod subagents;
/// Tracing subscriber initialization.
pub mod tracing_init;
/// Metrics facade for telemetry (counters, histograms, gauges).
pub mod metrics;

/// Centralized user-facing strings (errors, warnings, info, help).
pub mod ui_strings;

pub mod theme_tokens;

pub mod tokens;
pub mod tool;
pub mod tool_markers;
pub mod tool_stream;
pub use tool::{format_bytes, format_duration};
pub mod permissions;
pub mod trust;
pub mod update;
pub mod view;

// The tests module is unconditionally declared so that its pub-re-exports
// are visible to runie-testing dev-dependencies even in non-test builds.
// Individual items inside tests/ remain #[cfg(test)]-gated.
#[allow(unused)]
mod tests;

/// Canonical test helpers for `AppState` manipulation, re-exported from
/// `tests/support.rs` so that `runie-testing` can import the same helpers.
///
/// Internal helpers (`ENV_LOCK`, `seed_providers`, `tmp_store`, `minimal_session`)
/// stay in `tests/support.rs` and are NOT re-exported.
///
/// The module is unconditional so it compiles when `runie-core` is a dev-dependency
/// of `runie-testing` (non-test build).  The actual helpers live in
/// `tests/support.rs` which is gated `#[cfg(test)]`, so this module's re-exports
/// are only non-empty in test builds.
#[allow(unused)]
pub mod tests_support {
    // Re-export from the pub-re-exports in tests/mod.rs.
    #[cfg(test)]
    pub use crate::tests::exec;
    #[cfg(test)]
    pub use crate::tests::fresh_state;
    #[cfg(test)]
    pub use crate::tests::type_str;
}

pub use actors::session::RactorSessionActor;
pub use agent_phase::{elapsed_secs, format_elapsed, AgentPhase};
pub use auth::{AuthStorage, AuthToken};
pub use config::{Config, ConfigChange, ModelProvider, ModelsSection, TruncationSection};
pub use declarative::{CommandDef, DeclarativeLoader, SkillDef, Trigger};
pub use diff::{Diff, DiffHunk, DiffLine};
pub use dry_run::{run_dry_run, DryRunReport, DryRunStatus};
pub use edit_preview::EditPreview;
// NOTE: RunieError/RunieErrorKind were deleted — see crates/runie-core/src/error.rs note.
pub use event::Event;
pub use file_refs::{find_files, is_image_file, read_file_ref, FileRef};
pub use harness_skills::{
    HarnessConfig, HarnessSkill, HashlineEdit, HashlineEditConfig, HashlineEditSkill,
    SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult, TurnEndCtx, TurnEndResult,
    TurnStartCtx, TurnStartResult, VerificationConfig, VerificationLoopSkill,
};
pub use input_history::{filter_history, load_history, save_history, search_history};
pub use keybindings::{
    default_keybindings, event_from_name, load_keybindings, merged_keybindings,
    parse_keybindings_json,
};
pub use login_flow::{
    build_key_input, build_login_root, build_model_selector, build_provider_picker, LoginFlowState,
    LoginStep,
};
pub use model_catalog::{filter_models, model_catalog, ModelCapabilities, ModelInfo};
pub use permissions::{
    is_read_only_tool, is_sensitive_path, ApprovalSink, AutoAllowSink, PermissionAction,
    PermissionGate, PermissionRule, PermissionSet, ScriptedSink, TuiApprovalSink,
};
pub use prompts::{
    build_system_prompt, load_prompts, PromptSource, PromptTemplate, DEFAULT_PROMPT, DEFAULT_TOOLS,
};
pub use provider::{
    display_name, find_model, find_provider, find_provider_by_env_var, is_known_provider,
    known_providers, Provider, ProviderError, ProviderMetadata, ProviderMeta, RetryConfig,
    ResponseChunk,
};
pub use provider_event::{ModelError, ProviderEvent, StopReason};
pub use resource_loader::{
    derive_name_from_path, extract_frontmatter, extract_section, is_user_invocable,
    load_resources_from_dir, parse_resource_md, resolve_name,
};
pub use session::SessionMetadata;
pub use session::store::SessionStore;
pub use session::tree::{SessionTree, SessionTreeFilter, TreeNodeData};
pub use session::{format_as_markdown, Session};
pub use skills::{build_skills_context, load_all, load_from_dir, Skill};
pub use snapshot::{GitInfo, Snapshot};

pub use tokens::{
    estimate_tokens, estimate_tokens_for_model, estimate_tokens_with_tokenizer, token_tracker_for,
    TokenTracker,
};
pub use trust::{TrustDecision, TrustManager};
pub use view::{Element, Feed, LazyCache};
