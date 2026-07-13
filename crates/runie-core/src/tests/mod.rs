// All items below are individually #[cfg(test)]-gated so this module compiles
// as an empty shell in non-test builds.  The module itself is unconditional
// so that pub-re-exports are visible to runie-testing dev-dependencies.

#[cfg(test)]
mod arch_guardrails;
#[cfg(test)]
mod magic_number_lint;
#[cfg(test)]
mod support;

// Re-export shared test helpers.  Gated so this module compiles as an empty
// shell in non-test builds.
#[cfg(test)]
pub use support::{exec, fresh_state, seed_providers, tmp_store, type_str};

#[cfg(test)]
mod agent;
#[cfg(test)]
mod agent_error;
#[cfg(test)]
mod agent_streaming_tool;
#[cfg(test)]
mod appstate_structural;
#[cfg(test)]
mod autoscroll;
#[cfg(test)]
mod chat_visibility;
#[cfg(test)]
mod command_forms;
#[cfg(test)]
mod command_palette_close;
#[cfg(test)]
mod compaction;
#[cfg(test)]
mod context_grouping;
#[cfg(test)]
mod copy;
#[cfg(test)]
mod diagnostics;
#[cfg(test)]
mod dirty_flag;
#[cfg(test)]
mod dsl;
#[cfg(test)]
mod file_refs;
#[cfg(test)]
mod file_refs_lookup;
#[cfg(test)]
mod flow;
#[cfg(test)]
mod focus_events;
#[cfg(test)]
mod form_dialog;
#[cfg(test)]
mod fuzzy;
#[cfg(test)]
mod ghost_completion;
#[cfg(test)]
mod harness_skills;
#[cfg(test)]
mod hashline_edit_apply;
#[cfg(test)]
mod input_cursor;
#[cfg(test)]
mod input_flash;
#[cfg(test)]
mod input_grapheme;
#[cfg(test)]
mod input_history;
#[cfg(test)]
mod input_multiline;
#[cfg(test)]
mod input_paste;
#[cfg(test)]
mod input_chips;
#[cfg(test)]
mod input_receiver;
#[cfg(test)]
mod input_scroll;
#[cfg(test)]
mod input_undo;
#[cfg(test)]
mod input_word_nav;
#[cfg(test)]
mod list_files_render;
#[cfg(test)]
pub(crate) mod login_logout;
#[cfg(test)]
mod misc;
#[cfg(test)]
mod model_cycle;
#[cfg(test)]
mod plan_mode;
#[cfg(test)]
mod model_selector;
#[cfg(test)]
mod model_thinking;
#[cfg(test)]
mod paced_turn_completed;
#[cfg(test)]
mod paste;
#[cfg(test)]
mod placeholder;
#[cfg(test)]
mod queue;
#[cfg(test)]
mod queue_drain;
#[cfg(test)]
mod rapid_submit;
#[cfg(test)]
mod reload;
#[cfg(test)]
mod safety;
#[cfg(test)]
mod scoped_models;
#[cfg(test)]
mod session_extra;
#[cfg(test)]
mod session_store;
#[cfg(test)]
pub mod slash;
#[cfg(test)]
mod snapshot_optimization;
#[cfg(test)]
mod stack_navigation;
#[cfg(test)]
mod streaming_buffer;
#[cfg(test)]
mod theme_slash;
#[cfg(test)]
mod timer_bugs;
#[cfg(test)]
mod token_counters;
#[cfg(test)]
mod tokens;
#[cfg(test)]
mod tool_truncation;
#[cfg(test)]
mod transient;
#[cfg(test)]
mod turn_animation;
#[cfg(test)]
mod turn_complete_order;
#[cfg(test)]
mod turn_complete_visibility;
#[cfg(test)]
mod vim_mode;
#[cfg(test)]
mod vim_nav_history;
#[cfg(test)]
pub(crate) mod visible_helper;

#[cfg(test)]
mod user_message_duplicate;

#[cfg(test)]
mod no_ghost_agent;

#[cfg(test)]
mod thought_reasoning;
