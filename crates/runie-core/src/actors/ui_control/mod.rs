//! UiControlActor — owns dialog stack, login flow, and quit state.

mod actor;
mod messages;

pub use actor::UiControlActor;
pub use messages::{UiControlActorHandle, UiControlMsg};
