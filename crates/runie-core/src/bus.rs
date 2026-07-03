//! Typed event bus using tokio's broadcast channel.
//!
//! Provides a publish-subscribe bus. Late subscribers are served by
//! SessionActor disk-replay on startup; no in-memory ring buffer is needed.

use tokio::sync::broadcast;

/// Typed event bus for actor communication.
///
/// Uses tokio's broadcast channel internally to allow multiple subscribers.
///
/// # Type Parameter
/// - `E`: The event type (must be Send + Clone + 'static)
///
/// # Example
/// ```ignore
/// let bus = EventBus::<MyEvent>::new(100);
///
/// // Subscriber (misses events before subscription)
/// let mut sub = bus.subscribe();
///
/// // Publisher
/// bus.publish(MyEvent::Start);
/// bus.publish(MyEvent::Done);
///
/// // sub receives: MyEvent::Start, MyEvent::Done
/// ```
#[derive(Debug, Clone)]
pub struct EventBus<E: Send + Clone + 'static> {
    sender: broadcast::Sender<E>,
}

/// Receiver for bus events.
///
/// Replaces the former ReplayReceiver; now backed directly by
/// `broadcast::Receiver`. Late subscriber catch-up is handled by
/// SessionActor disk-replay at startup.
pub type Receiver<E> = broadcast::Receiver<E>;

impl<E: Send + Clone + 'static + std::fmt::Debug> EventBus<E> {
    /// Create a new EventBus with the specified channel capacity.
    ///
    /// The internal broadcast channel is created with 2x the requested capacity
    /// to accommodate burst traffic. For the leader bus, use a capacity of at
    /// least 1000 to handle streaming deltas without dropping critical events.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1) * 2);
        Self { sender }
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns the number of subscribers that received the event.
    /// Logs a warning if there are no subscribers to catch the event.
    pub fn publish(&self, event: E) -> usize {
        let count = self.sender.receiver_count();
        if count == 0 {
            tracing::warn!("publishing event with zero subscribers: {:?}", event);
        }
        match self.sender.send(event) {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("broadcast send failed: {:?}", e);
                0
            }
        }
    }

    /// Subscribe to events.
    ///
    /// Returns a receiver that will receive events broadcast after subscription.
    pub fn subscribe(&self) -> Receiver<E> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq)]
    enum TestEvent {
        Start,
        Data(u32),
        End,
    }

    fn drain<E: Clone + Send + 'static>(sub: &mut Receiver<E>) -> Vec<E> {
        let mut events = Vec::new();
        for _ in 0..100 {
            match sub.try_recv() {
                Ok(e) => events.push(e),
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }
        events
    }

    #[tokio::test]
    async fn bus_publish_subscribe_round_trip() {
        let bus = EventBus::<TestEvent>::new(10);

        let mut sub = bus.subscribe();

        bus.publish(TestEvent::Start);
        bus.publish(TestEvent::Data(42));
        bus.publish(TestEvent::End);

        let events: Vec<TestEvent> = drain(&mut sub);
        assert_eq!(
            events,
            vec![TestEvent::Start, TestEvent::Data(42), TestEvent::End,]
        );
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        let bus = EventBus::<TestEvent>::new(10);

        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        bus.publish(TestEvent::Data(1));
        bus.publish(TestEvent::Data(2));

        let events1: Vec<TestEvent> = drain(&mut sub1);
        let events2: Vec<TestEvent> = drain(&mut sub2);

        assert_eq!(events1, vec![TestEvent::Data(1), TestEvent::Data(2)]);
        assert_eq!(events2, vec![TestEvent::Data(1), TestEvent::Data(2)]);
    }

    #[tokio::test]
    async fn publish_from_spawned_task_is_received() {
        let bus = EventBus::<TestEvent>::new(10);
        let mut sub = bus.subscribe();
        let bus2 = bus.clone();
        tokio::spawn(async move {
            bus2.publish(TestEvent::Data(99));
        });
        let event = sub.recv().await.unwrap();
        assert_eq!(event, TestEvent::Data(99));
    }

    #[tokio::test]
    async fn publish_after_spawn_blocking_is_received() {
        let bus = EventBus::<TestEvent>::new(10);
        let mut sub = bus.subscribe();
        let bus2 = bus.clone();
        tokio::spawn(async move {
            let _ = tokio::task::spawn_blocking(|| 42).await;
            bus2.publish(TestEvent::Data(99));
        });
        let event = sub.recv().await.unwrap();
        assert_eq!(event, TestEvent::Data(99));
    }

    /// Layer 2: Verify `publish` returns zero and does not panic when no subscribers.
    #[test]
    fn publish_with_no_subscribers_returns_zero() {
        let bus = Arc::new(EventBus::<TestEvent>::new(10));
        let count = bus.publish(TestEvent::Data(42));
        assert_eq!(count, 0, "publish to zero subscribers should return 0");
    }

    /// Layer 2: Verify `subscriber_count` is zero when no subscribers exist.
    #[test]
    fn subscriber_count_zero_when_no_subscriptions() {
        let bus = EventBus::<TestEvent>::new(10);
        assert_eq!(bus.subscriber_count(), 0);
    }

    /// Layer 2: Verify `subscriber_count` reflects active subscriptions.
    #[test]
    fn subscriber_count_reflects_subscriptions() {
        let bus = EventBus::<TestEvent>::new(10);
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }
}
