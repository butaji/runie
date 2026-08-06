//! Event bus. Wraps `tokio::sync::broadcast` so subscribers can attach and
//! detach independently.

use tokio::sync::broadcast;

use crate::{pi_event::PiAgentEvent, types::AgentEvent};

/// Per-topic broadcast capacity. Sized to absorb a full agent run's worth of
/// events without dropping for slow subscribers.
pub const BUS_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BUS_CAPACITY);
        Self { tx }
    }

    pub fn publish(&self, event: AgentEvent) {
        // Broadcast `send` returns Err only if there are no receivers; we
        // deliberately ignore it (the subscriber is optional).
        let _ = self.tx.send(event);
    }

    /// Publish only the closed Pi-core event contract.
    pub fn publish_pi(&self, event: PiAgentEvent) {
        self.publish(event.try_into_agent_event());
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.tx.subscribe()
    }

    pub fn subscribe_pi(&self) -> PiEventReceiver {
        PiEventReceiver {
            inner: self.subscribe(),
        }
    }
}

pub struct PiEventReceiver {
    inner: broadcast::Receiver<AgentEvent>,
}

impl PiEventReceiver {
    pub async fn recv(&mut self) -> Result<PiAgentEvent, broadcast::error::RecvError> {
        loop {
            let event = self.inner.recv().await?;
            if let Ok(pi_event) = PiAgentEvent::try_from(event) {
                return Ok(pi_event);
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscriber_receives_published_events() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish(AgentEvent::AgentStart);
        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AgentEvent::AgentStart));
    }

    #[tokio::test]
    async fn publish_with_no_receivers_is_ok() {
        let bus = EventBus::new();
        // No subscribers; should not panic.
        bus.publish(AgentEvent::AgentStart);
    }

    #[tokio::test]
    async fn typed_pi_subscription_filters_application_events() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe_pi();
        bus.publish(AgentEvent::ThemeChanged {
            theme: crate::types::ThemeKind::GrokNight,
        });
        bus.publish_pi(PiAgentEvent::TurnStart);
        assert!(matches!(rx.recv().await.unwrap(), PiAgentEvent::TurnStart));
    }
}
