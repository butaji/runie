//! Typed event bus using tokio's broadcast channel.
//!
//! Provides a publish-subscribe bus with replay buffer for late subscribers.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Typed event bus for actor communication.
///
/// Uses tokio's broadcast channel internally to allow multiple subscribers
/// and provides a replay buffer so late subscribers can catch up on recent events.
///
/// # Type Parameter
/// - `E`: The event type (must be Send + Clone + 'static)
///
/// # Example
/// ```ignore
/// let bus = EventBus::<MyEvent>::new(100); // 100-event replay buffer
///
/// // Subscriber 1 (misses events before subscription)
/// let mut sub1 = bus.subscribe();
///
/// // Publisher
/// bus.publish(MyEvent::Start);
/// bus.publish(MyEvent::Done);
///
/// // Subscriber 2 (gets replay of recent events)
/// let mut sub2 = bus.subscribe_with_replay();
///
/// // sub1 receives: MyEvent::Start, MyEvent::Done
/// // sub2 receives: MyEvent::Start, MyEvent::Done (from replay buffer)
/// ```
#[derive(Debug, Clone)]
pub struct EventBus<E: Send + Clone + 'static> {
    sender: broadcast::Sender<E>,
    replay: Arc<ReplayBuffer<E>>,
}

/// Replay buffer stores recent events for late subscribers.
#[derive(Debug)]
struct ReplayBuffer<E: Send + Clone> {
    events: Mutex<VecDeque<E>>,
    capacity: usize,
}

impl<E: Send + Clone + 'static> EventBus<E> {
    /// Create a new EventBus with the specified replay buffer capacity.
    ///
    /// New subscribers will receive up to `capacity` of the most recent events
    /// when they first subscribe.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1) * 2);
        Self {
            sender,
            replay: Arc::new(ReplayBuffer::new(capacity)),
        }
    }

    /// Publish an event to all subscribers.
    ///
    /// Returns the number of subscribers that received the event.
    /// Late subscribers (that joined after this event) will receive it
    /// via the replay buffer when they subscribe.
    pub fn publish(&self, event: E) -> usize {
        // Store in replay buffer
        self.replay.push(event.clone());

        // Broadcast to live subscribers
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribe to events.
    ///
    /// Returns a receiver that will receive events broadcast after subscription.
    /// Note: Does not replay past events (use subscribe_with_replay for that).
    pub fn subscribe(&self) -> ReplayReceiver<E> {
        ReplayReceiver::new(self.sender.subscribe())
    }

    /// Subscribe with replay of recent events.
    ///
    /// The returned receiver will receive up to `capacity` of the most recent
    /// events from the replay buffer before continuing with live events.
    pub fn subscribe_with_replay(&self) -> ReplayReceiver<E> {
        ReplayReceiver::with_replay(self.sender.subscribe(), Arc::clone(&self.replay))
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl<E: Send + Clone> ReplayBuffer<E> {
    fn new(capacity: usize) -> Self {
        Self {
            events: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    fn push(&self, event: E) {
        let mut events = self.events.lock();
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(event);
    }

    fn clone_events(&self) -> Vec<E> {
        self.events.lock().iter().cloned().collect()
    }
}

/// Receiver that handles replay + live events.
pub struct ReplayReceiver<E: Send + Clone> {
    replay_queue: VecDeque<E>,
    receiver: broadcast::Receiver<E>,
}

impl<E: Send + Clone + 'static> ReplayReceiver<E> {
    fn new(receiver: broadcast::Receiver<E>) -> Self {
        Self {
            replay_queue: VecDeque::new(),
            receiver,
        }
    }

    /// Create a new receiver with replay buffer contents.
    fn with_replay(receiver: broadcast::Receiver<E>, replay: Arc<ReplayBuffer<E>>) -> Self {
        let events: Vec<E> = replay.clone_events();
        Self {
            replay_queue: events.into(),
            receiver,
        }
    }

    /// Receive the next event (from replay buffer first, then live).
    pub async fn recv(&mut self) -> Result<E, broadcast::error::RecvError> {
        // First yield from replay buffer if any
        if let Some(event) = self.replay_queue.pop_front() {
            return Ok(event);
        }
        // Then wait for live events
        self.receiver.recv().await
    }

    /// Try to receive an event without waiting. Returns None if no event is available.
    pub fn try_recv(&mut self) -> Option<Result<E, broadcast::error::RecvError>> {
        // First check replay buffer
        if let Some(event) = self.replay_queue.pop_front() {
            return Some(Ok(event));
        }
        // Then try non-blocking receive
        match self.receiver.try_recv() {
            Ok(event) => Some(Ok(event)),
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => {
                Some(Err(broadcast::error::RecvError::Closed))
            }
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                Some(Err(broadcast::error::RecvError::Lagged(n)))
            }
        }
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
    async fn event_bus_replays_last_n_events() {
        let bus = EventBus::<TestEvent>::new(5);

        // Publish more events than replay capacity
        for i in 1..=10 {
            bus.publish(TestEvent::Data(i));
        }

        // Late subscriber with replay should get only last 5 events (not all 10)
        let mut sub = bus.subscribe_with_replay();
        let events: Vec<TestEvent> = drain(&mut sub);

        // Should contain last 5 events: Data(6), Data(7), Data(8), Data(9), Data(10)
        assert_eq!(events.len(), 5);
        assert_eq!(
            events,
            vec![
                TestEvent::Data(6),
                TestEvent::Data(7),
                TestEvent::Data(8),
                TestEvent::Data(9),
                TestEvent::Data(10),
            ]
        );
    }

    #[tokio::test]
    async fn replay_not_consumed_by_first_subscriber() {
        let bus = EventBus::<TestEvent>::new(5);

        for i in 1..=3 {
            bus.publish(TestEvent::Data(i));
        }

        let mut sub1 = bus.subscribe_with_replay();
        let events1: Vec<TestEvent> = drain(&mut sub1);
        assert_eq!(
            events1,
            vec![TestEvent::Data(1), TestEvent::Data(2), TestEvent::Data(3)]
        );

        // A second late subscriber should still receive the same replay.
        let mut sub2 = bus.subscribe_with_replay();
        let events2: Vec<TestEvent> = drain(&mut sub2);
        assert_eq!(
            events2,
            vec![TestEvent::Data(1), TestEvent::Data(2), TestEvent::Data(3)]
        );
    }

    fn drain<E: Clone + Send + 'static>(sub: &mut ReplayReceiver<E>) -> Vec<E> {
        let mut events = Vec::new();
        for _ in 0..100 {
            if let Some(Ok(e)) = sub.try_recv() {
                events.push(e);
            }
        }
        events
    }
}
