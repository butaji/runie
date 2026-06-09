//! Shared Event Bus with typed channels per actor
//!
//! Architecture:
//!   - Each actor has its own typed channel
//!   - Events are tagged as domain (persisted) or ephemeral (not persisted)
//!   - Subscribers can filter by event type and tag

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Bash command output (from ! prefix) — not sent to agent
    BashOutput { command: String, output: String },
    /// Configuration file changed
    ConfigChanged { path: std::path::PathBuf, changes: std::collections::HashMap<String, ConfigValue> },
}

/// Configuration value types for ConfigChanged events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<ConfigValue>),
    Object(std::collections::HashMap<String, ConfigValue>),
}

impl ConfigValue {
    /// Parse a TOML value into ConfigValue
    pub fn from_toml(value: &toml::Value) -> Self {
        match value {
            toml::Value::String(s) => ConfigValue::String(s.clone()),
            toml::Value::Integer(i) => ConfigValue::Integer(*i),
            toml::Value::Float(f) => ConfigValue::Float(*f),
            toml::Value::Boolean(b) => ConfigValue::Boolean(*b),
            toml::Value::Datetime(dt) => ConfigValue::String(dt.to_string()),
            toml::Value::Array(arr) => {
                ConfigValue::Array(arr.iter().map(ConfigValue::from_toml).collect())
            }
            toml::Value::Table(table) => {
                let mut map = std::collections::HashMap::new();
                for (k, v) in table {
                    map.insert(k.clone(), ConfigValue::from_toml(v));
                }
                ConfigValue::Object(map)
            }
        }
    }
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
}

impl SubscriptionFilter {
    pub fn all() -> Self {
        Self {
            include_domain: true,
            include_ephemeral: true,
        }
    }

    pub fn domain_only() -> Self {
        Self {
            include_domain: true,
            include_ephemeral: false,
        }
    }

    pub fn ephemeral_only() -> Self {
        Self {
            include_domain: false,
            include_ephemeral: true,
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

/// Thread-safe broadcast channel for inter-actor communication
#[derive(Clone)]
pub struct BroadcastChannel {
    tx: Sender<BusEventEnvelope>,
    rx: Arc<Mutex<Receiver<BusEventEnvelope>>>,
}

impl Default for BroadcastChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl BroadcastChannel {
    /// Create a new broadcast channel
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }

    /// Send an event to all subscribers
    pub fn send(&self, event: BusEventEnvelope) -> Result<(), std::sync::mpsc::SendError<BusEventEnvelope>> {
        self.tx.send(event)
    }

    /// Try to receive without blocking
    pub fn try_recv(&self) -> Result<BusEventEnvelope, std::sync::mpsc::TryRecvError> {
        self.rx.lock().unwrap().try_recv()
    }

    /// Receive, blocking until available
    pub fn recv(&self) -> Result<BusEventEnvelope, std::sync::mpsc::RecvError> {
        self.rx.lock().unwrap().recv()
    }

    /// Get a cloned receiver for subscribing
    pub fn receiver(&self) -> Arc<Mutex<Receiver<BusEventEnvelope>>> {
        self.rx.clone()
    }
}

/// Shared event bus for inter-actor communication
#[derive(Clone, Default)]
pub struct EventBus {
    /// Broadcast channel for events
    broadcast: BroadcastChannel,
    /// Subscription filters per actor
    subscriptions: Arc<Mutex<HashMap<ActorId, SubscriptionFilter>>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe an actor to specific event types
    pub fn subscribe(&self, actor: ActorId, filter: SubscriptionFilter) {
        self.subscriptions.lock().unwrap().insert(actor, filter);
    }

    /// Unsubscribe an actor
    pub fn unsubscribe(&self, actor: &ActorId) {
        self.subscriptions.lock().unwrap().remove(actor);
    }

    /// Publish an event to all subscribed actors
    pub fn publish(&self, event: BusEventEnvelope) {
        let _ = self.broadcast.send(event);
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
        self.broadcast.recv()
    }

    /// Try to receive from the broadcast channel
    pub fn broadcast_try_recv(&self) -> Result<BusEventEnvelope, std::sync::mpsc::TryRecvError> {
        self.broadcast.try_recv()
    }

    /// Get the broadcast receiver
    pub fn broadcast_receiver(&self) -> Arc<Mutex<Receiver<BusEventEnvelope>>> {
        self.broadcast.receiver()
    }

    /// Register a typed channel for an actor
    pub fn register<T: Clone + Send + 'static>(&self, _actor: ActorId) -> ActorChannel<T> {
        ActorChannel::new()
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
    fn test_event_bus_publish_and_broadcast() {
        let bus = EventBus::new();
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
        let bus = EventBus::new();
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

    #[test]
    fn test_event_bus_clone() {
        let bus1 = EventBus::new();
        let bus2 = bus1.clone();
        bus1.publish_domain(DomainEvent::SpawnAgent);
        assert!(bus2.broadcast_try_recv().is_ok());
    }

    #[test]
    fn test_broadcast_channel_arc_mutex() {
        let channel = BroadcastChannel::new();
        let receiver = channel.receiver();
        channel.send(BusEventEnvelope::Domain(DomainEvent::SpawnAgent)).unwrap();
        assert!(receiver.lock().unwrap().recv().is_ok());
    }
}
