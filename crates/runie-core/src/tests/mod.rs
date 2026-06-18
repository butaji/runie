#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod agent;
#[cfg(test)]
mod agent_error;
#[cfg(test)]
mod agents_manager_e2e;
#[cfg(test)]
mod appstate_structural;
#[cfg(test)]
mod autoscroll;
#[cfg(test)]
mod chat_visibility;
#[cfg(test)]
mod context_grouping;
#[cfg(test)]
mod command_forms;
#[cfg(test)]
mod compaction;
#[cfg(test)]
mod copy;
#[cfg(test)]
mod diagnostics;
#[cfg(test)]
mod dsl;
#[cfg(test)]
mod file_refs;
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
mod hints;
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
mod input_scroll;
#[cfg(test)]
mod input_undo;
#[cfg(test)]
mod input_word_nav;
#[cfg(test)]
mod login_logout;
#[cfg(test)]
mod misc;
#[cfg(test)]
mod model_cycle;
#[cfg(test)]
mod model_selector;
#[cfg(test)]
mod paste;
#[cfg(test)]
mod placeholder;
#[cfg(test)]
mod queue;
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
mod session_tree;
#[cfg(test)]
pub mod slash;
#[cfg(test)]
mod snapshot_optimization;
#[cfg(test)]
mod stack_navigation;
#[cfg(test)]
mod streaming_buffer;
#[cfg(test)]
mod subagent_cmd;
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
mod turn_complete_order;
#[cfg(test)]
mod turn_complete_visibility;
#[cfg(test)]
mod vim_mode;
#[cfg(test)]
pub(crate) mod visible_helper;
