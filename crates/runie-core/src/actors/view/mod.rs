//! ViewActor — owns the authoritative `ViewState`.

mod actor;
mod messages;
mod ractor_view;

pub use actor::ViewActor;
pub use messages::{ViewActorHandle, ViewMsg};

// Ractor-based implementation (migration in progress).
pub use ractor_view::{RactorViewActor, RactorViewHandle};
