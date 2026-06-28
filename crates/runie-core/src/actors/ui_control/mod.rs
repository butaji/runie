//! UiControlActor — owns dialog stack, login flow, and quit state.

mod actor;
mod messages;

mod ractor_ui_control;

pub use actor::UiControlActor;
pub use messages::{UiControlActorHandle, UiControlMsg};

// Ractor-based implementation (migration in progress).
pub use ractor_ui_control::{RactorUiControlActor, RactorUiControlHandle};

// UiControlActorHandle now points to the ractor-based handle.
pub type UiControlActorHandle = RactorUiControlHandle;
