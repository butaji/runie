//! Subscriber registry: awaits each handler in registration order.
//!
//! Mirrors the TS README §Events rule that `subscribe()` listeners are
//! awaited in registration order, with `agent_end` acting as a settlement
//! barrier.

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::types::AgentEvent;

/// Opaque subscriber id returned by `SubscriberRegistry::register`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubId(pub usize);

/// A subscriber receives events and may complete asynchronously.
#[async_trait::async_trait]
pub trait Subscriber: Send + Sync + 'static {
    async fn handle(&mut self, event: &AgentEvent);
}

/// Type-erased subscriber handle.
type BoxedSubscriber = Box<dyn Subscriber>;

#[derive(Default, Clone)]
pub struct SubscriberRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

#[derive(Default)]
struct RegistryInner {
    next_id: usize,
    subs: Vec<(SubId, BoxedSubscriber)>,
}

impl SubscriberRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, sub: BoxedSubscriber) -> SubId {
        let mut g = self.inner.lock().await;
        let id = SubId(g.next_id);
        g.next_id += 1;
        g.subs.push((id, sub));
        id
    }

    pub async fn unregister(&self, id: SubId) {
        let mut g = self.inner.lock().await;
        g.subs.retain(|(sid, _)| *sid != id);
    }

    /// Dispatch `event` to every subscriber in registration order, awaiting
    /// each before starting the next. This enforces the README barrier.
    pub async fn dispatch(&self, event: &AgentEvent) {
        let mut g = self.inner.lock().await;
        for (_, sub) in &mut g.subs {
            sub.handle(event).await;
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
}
