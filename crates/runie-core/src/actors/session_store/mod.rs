//! `SessionStoreActor` — owns named-session IO.

pub mod actor;
pub mod messages;

pub use actor::SessionStoreActor;
pub use messages::{SessionStoreActorHandle, SessionStoreMsg};
