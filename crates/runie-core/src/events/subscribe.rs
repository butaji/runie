//! Subscriber registry: awaits each handler in registration order.
//!
//! Mirrors the TS README §Events rule that `subscribe()` listeners are
//! awaited in registration order, with `agent_end` acting as a settlement
//! barrier.

use std::sync::Arc;

use tokio::sync::{watch, Mutex};

use crate::{pi_event::PiAgentEvent, types::AgentEvent};

/// Opaque subscriber id returned by `SubscriberRegistry::register`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubId(pub usize);

/// A subscriber receives events and may complete asynchronously.
#[async_trait::async_trait]
pub trait Subscriber: Send + Sync + 'static {
    async fn handle(&mut self, event: &AgentEvent);

    /// Pi supplies the active abort signal to lifecycle listeners. The
    /// default keeps existing subscribers source-compatible while allowing
    /// async listeners to observe actor-owned cancellation state.
    async fn handle_with_abort(
        &mut self,
        event: &AgentEvent,
        _abort: Option<&watch::Receiver<bool>>,
    ) {
        self.handle(event).await;
    }
}

/// Pi-closed subscriber contract. Application-only events are filtered by
/// the registry adapter before this callback is invoked.
#[async_trait::async_trait]
pub trait PiSubscriber: Send + Sync + 'static {
    async fn handle_pi(&mut self, event: &PiAgentEvent);
}

#[derive(Default, Clone)]
pub struct SubscriberRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

#[derive(Default)]
struct RegistryInner {
    next_id: usize,
    subs: Vec<(SubId, SubscriberEntry)>,
}

/// Keep Pi and compatibility subscribers distinct all the way to dispatch.
/// The shared entry list still provides one deterministic registration order.
enum SubscriberEntry {
    Application(Box<dyn Subscriber>),
    Pi(Box<dyn PiSubscriber>),
}

impl SubscriberRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, sub: Box<dyn Subscriber>) -> SubId {
        let mut g = self.inner.lock().await;
        let id = SubId(g.next_id);
        g.next_id += 1;
        g.subs.push((id, SubscriberEntry::Application(sub)));
        id
    }

    pub async fn register_pi(&self, sub: Box<dyn PiSubscriber>) -> SubId {
        let mut g = self.inner.lock().await;
        let id = SubId(g.next_id);
        g.next_id += 1;
        g.subs.push((id, SubscriberEntry::Pi(sub)));
        id
    }

    pub async fn unregister(&self, id: SubId) {
        let mut g = self.inner.lock().await;
        g.subs.retain(|(sid, _)| *sid != id);
    }

    /// Dispatch `event` to every subscriber in registration order, awaiting
    /// each before starting the next. This enforces the README barrier.
    pub async fn dispatch(&self, event: &AgentEvent) {
        self.dispatch_with_abort(event, None).await;
    }

    /// Dispatch with the current actor-owned abort projection.
    pub async fn dispatch_with_abort(
        &self,
        event: &AgentEvent,
        abort: Option<&watch::Receiver<bool>>,
    ) {
        let mut g = self.inner.lock().await;
        for (_, sub) in &mut g.subs {
            if let SubscriberEntry::Application(sub) = sub {
                sub.handle_with_abort(event, abort).await;
            }
        }
    }

    /// Dispatch only the closed Pi contract, preserving registration order.
    pub async fn dispatch_pi(&self, event: &PiAgentEvent) {
        let mut g = self.inner.lock().await;
        for (_, sub) in &mut g.subs {
            if let SubscriberEntry::Pi(sub) = sub {
                sub.handle_pi(event).await;
            }
        }
    }

    pub async fn len(&self) -> usize {
        self.inner.lock().await.subs.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.subs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSub {
        idx: usize,
        order: Arc<AtomicUsize>,
    }

    struct PiCountingSub(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl PiSubscriber for PiCountingSub {
        async fn handle_pi(&mut self, event: &PiAgentEvent) {
            assert!(matches!(event, PiAgentEvent::TurnStart));
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[async_trait::async_trait]
    impl Subscriber for CountingSub {
        async fn handle(&mut self, _event: &AgentEvent) {
            self.order.fetch_add(1, Ordering::SeqCst);
            assert_eq!(self.order.load(Ordering::SeqCst), self.idx + 1);
        }
    }

    #[tokio::test]
    async fn subscribers_called_in_registration_order() {
        let reg = SubscriberRegistry::new();
        let order = Arc::new(AtomicUsize::new(0));
        for i in 0..5 {
            reg.register(Box::new(CountingSub {
                idx: i,
                order: order.clone(),
            }))
            .await;
        }
        reg.dispatch(&AgentEvent::AgentStart).await;
        assert_eq!(order.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn pi_subscribers_ignore_application_events() {
        let reg = SubscriberRegistry::new();
        let calls = Arc::new(AtomicUsize::new(0));
        reg.register_pi(Box::new(PiCountingSub(calls.clone())))
            .await;
        reg.dispatch(&AgentEvent::ThemeChanged {
            theme: crate::types::ThemeKind::GrokNight,
        })
        .await;
        reg.dispatch_pi(&PiAgentEvent::TurnStart).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    struct AbortAwareSub(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl Subscriber for AbortAwareSub {
        async fn handle(&mut self, _event: &AgentEvent) {}

        async fn handle_with_abort(
            &mut self,
            _event: &AgentEvent,
            abort: Option<&watch::Receiver<bool>>,
        ) {
            if abort.is_some_and(|signal| *signal.borrow()) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    #[tokio::test]
    async fn awaited_subscriber_observes_actor_abort_projection() {
        let reg = SubscriberRegistry::new();
        let seen = Arc::new(AtomicUsize::new(0));
        reg.register(Box::new(AbortAwareSub(seen.clone()))).await;
        let (tx, rx) = watch::channel(true);
        reg.dispatch_with_abort(&AgentEvent::AgentEnd { messages: vec![] }, Some(&rx))
            .await;
        assert_eq!(seen.load(Ordering::SeqCst), 1);
        drop(tx);
    }
}
