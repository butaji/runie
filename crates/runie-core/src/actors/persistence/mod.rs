//! Persistence actor for trust decisions and input history.

mod actor;
mod messages;

pub use actor::PersistenceActor;
pub use messages::{PersistenceActorHandle, PersistenceMsg};
