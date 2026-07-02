//! Ractor-based SessionActor.

use ractor::async_trait;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::instrument;

use crate::actors::ractor_adapter::spawn_ractor;
use crate::bus::EventBus;
use crate::model::SessionState;
use crate::session::store::SessionStore;
use crate::trust::TrustManager;
use crate::Event;

use super::messages::SessionMsg;
use super::ractor_session_handle::RactorSessionHandle;
use super::session_handlers::{SessionActorState, RactorSessionActor};

impl RactorSessionActor {
    /// Spawn a `RactorSessionActor` on the given event bus.
    pub async fn spawn(
        bus: EventBus<Event>,
    ) -> Result<(RactorSessionHandle, ractor::ActorCell, tokio::task::JoinHandle<()>), ractor::SpawnErr> {
        let (handle, join, cell) = spawn_ractor(None, Self, bus).await?;
        Ok((RactorSessionHandle::new(handle), cell, join))
    }
}

#[async_trait]
impl Actor for RactorSessionActor {
    type Msg = SessionMsg;
    type State = SessionActorState;
    type Arguments = EventBus<Event>;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        bus: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // Load trust and history on startup
        let trust = tokio::task::spawn_blocking(TrustManager::load)
            .await
            .unwrap_or_default();
        let store = SessionStore::default_store()
            .unwrap_or_else(|| SessionStore::new(std::env::temp_dir().join("runie_sessions")));

        let state = SessionActorState {
            bus,
            trust: trust.clone(),
            store,
            session_state: SessionState::default(),
            next_id: 0,
        };
        state.emit(Event::TrustLoaded {
            decisions: trust.decisions(),
        });
        let entries = tokio::task::spawn_blocking(crate::input_history::load_history)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();
        state.emit(Event::HistoryLoaded { entries });
        Ok(state)
    }

    #[instrument(name = "session_actor", skip_all, fields(msg = ?msg))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        RactorSessionActor::handle_msg(state, msg).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::Receiver;

    /// Wait for an event matching a predicate with a deterministic timeout.
    async fn wait_for_event<F>(sub: &mut Receiver<Event>, pred: F) -> bool
    where
        F: Fn(&Event) -> bool,
    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            let timeout_duration = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(timeout_duration, sub.recv()).await {
                Ok(Ok(evt)) => {
                    if pred(&evt) {
                        return true;
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        false
    }

    #[tokio::test]
    async fn ractor_session_actor_spawns() {
        let bus = EventBus::<Event>::new(16);
        let result = RactorSessionActor::spawn(bus).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ractor_session_handles_trust_loaded() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (_handle, _cell, _join) = RactorSessionActor::spawn(bus).await.unwrap();

        let found = wait_for_event(&mut sub, |e| matches!(e, Event::TrustLoaded { .. })).await;
        assert!(found, "Expected TrustLoaded event");
    }

    #[tokio::test]
    async fn ractor_session_adds_user_message() {
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();
        let (handle, _cell, _join) = RactorSessionActor::spawn(bus).await.unwrap();

        handle.try_add_user_message("hello".to_string(), vec![]);

        let found = wait_for_event(&mut sub, |e| matches!(e, Event::SessionChanged { .. })).await;
        assert!(
            found,
            "Expected SessionChanged event after adding user message"
        );
    }

    #[tokio::test]
    async fn ractor_session_resume_most_recent_emits_session_operation_failed() {
        // When no sessions exist, handle_resume_most_recent emits SessionOperationFailed.
        let bus = EventBus::<Event>::new(16);
        let mut sub = bus.subscribe();

        // Directly call the handler (same logic as ResumeMostRecent message).
        // In a fresh temp-store actor, no sessions exist → expect failure event.
        use crate::session::store::SessionStore;
        use crate::actors::session::session_handlers::RactorSessionActor;
        let store = SessionStore::new(std::env::temp_dir().join("runie_test_resume_nonexistent"));
        let state = &mut crate::actors::session::session_handlers::SessionActorState {
            bus: bus.clone(),
            trust: crate::trust::TrustManager::default(),
            store,
            session_state: crate::model::SessionState::default(),
            next_id: 0,
        };
        RactorSessionActor::handle_resume_most_recent(state).await;

        let found = wait_for_event(&mut sub, |e| {
            matches!(
                e,
                Event::SessionOperationFailed {
                    operation,
                    error: _
                } if operation == "resume"
            )
        }).await;
        assert!(
            found,
            "Expected SessionOperationFailed(resume) when no sessions exist"
        );
    }
}
