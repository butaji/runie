//! Permission actor module.

mod actor;
pub mod messages;

mod ractor_permission;

pub use actor::PermissionActor;
pub use messages::PermissionMsg;

// Ractor-based implementation (migration in progress).
pub use ractor_permission::{RactorPermissionActor, RactorPermissionHandle};

// PermissionActorHandle now points to the ractor-based handle.
pub type PermissionActorHandle = RactorPermissionHandle;
