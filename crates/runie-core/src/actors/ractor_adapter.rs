//! Ractor adapter types: spawn helper and re-exports.
//!
//! All actors use `ractor` as the runtime. This module provides:
//! - `spawn_ractor()` — ergonomic actor spawn helper that returns `ActorRef` directly

use ractor::concurrency::JoinHandle;
use ractor::SpawnErr as RactorSpawnErr;
use ractor::{Actor, ActorRef};

#[cfg(test)]
use crate::bus::EventBus;

// Re-export ractor types for convenience
pub use ractor::{async_trait, ActorName};

// Re-exports
pub use ractor::{ActorErr, ActorProcessingErr, MessagingErr, RactorErr};

/// Spawn a ractor actor and return an `ActorRef`.
pub async fn spawn_ractor<A>(
    name: Option<ActorName>,
    actor: A,
    args: A::Arguments,
) -> Result<(ActorRef<A::Msg>, JoinHandle<()>, ractor::ActorCell), RactorSpawnErr>
where
    A: Actor,
{
    let (actor_ref, handle) = Actor::spawn(name, actor, args).await?;
    Ok((actor_ref.clone(), handle, actor_ref.get_cell()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that actors can publish events directly via EventBus.
    #[tokio::test]
    async fn event_bus_actors_publish_directly() {
        let bus = EventBus::<String>::new(16);

        let mut sub = bus.subscribe();
        bus.publish("test".to_string());

        let result = sub.recv().await.unwrap();
        assert_eq!(result, "test");
    }
}
