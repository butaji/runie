//! Shared Event Bus with typed channels per actor
//!
//! Architecture:
//!   - Each actor has its own typed channel
//!   - Events are tagged as domain (persisted) or ephemeral (not persisted)
//!   - Subscribers can filter by event type and tag

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

/// Event tag determines persistence behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTag {
    /// Domain events are persisted to session history
    Domain,
    /// Ephemeral events are not persisted
    Ephemeral,
}

/// Trait for event types that can be published on the bus
pub trait BusEvent: Clone + Send + 'static {
    /// Returns the event's tag (domain vs ephemeral)
    fn tag(&self) -> EventTag;
}

/// Domain events — persisted to session
#[derive(Debug, Clone)]
pub enum DomainEvent {
    Submit { content: String },
    SpawnAgent,
    AgentThinking { id: String },
    AgentThoughtDone { id: String },
    AgentResponse { id: String, content: String },
    AgentTurnComplete { id: String, duration_secs: f64 },
    AgentToolStart { id: String, name: String },
    AgentToolEnd { id: String, name: String, duration_secs: f64, output: String },
    AgentDone { id: String },
    AgentError { id: String, message: String },
    SwitchModel { provider: String, model: String },
    FollowUp { content: String },
    ToolRegistered { name: String },
}

impl BusEvent for DomainEvent {
    fn tag(&self) -> EventTag {
        EventTag::Domain
    }
}

/// Ephemeral events — not persisted
#[derive(Debug, Clone)]
pub enum EphemeralEvent {
    Input(char),
    Backspace,
    CursorLeft,
    CursorRight,
    CursorStart,
    CursorEnd,
    DeleteWord,
    DeleteToEnd,
    DeleteToStart,
    KillChar,
    HistoryPrev,
    HistoryNext,
    Undo,
    Redo,
    CursorWordLeft,
    CursorWordRight,
    Paste(String),
    ScrollUp,
    ScrollDown,
    ToggleExpand,
    Abort,
}

impl BusEvent for EphemeralEvent {
    fn tag(&self) -> EventTag {
        EventTag::Ephemeral
    }
}

/// Unified event envelope for the bus
#[derive(Debug, Clone)]
pub enum BusEventEnvelope {
    Domain(DomainEvent),
    Ephemeral(EphemeralEvent),
}

impl BusEventEnvelope {
    pub fn tag(&self) -> EventTag {
        match self {
            BusEventEnvelope::Domain(_) => EventTag::Domain,
            BusEventEnvelope::Ephemeral(_) => EventTag::Ephemeral,
        }
    }
}

/// Actor identifier for typed channels
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActorId {
    AgentLoop,
    QueueAgent,
    SessionManager,
    ConfigAgent,
    ToolActor { name: String },
    RenderActor,
    InputActor,
}

impl std::fmt::Display for ActorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorId::AgentLoop => write!(f, "AgentLoop"),
            ActorId::QueueAgent => write!(f, "QueueAgent"),
            ActorId::SessionManager => write!(f, "SessionManager"),
            ActorId::ConfigAgent => write!(f, "ConfigAgent"),
            ActorId::ToolActor { name } => write!(f, "ToolActor({})", name),
            ActorId::RenderActor => write!(f, "RenderActor"),
            ActorId::InputActor => write!(f, "InputActor"),
        }
    }
}

/// Typed channel endpoint for an actor
#[derive(Debug)]
pub struct ActorChannel<T> {
    pub tx: Sender<T>,
    pub rx: Receiver<T>,
}

impl<T: Clone + Send + 'static> ActorChannel<T> {
    /// Create a new typed channel pair
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&self) -> Result<T, std::sync::mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    /// Receive a message, blocking until available
    pub fn recv(&self) -> Result<T, std::sync::mpsc::RecvError> {
        self.rx.recv()
    }
}

impl<T: Clone + Send + 'static> Default for ActorChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscription filter for event filtering
#[derive(Debug, Clone, Default)]
pub struct SubscriptionFilter {
    pub include_domain: bool,
    pub include_ephemeral: bool,
    pub actor_ids: Option<Vec<ActorId>>,
}

impl SubscriptionFilter {
    pub fn all() -> Self {
        Self {
            include_domain: true,
            include_ephemeral: true,
            actor_ids: None,
        }
    }

    pub fn domain_only() -> Self {
        Self {
            include_domain: true,
            include_ephemeral: false,
            actor_ids: None,
        }
    }

    pub fn ephemeral_only() -> Self {
        Self {
            include_domain: false,
            include_ephemeral: true,
            actor_ids: None,
        }
    }

    pub fn matches(&self, event: &BusEventEnvelope) -> bool {
        match event {
            BusEventEnvelope::Domain(_) if !self.include_domain => false,
            BusEventEnvelope::Ephemeral(_) if !self.include_ephemeral => false,
            _ => true,
        }
    }
}

/// Shared event bus for inter-actor communication
pub struct EventBus {
    /// Typed channels per actor
    channels: HashMap<ActorId, Box<dyn std::any::Any + Send>>,
    /// Broadcast channel for events
    broadcast_tx: Sender<BusEventEnvelope>,
    broadcast_rx: Receiver<BusEventEnvelope>,
    /// Subscription filters per actor
    subscriptions: HashMap<ActorId, SubscriptionFilter>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            channels: HashMap::new(),
            broadcast_tx: tx,
            broadcast_rx: rx,
            subscriptions: HashMap::new(),
        }
    }

    /// Register a typed channel for an actor
    pub fn register<T: Clone + Send + 'static>(&mut self, actor: ActorId) -> ActorChannel<T> {
        let channel = ActorChannel::new();
        self.channels.insert(actor.clone(), Box::new(channel.tx.clone()));
        self.subscriptions.insert(actor, SubscriptionFilter::all());
        channel
    }

    /// Subscribe an actor to specific event types
    pub fn subscribe(&mut self, actor: ActorId, filter: SubscriptionFilter) {
        self.subscriptions.insert(actor, filter);
    }

    /// Publish an event to all subscribed actors
    pub fn publish(&self, event: BusEventEnvelope) {
        // Send to broadcast channel
        let _ = self.broadcast_tx.send(event.clone());

        // Route to typed channels based on subscriptions
        for (actor, filter) in &self.subscriptions {
            if filter.matches(&event) {
                if let Some(channel) = self.channels.get(actor) {
                    if let Some(tx) = channel.downcast_ref::<Sender<BusEventEnvelope>>() {
                        let _ = tx.send(event.clone());
                    }
                }
            }
        }
    }

    /// Publish a domain event
    pub fn publish_domain(&self, event: DomainEvent) {
        self.publish(BusEventEnvelope::Domain(event));
    }

    /// Publish an ephemeral event
    pub fn publish_ephemeral(&self, event: EphemeralEvent) {
        self.publish(BusEventEnvelope::Ephemeral(event));
    }

    /// Receive from the broadcast channel
    pub fn broadcast_recv(&self) -> Result<BusEventEnvelope, std::sync::mpsc::RecvError> {
        self.broadcast_rx.recv()
    }

    /// Try to receive from the broadcast channel
    pub fn broadcast_try_recv(&self) -> Result<BusEventEnvelope, std::sync::mpsc::TryRecvError> {
        self.broadcast_rx.try_recv()
    }

    /// Get the broadcast receiver for the render actor
    pub fn broadcast_receiver(&self) -> &Receiver<BusEventEnvelope> {
        &self.broadcast_rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_events_are_tagged() {
        let event = DomainEvent::Submit { content: "test".into() };
        assert_eq!(event.tag(), EventTag::Domain);
    }

    #[test]
    fn test_ephemeral_events_are_tagged() {
        let event = EphemeralEvent::ScrollUp;
        assert_eq!(event.tag(), EventTag::Ephemeral);
    }

    #[test]
    fn test_event_envelope_tag() {
        let domain = BusEventEnvelope::Domain(DomainEvent::Submit { content: "test".into() });
        assert_eq!(domain.tag(), EventTag::Domain);

        let ephemeral = BusEventEnvelope::Ephemeral(EphemeralEvent::Input('a'));
        assert_eq!(ephemeral.tag(), EventTag::Ephemeral);
    }

    #[test]
    fn test_actor_channel_basic() {
        let channel: ActorChannel<String> = ActorChannel::new();
        channel.tx.send("hello".to_string()).unwrap();
        let received = channel.rx.recv().unwrap();
        assert_eq!(received, "hello");
    }

    #[test]
    fn test_subscription_filter_domain_only() {
        let filter = SubscriptionFilter::domain_only();
        let domain = BusEventEnvelope::Domain(DomainEvent::Submit { content: "test".into() });
        let ephemeral = BusEventEnvelope::Ephemeral(EphemeralEvent::ScrollUp);
        assert!(filter.matches(&domain));
        assert!(!filter.matches(&ephemeral));
    }

    #[test]
    fn test_subscription_filter_ephemeral_only() {
        let filter = SubscriptionFilter::ephemeral_only();
        let domain = BusEventEnvelope::Domain(DomainEvent::Submit { content: "test".into() });
        let ephemeral = BusEventEnvelope::Ephemeral(EphemeralEvent::ScrollUp);
        assert!(!filter.matches(&domain));
        assert!(filter.matches(&ephemeral));
    }

    #[test]
    fn test_event_bus_register() {
        let mut bus = EventBus::new();
        let channel: ActorChannel<String> = bus.register(ActorId::AgentLoop);
        assert!(channel.tx.send("test".to_string()).is_ok());
    }

    #[test]
    fn test_event_bus_publish_and_broadcast() {
        let mut bus = EventBus::new();
        let _channel: ActorChannel<BusEventEnvelope> = bus.register(ActorId::RenderActor);

        bus.publish_domain(DomainEvent::Submit { content: "hello".into() });

        let received = bus.broadcast_recv().unwrap();
        match received {
            BusEventEnvelope::Domain(DomainEvent::Submit { content }) => {
                assert_eq!(content, "hello");
            }
            _ => panic!("Expected domain Submit event"),
        }
    }

    #[test]
    fn test_event_bus_ephemeral_publish() {
        let mut bus = EventBus::new();
        let _channel: ActorChannel<BusEventEnvelope> = bus.register(ActorId::InputActor);

        bus.publish_ephemeral(EphemeralEvent::ScrollUp);

        let received = bus.broadcast_recv().unwrap();
        match received {
            BusEventEnvelope::Ephemeral(EphemeralEvent::ScrollUp) => {}
            _ => panic!("Expected ephemeral ScrollUp event"),
        }
    }

    #[test]
    fn test_actor_id_display() {
        assert_eq!(ActorId::AgentLoop.to_string(), "AgentLoop");
        assert_eq!(ActorId::SessionManager.to_string(), "SessionManager");
        assert_eq!(
            ActorId::ToolActor { name: "bash".into() }.to_string(),
            "ToolActor(bash)"
        );
    }
}
