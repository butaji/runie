//! `ConfigActor` — the single owner of `~/.runie/config.toml`.

mod actor;
mod messages;
#[cfg(test)]
mod tests;

pub use actor::ConfigActor;
pub use messages::{ConfigActorHandle, ConfigMsg, ConfigReply};
