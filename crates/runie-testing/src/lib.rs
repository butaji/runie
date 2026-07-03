//! Shared test infrastructure for Runie crates.

pub mod conditional;
pub mod env_lock;
pub mod event_helpers;
pub mod events;
pub mod keystroke_dsl;
pub mod mock_tool_skill;
pub mod time_helpers;
pub mod tool_aliases;

pub mod tests;

// Provider and agent test helpers are gated behind features because they
// depend on runie-provider and runie-agent respectively.
#[cfg(feature = "provider")]
pub mod fixtures;
#[cfg(feature = "agent")]
pub mod replay_provider;

pub use env_lock::{env_lock, with_env, EnvGuard, EnvRestore, ENV_LOCK};
pub use event_helpers::{assert_event, count_events, find_event};
pub use events::{
    ev_completed, ev_error, ev_output_text_delta, ev_response_created, llm_finish, llm_text_delta,
};
#[cfg(feature = "provider")]
pub use fixtures::openai::fixture as openai_fixture;
#[cfg(feature = "provider")]
pub use fixtures::{
    allow_all_gate, load_default_config_for_test, mock_provider, session_store_for_test, temp_home,
};
pub use keystroke_dsl::{key_event_to_dsl, parse_keystroke, parse_sequence, parse_tmux_style};
pub use mock_tool_skill::{
    mock_tool_skill, mock_tool_skill_minimax, MockToolSkill, RecordingSkill,
};
#[cfg(feature = "agent")]
pub use replay_provider::{
    capture_events, dyn_grok_replay_provider, dyn_replay_provider, dyn_replay_provider_with,
    grok_replay_from_fixtures, GrokReplayProvider, ReplayProvider,
};
pub use tests::state::{exec, fresh_state, type_str};
pub use time_helpers::{with_timeout, TestTimeGuard};
pub use tool_aliases::{
    display_name, grok_to_runie, is_read_only_tool, normalize_tool_name, runie_to_grok,
    runie_to_grok_all, transform_args,
};
