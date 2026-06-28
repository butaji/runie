//! Shared test infrastructure for Runie crates.

pub mod events;
pub mod fixtures;
pub mod mock_tool_skill;
pub mod replay_provider;
pub mod runner;
pub mod tests;
pub mod timeout;

#[macro_use]
pub mod macros;

pub use events::{
    ev_completed, ev_error, ev_output_text_delta, ev_response_created, llm_finish, llm_text_delta,
};
pub use fixtures::{
    allow_all_gate, load_default_config_for_test, mock_provider, session_store_for_test, temp_home,
};
pub use mock_tool_skill::{mock_tool_skill, mock_tool_skill_minimax, MockToolSkill, RecordingSkill};
pub use replay_provider::{capture_events, dyn_replay_provider, ReplayProvider};
pub use runner::{TestRunner, TestSubmissionId};
pub use tests::state::{exec, fresh_state, type_str};
