//! Safety and permission commands.

use crate::commands::CommandResult;
use crate::model::AppState;

crate::handlers! {
    registry,
    "readonly" => |_: &mut AppState, _: &str| CommandResult::Event(crate::Event::ToggleReadOnly),
    "trust" => |_: &mut AppState, _: &str| CommandResult::Event(crate::Event::TrustProject),
    "untrust" => |_: &mut AppState, _: &str| CommandResult::Event(crate::Event::UntrustProject),
}
