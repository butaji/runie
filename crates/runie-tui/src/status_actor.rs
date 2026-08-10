//! Actor-owned status projection.

use runie_core::types::AgentEvent;
use runie_core::{declare_reducer_actor, spawn_owned_worker, task_owner::TaskOwner};
use tokio::sync::watch;

use crate::widgets::{Status, StatusBar, StatusMsg, StatusSnapshot};
use runie_tui_model::project_event;

declare_reducer_actor!(StatusReducer, StatusSnapshot, StatusMsg);

/// Handle to the single owner of the status projection.
#[derive(Clone)]
pub struct StatusActor {
    reducer: StatusReducer,
    _bus_owner: Option<std::sync::Arc<TaskOwner>>,
}

impl StatusActor {
    pub fn new() -> Self {
        let initial = StatusSnapshot {
            elapsed_ticks: crate::clock::parity_elapsed_ticks().unwrap_or_default(),
            ..StatusSnapshot::default()
        };
        let elapsed_seed = crate::clock::parity_elapsed_ticks();
        let reducer = StatusReducer::new(16, initial, move |state, message| {
            state.apply(message, elapsed_seed)
        });
        Self {
            reducer,
            _bus_owner: None,
        }
    }

    /// Construct a live status projection that owns its event-bus subscription.
    /// The renderer remains a pure event consumer and no longer mutates this
    /// actor as a side effect of drawing the feed.
    pub fn new_with_bus(bus: &runie_core::events::EventBus) -> Self {
        let mut actor = Self::new();
        let mut events = bus.subscribe();
        let reducer = actor.reducer.clone();
        actor._bus_owner = Some(spawn_owned_worker!(async move {
            loop {
                let event = match events.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        let messages = vec![StatusMsg::Set(Status::Error(format!(
                            "event stream lagged ({count} events)",
                        )))];
                        for message in messages {
                            if !reducer.apply(message).await {
                                return;
                            }
                        }
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let messages = project_event(&event).status;
                for message in messages {
                    if !reducer.apply(message).await {
                        return;
                    }
                }
            }
        }));
        actor
    }

    pub async fn apply(&self, message: StatusMsg) {
        let _ = self.reducer.apply(message).await;
    }

    /// Apply all status-owned transitions represented by one core event.
    /// Unknown events are intentionally a no-op for this projection.
    pub async fn apply_event(&self, event: &AgentEvent) {
        let messages = project_event(event).status;
        for message in messages {
            let _ = self.reducer.apply(message).await;
        }
    }

    pub fn snapshot(&self) -> StatusBar {
        StatusBar::from_model_snapshot(self.reducer.snapshot())
    }

    pub fn model_snapshot(&self) -> StatusSnapshot {
        self.reducer.snapshot()
    }

    pub fn shared_model_snapshot(&self) -> runie_core::SharedSnapshot<StatusSnapshot> {
        self.reducer.shared_snapshot()
    }

    pub fn shared_subscribe(&self) -> watch::Receiver<runie_core::SharedSnapshot<StatusSnapshot>> {
        self.reducer.shared_subscribe()
    }

    pub fn subscribe(&self) -> watch::Receiver<StatusSnapshot> {
        self.reducer.subscribe()
    }
}

impl Default for StatusActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::StatusActor;
    use crate::widgets::{Status, StatusMsg};
    use runie_core::types::AgentEvent;

    #[tokio::test]
    async fn actor_publishes_acknowledged_reducer_snapshot() {
        let actor = StatusActor::new();
        actor.apply(StatusMsg::Set(Status::Thinking)).await;
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
        let model = actor.model_snapshot();
        assert_eq!(model.state, Status::Thinking);
        assert_eq!(model.theme, runie_core::types::ThemeKind::GrokNight);
        assert!(model.turn_usage.is_none());
    }

    #[tokio::test]
    async fn actor_applies_status_owned_core_event() {
        let actor = StatusActor::new();
        let mut updates = actor.subscribe();
        actor.apply_event(&AgentEvent::TurnStart).await;
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
        assert!(updates.has_changed().expect("actor is alive"));
        updates.borrow_and_update();
        assert!(!updates.has_changed().expect("actor is alive"));
    }

    #[tokio::test]
    async fn actor_projects_agent_start_as_thinking() {
        let actor = StatusActor::new();
        actor.apply_event(&AgentEvent::AgentStart).await;
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
    }

    #[tokio::test]
    async fn bus_owned_actor_reduces_status_events_without_renderer_dispatch() {
        let bus = runie_core::events::EventBus::new();
        let actor = StatusActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();
        bus.publish(AgentEvent::AgentStart);
        snapshot.changed().await.expect("status bus projection");
        assert_eq!(actor.snapshot().current(), &Status::Thinking);
    }

    #[tokio::test]
    async fn bus_owned_actor_surfaces_lag_as_error_status() {
        use runie_core::events::bus::{EventBus, BUS_CAPACITY};

        let bus = EventBus::new();
        // The keepalive subscriber keeps at least one receiver attached so
        // `publish` cannot drop events with no receivers and forces the
        // actor's subscription to lag behind the broadcast tail.
        let _keepalive = bus.subscribe();
        let actor = StatusActor::new_with_bus(&bus);
        let mut snapshot = actor.subscribe();

        // Publish enough events to overflow the broadcast ring buffer so the
        // actor's next `recv` observes `RecvError::Lagged`. Each event maps
        // to an empty `StatusMsg` batch, so the only state transition the
        // actor will publish is the explicit `Status::Error` from the lag.
        for _ in 0..BUS_CAPACITY + 1 {
            bus.publish(AgentEvent::ActiveToolsChanged { tool_names: vec![] });
        }

        // The bus bridge forwards the lag as an explicit `Status::Error`
        // before draining any post-lag tail event, so the first snapshot
        // change after `publish` is the lag diagnostic.
        snapshot.changed().await.expect("status actor alive");
        match &actor.model_snapshot().state {
            Status::Error(text) => assert!(
                text.contains("event stream lagged"),
                "expected lag diagnostic, got {text:?}"
            ),
            other => panic!("expected Status::Error from lag, got {other:?}"),
        }
    }
}
