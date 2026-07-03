//! Permission actor module.

pub mod messages;

pub mod ractor_permission;

pub use messages::PermissionMsg;
pub use ractor_permission::{RactorPermissionActor, RactorPermissionHandle};
