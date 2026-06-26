//! ViewActor — owns the authoritative `ViewState`.

mod actor;
mod messages;

pub use actor::ViewActor;
pub use messages::{ViewActorHandle, ViewMsg};
