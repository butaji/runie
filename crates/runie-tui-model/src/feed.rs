//! Renderer-independent transcript line vocabulary and reducer intents.
use std::collections::{HashMap, HashSet};

use runie_core::types::{ThemeKind, ToolDisplayMode};
#[path = "feed_utils.rs"]
mod feed_utils;
pub use feed_utils::*;

include!("feed_types_core.rs");
include!("feed_tool_lookup.rs");
include!("feed_types_tools.rs");
include!("feed_snapshot.rs");
include!("feed_types_tools_tail.rs");
include!("feed_messages.rs");
include!("feed_message_domain.rs");
include!("feed_state.rs");
include!("feed_snapshot_state.rs");
include!("feed_reducers.rs");
include!("feed_workflow.rs");
include!("feed_navigation.rs");
include!("feed_tool_display.rs");
include!("feed_selection.rs");
include!("feed_line_ops.rs");
include!("feed_reducer_boundary.rs");
include!("feed_activity.rs");
include!("feed_assistant.rs");
include!("feed_view_state.rs");
include!("feed_state_27.rs");

include!("feed_tool_lifecycle.rs");
include!("feed_state_tool_rows.rs");

#[cfg(test)]
#[path = "feed_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "feed_tests_extra.rs"]
mod tests_extra;
#[cfg(test)]
#[path = "feed_tests_extra2.rs"]
mod tests_extra2;
#[cfg(test)]
#[path = "feed_tests_more.rs"]
mod tests_more;
#[cfg(test)]
#[path = "feed_tests_tail.rs"]
mod tests_tail;
#[cfg(test)]
#[path = "feed_tests_tail2.rs"]
mod tests_tail2;
