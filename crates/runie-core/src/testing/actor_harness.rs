//! Test harness for actor-based state.
//!
//! This module provides a `TestHarness` that enables unit testing of actor-based
//! state without requiring a full runtime or async infrastructure. It records
//! sent messages and emitted facts for verification.
//!
//! # Usage
//!
//! ```ignore
//! use crate::testing::actor_harness::TestHarness;
//! use crate::actors::{InputMsg, input::InputActor};
//!
//! let harness = TestHarness::new();
//! harness.spawn_actor::<InputActor>();
//! harness.send(InputMsg::InsertChar('h'));
//! assert!(harness.facts().contains(&/* expected fact */));
//! ```

use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

/// A test event bus that records all published events.
#[derive(Clone, Debug)]
pub struct TestEventBus<E> {
    events: Arc<Mutex<Vec<E>>>,
    inner: broadcast::Sender<E>,
}

impl<E: Clone> TestEventBus<E> {
    /// Create a new test event bus.
    pub fn new(capacity: usize) -> Self {
        let (inner, _) = broadcast::channel(capacity);
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            inner,
        }
    }

    /// Publish an event to the bus.
    pub fn publish(&self, event: E) {
        if let Err(e) = self.inner.send(event.clone()) {
            // Channel closed - ignore
            let _ = e;
        }
        if let Ok(mut guard) = self.events.lock() {
            guard.push(event);
        }
    }

    /// Subscribe to events (for forwarding to real actors).
    pub fn subscribe(&self) -> broadcast::Receiver<E> {
        self.inner.subscribe()
    }

    /// Get all events that have been published.
    pub fn events(&self) -> Vec<E> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Clear all recorded events.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.events.lock() {
            guard.clear();
        }
    }
}

impl<E: Clone> Default for TestEventBus<E> {
    fn default() -> Self {
        Self::new(32)
    }
}

/// Test harness for actor-based state.
///
/// Provides a controlled environment for testing actors without a full runtime:
/// - Records all emitted facts/events
/// - Allows spawning actors with a test event bus
/// - Provides synchronous access to recorded facts
///
/// # Type Parameters
/// - `E`: The event type emitted by actors
pub struct TestHarness<E: Clone> {
    bus: TestEventBus<E>,
}

impl<E: Clone> Default for TestHarness<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Clone> TestHarness<E> {
    /// Create a new empty harness with a test event bus.
    pub fn new() -> Self {
        Self {
            bus: TestEventBus::new(32),
        }
    }

    /// Get a clone of the test event bus.
    pub fn bus(&self) -> TestEventBus<E> {
        self.bus.clone()
    }

    /// Get all recorded facts/events.
    pub fn facts(&self) -> Vec<E> {
        self.bus.events()
    }

    /// Clear all recorded facts.
    pub fn clear(&self) {
        self.bus.clear();
    }

    /// Publish a fact to the test bus.
    pub fn publish(&self, event: E) {
        self.bus.publish(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test event types for the test harness.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TestEvent {
        Increment,
        Decrement,
        Value(i32),
    }

    /// Simple counter actor for testing.
    pub mod counter_actor {
        use super::*;
        use crate::actors::Actor;

        pub struct CounterActor {
            value: i32,
        }

        impl CounterActor {
            pub fn new() -> Self {
                Self { value: 0 }
            }
        }

        #[derive(Clone, Debug)]
        pub enum CounterMsg {
            Increment,
            Decrement,
            GetValue,
        }

        impl Actor for CounterActor {
            type Msg = CounterMsg;
            type Event = TestEvent;

            async fn run_body(
                self,
                mut rx: tokio::sync::mpsc::Receiver<Self::Msg>,
                bus: crate::bus::EventBus<Self::Event>,
            ) {
                let mut value = self.value;
                while let Some(msg) = rx.recv().await {
                    match msg {
                        CounterMsg::Increment => {
                            value += 1;
                            bus.publish(TestEvent::Increment);
                            bus.publish(TestEvent::Value(value));
                        }
                        CounterMsg::Decrement => {
                            value -= 1;
                            bus.publish(TestEvent::Decrement);
                            bus.publish(TestEvent::Value(value));
                        }
                        CounterMsg::GetValue => {
                            bus.publish(TestEvent::Value(value));
                        }
                    }
                }
            }
        }
    }

    use counter_actor::*;

    /// L1: TestHarness::new creates a harness with an empty bus.
    #[test]
    fn harness_new_is_empty() {
        let harness: TestHarness<TestEvent> = TestHarness::new();
        assert!(harness.facts().is_empty());
    }

    /// L1: TestHarness::bus returns a clone of the test bus.
    #[test]
    fn harness_bus_works() {
        let harness: TestHarness<TestEvent> = TestHarness::new();
        let bus = harness.bus();
        assert!(bus.events().is_empty());
    }

    /// L1: TestEventBus records published events.
    #[test]
    fn test_bus_records_events() {
        let bus = TestEventBus::<TestEvent>::new(32);
        bus.publish(TestEvent::Increment);
        bus.publish(TestEvent::Value(42));

        let events = bus.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], TestEvent::Increment);
        assert_eq!(events[1], TestEvent::Value(42));
    }

    /// L1: TestEventBus::clear removes all events.
    #[test]
    fn test_bus_clear() {
        let bus = TestEventBus::<TestEvent>::new(32);
        bus.publish(TestEvent::Increment);
        bus.publish(TestEvent::Value(1));
        assert_eq!(bus.events().len(), 2);

        bus.clear();
        assert_eq!(bus.events().len(), 0);
    }

    /// L1: Actor receives and handles messages via test bus.
    #[tokio::test]
    async fn actor_handles_increment() {
        let harness: TestHarness<TestEvent> = TestHarness::new();
        let test_bus = harness.bus();
        let actor_bus = crate::bus::EventBus::new(32);

        // Forward events to our test bus
        let forwarding_bus = test_bus.clone();
        let actor_bus_for_forwarder = actor_bus.clone();
        let _handle = tokio::spawn(async move {
            let mut rx = actor_bus_for_forwarder.subscribe();
            while let Ok(event) = rx.recv().await {
                forwarding_bus.publish(event);
            }
        });

        let actor = CounterActor::new();
        let (tx, handle) = crate::actors::spawn_actor(actor, actor_bus);

        tx.send(CounterMsg::Increment).await.unwrap();
        tokio::task::yield_now().await;
        tx.send(CounterMsg::Increment).await.unwrap();
        tokio::task::yield_now().await;
        tx.send(CounterMsg::GetValue).await.unwrap();
        tokio::task::yield_now().await;

        drop(tx);
        handle.await.unwrap();

        let events = harness.facts();
        assert!(events.contains(&TestEvent::Increment));
        assert!(events.contains(&TestEvent::Value(2)));
    }

    /// L2: Harness::publish adds facts.
    #[test]
    fn harness_publish_adds_facts() {
        let harness: TestHarness<TestEvent> = TestHarness::new();
        harness.publish(TestEvent::Value(99));

        let events = harness.facts();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], TestEvent::Value(99));
    }

    /// L2: Harness::clear removes all facts.
    #[test]
    fn harness_clear_works() {
        let harness: TestHarness<TestEvent> = TestHarness::new();
        harness.publish(TestEvent::Increment);
        harness.publish(TestEvent::Value(5));
        assert_eq!(harness.facts().len(), 2);

        harness.clear();
        assert!(harness.facts().is_empty());
    }

    /// L1: TestEventBus supports multiple subscribers (via subscribe).
    #[tokio::test]
    async fn test_bus_multiple_subscribers() {
        let bus = TestEventBus::<TestEvent>::new(32);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.publish(TestEvent::Increment);

        // Both subscribers should receive the event
        let evt1 = rx1.recv().await.unwrap();
        let evt2 = rx2.recv().await.unwrap();

        assert_eq!(evt1, TestEvent::Increment);
        assert_eq!(evt2, TestEvent::Increment);
    }

    /// L1: Counter actor correctly decrements.
    #[tokio::test]
    async fn actor_handles_decrement() {
        let harness: TestHarness<TestEvent> = TestHarness::new();
        let test_bus = harness.bus();
        let actor_bus = crate::bus::EventBus::new(32);

        let forwarding_bus = test_bus.clone();
        let actor_bus_for_forwarder = actor_bus.clone();
        let _handle = tokio::spawn(async move {
            let mut rx = actor_bus_for_forwarder.subscribe();
            while let Ok(event) = rx.recv().await {
                forwarding_bus.publish(event);
            }
        });

        let actor = CounterActor::new();
        let (tx, handle) = crate::actors::spawn_actor(actor, actor_bus);

        tx.send(CounterMsg::Decrement).await.unwrap();
        tokio::task::yield_now().await;
        tx.send(CounterMsg::Decrement).await.unwrap();
        tokio::task::yield_now().await;

        drop(tx);
        handle.await.unwrap();

        let events = harness.facts();
        assert!(events.contains(&TestEvent::Decrement));
        assert!(events.contains(&TestEvent::Value(-2)));
    }
}
