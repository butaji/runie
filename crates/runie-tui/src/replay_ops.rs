//! Replay operations - parsing and applying UI ops.

pub mod parse;
pub mod apply;

pub use parse::{load_scenario, parse_scenario, parse_ui_op};
pub use apply::{apply_ui_op, parse_mode};

use super::{UiOp, ScenarioAction, Replay};
use crate::tui::TuiMode;
