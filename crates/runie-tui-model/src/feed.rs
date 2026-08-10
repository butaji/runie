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
include!("feed_state_00.rs");
include!("feed_state_01.rs");
include!("feed_state_02.rs");
include!("feed_state_03.rs");
include!("feed_state_04.rs");
include!("feed_state_05.rs");
include!("feed_state_06.rs");
include!("feed_state_07.rs");
include!("feed_state_08.rs");
include!("feed_state_09.rs");
include!("feed_state_10.rs");
include!("feed_state_11.rs");
include!("feed_state_12.rs");
include!("feed_state_13.rs");
include!("feed_state_14.rs");
include!("feed_state_15.rs");
include!("feed_state_16.rs");
include!("feed_state_17.rs");
include!("feed_state_18.rs");
include!("feed_state_19.rs");
include!("feed_state_20.rs");
include!("feed_state_21.rs");
include!("feed_state_22.rs");
include!("feed_state_23.rs");
include!("feed_state_24.rs");
include!("feed_state_25.rs");
include!("feed_state_26.rs");
include!("feed_state_27.rs");
include!("feed_state_28.rs");
include!("feed_state_29.rs");
include!("feed_state_30.rs");
include!("feed_state_31.rs");
include!("feed_state_32.rs");
include!("feed_state_33.rs");
include!("feed_state_34.rs");
include!("feed_state_35.rs");
include!("feed_state_36.rs");
include!("feed_state_37.rs");
include!("feed_state_38.rs");
include!("feed_state_39.rs");
include!("feed_state_40.rs");
include!("feed_state_41.rs");
include!("feed_state_42.rs");
include!("feed_state_43.rs");
include!("feed_state_44.rs");
include!("feed_state_45.rs");
include!("feed_state_46.rs");
include!("feed_state_47.rs");
include!("feed_state_48.rs");
include!("feed_state_49.rs");
include!("feed_state_50.rs");
include!("feed_state_51.rs");
include!("feed_state_52.rs");
include!("feed_state_53.rs");

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
