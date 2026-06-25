//! Typed event bus using tokio's broadcast channel.
//!
//! Provides a publish-subscribe bus. Late subscribers are served by
//! SessionActor disk-replay on startup; no in-memory ring buffer is needed.

use tokio::sync::broadcast;

use crate::channels::ChannelDecoder;
use crate::event::Event;

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

impl<E: Send + Clone + 'static> EventBus<E> {
    /// Create a new EventBus with the specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1) * 2);
        Self { sender }
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns the number of subscribers that received the event.
    pub fn publish(&self, event: E) -> usize {
        self.sender.send(event).unwrap_or(0)
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

// ─────────────────────────────────────────────────────────────────────────────
// Specialized impl for EventBus<Event> to support channel decoders
// ─────────────────────────────────────────────────────────────────────────────

impl EventBus<Event> {
    /// Subscribe a channel decoder that processes events and forwards outputs.
    ///
    /// The decoder runs in a background thread, processing events and sending
    /// outputs through the returned channel.
    pub fn subscribe_channel<C: ChannelDecoder + 'static>(
        &self,
        mut decoder: C,
        output_tx: std::sync::mpsc::Sender<C::Output>,
    ) {
        let mut rx = self.subscribe();
        std::thread::spawn(move || {
            loop {
                match rx.try_recv() {
                    Ok(event) => {
                        if let Some(output) = decoder.process(&event) {
                            if output_tx.send(output).is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::TryRecvError::Closed) => break,
                    Err(broadcast::error::TryRecvError::Lagged(_)) | Err(broadcast::error::TryRecvError::Empty) => {}
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
