//! Permission actor module.

mod actor;
pub mod messages;

pub use actor::PermissionActor;
pub use messages::{PermissionActorHandle, PermissionMsg};
