//! Orchestrator — Central spawn point for all actors
//!
//! The Orchestrator holds typed channels to each actor and routes
//! messages accordingly. ToolActors are spawned dynamically on
//! ToolStart events.

use crate::event_bus::{
    ActorChannel, ActorId, BusEventEnvelope, DomainEvent, EphemeralEvent, EventBus,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Actor handle for sending messages to spawned actors
#[derive(Clone)]
pub struct ActorHandle {
    pub id: ActorId,
    tx: Arc<std::sync::mpsc::Sender<BusEventEnvelope>>,
}

impl ActorHandle {
    /// Send a domain event to this actor
    pub fn tell(&self, event: DomainEvent) {
        let _ = self.tx.send(BusEventEnvelope::Domain(event));
    }

    /// Send an ephemeral event to this actor
    pub fn tell_ephemeral(&self, event: EphemeralEvent) {
        let _ = self.tx.send(BusEventEnvelope::Ephemeral(event));
    }
}

/// Orchestrator manages all actors and their channels
pub struct Orchestrator {
    /// The shared event bus (thread-safe via Arc)
    bus: EventBus,
    /// Typed handles to all spawned actors
    actors: HashMap<ActorId, ActorHandle>,
    /// Join handles for actor threads
    threads: Vec<JoinHandle<()>>,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Orchestrator {
    /// Create a new orchestrator with the shared event bus
    pub fn new() -> Self {
        Self {
            bus: EventBus::new(),
            actors: HashMap::new(),
            threads: Vec::new(),
        }
    }

    /// Register an actor and return its channel
    pub fn register<T: Clone + Send + 'static>(&mut self, actor: ActorId) -> ActorChannel<T> {
        self.bus.register(actor)
    }

    /// Spawn an actor with its event handler
    ///
    /// The actor function receives the bus and its typed channel,
    /// and runs in a dedicated thread.
    pub fn spawn<F>(&mut self, id: ActorId, handler: F) -> ActorHandle
    where
        F: FnOnce(EventBus, ActorChannel<BusEventEnvelope>) + Send + 'static,
    {
        let channel = self.register::<BusEventEnvelope>(id.clone());
        let bus = self.bus.clone();

        let handle = ActorHandle {
            id: id.clone(),
            tx: Arc::new(channel.tx.clone()),
        };

        let thread = thread::spawn(move || {
            handler(bus, channel);
        });

        self.threads.push(thread);
        self.actors.insert(id, handle.clone());
        handle
    }

    /// Spawn the AgentLoop actor
    pub fn spawn_agent_loop<F>(&mut self, handler: F) -> ActorHandle
    where
        F: FnOnce(EventBus, ActorChannel<BusEventEnvelope>) + Send + 'static,
    {
        self.spawn(ActorId::AgentLoop, handler)
    }

    /// Spawn the QueueAgent actor
    pub fn spawn_queue_agent<F>(&mut self, handler: F) -> ActorHandle
    where
        F: FnOnce(EventBus, ActorChannel<BusEventEnvelope>) + Send + 'static,
    {
        self.spawn(ActorId::QueueAgent, handler)
    }

    /// Spawn the SessionManager actor
    pub fn spawn_session_manager<F>(&mut self, handler: F) -> ActorHandle
    where
        F: FnOnce(EventBus, ActorChannel<BusEventEnvelope>) + Send + 'static,
    {
        self.spawn(ActorId::SessionManager, handler)
    }

    /// Spawn the ConfigAgent actor
    pub fn spawn_config_agent<F>(&mut self, handler: F) -> ActorHandle
    where
        F: FnOnce(EventBus, ActorChannel<BusEventEnvelope>) + Send + 'static,
    {
        self.spawn(ActorId::ConfigAgent, handler)
    }

    /// Spawn a ToolActor dynamically on ToolStart events
    pub fn spawn_tool_actor<F>(&mut self, name: String, handler: F) -> ActorHandle
    where
        F: FnOnce(EventBus, ActorChannel<BusEventEnvelope>) + Send + 'static,
    {
        let id = ActorId::ToolActor { name };
        self.spawn(id, handler)
    }

    /// Get a handle to a spawned actor
    pub fn actor(&self, id: &ActorId) -> Option<&ActorHandle> {
        self.actors.get(id)
    }

    /// Get a clone of the shared event bus
    pub fn bus(&self) -> EventBus {
        self.bus.clone()
    }

    /// Send a domain event to a specific actor
    pub fn tell(&self, actor: &ActorId, event: DomainEvent) {
        if let Some(handle) = self.actors.get(actor) {
            handle.tell(event);
        }
    }

    /// Send an ephemeral event to a specific actor
    pub fn tell_ephemeral(&self, actor: &ActorId, event: EphemeralEvent) {
        if let Some(handle) = self.actors.get(actor) {
            handle.tell_ephemeral(event);
        }
    }

    /// Broadcast a domain event to all actors via the bus
    pub fn broadcast(&self, event: DomainEvent) {
        self.bus.publish_domain(event);
    }

    /// Broadcast an ephemeral event to all actors via the bus
    pub fn broadcast_ephemeral(&self, event: EphemeralEvent) {
        self.bus.publish_ephemeral(event);
    }

    /// Get list of all spawned actor IDs
    pub fn actor_ids(&self) -> Vec<ActorId> {
        self.actors.keys().cloned().collect()
    }

    /// Wait for all actor threads to complete
    pub fn shutdown(self) {
        // Drop the actors HashMap to release handles
        drop(self.actors);
        // Join all threads
        for thread in self.threads {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let orch = Orchestrator::new();
        assert!(orch.actor_ids().is_empty());
    }

    #[test]
    fn test_spawn_actor() {
        let mut orch = Orchestrator::new();
        let received = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let received_clone = received.clone();

        let handle = orch.spawn(ActorId::AgentLoop, move |_bus, _channel| {
            received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        assert_eq!(handle.id, ActorId::AgentLoop);
        assert!(orch.actor(&ActorId::AgentLoop).is_some());
    }

    #[test]
    fn test_spawn_agent_loop() {
        let mut orch = Orchestrator::new();
        let handle = orch.spawn_agent_loop(|_bus, _channel| {});
        assert_eq!(handle.id, ActorId::AgentLoop);
    }

    #[test]
    fn test_spawn_queue_agent() {
        let mut orch = Orchestrator::new();
        let handle = orch.spawn_queue_agent(|_bus, _channel| {});
        assert_eq!(handle.id, ActorId::QueueAgent);
    }

    #[test]
    fn test_spawn_session_manager() {
        let mut orch = Orchestrator::new();
        let handle = orch.spawn_session_manager(|_bus, _channel| {});
        assert_eq!(handle.id, ActorId::SessionManager);
    }

    #[test]
    fn test_spawn_config_agent() {
        let mut orch = Orchestrator::new();
        let handle = orch.spawn_config_agent(|_bus, _channel| {});
        assert_eq!(handle.id, ActorId::ConfigAgent);
    }

    #[test]
    fn test_spawn_tool_actor() {
        let mut orch = Orchestrator::new();
        let handle = orch.spawn_tool_actor("bash".to_string(), |_bus, _channel| {});
        assert!(matches!(
            handle.id,
            ActorId::ToolActor { name } if name == "bash"
        ));
    }

    #[test]
    fn test_actor_handle_tell() {
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = ActorHandle {
            id: ActorId::AgentLoop,
            tx: Arc::new(tx),
        };

        handle.tell(DomainEvent::Submit { content: "test".into() });

        let received = rx.recv_timeout(std::time::Duration::from_millis(100)).unwrap();
        match received {
            BusEventEnvelope::Domain(DomainEvent::Submit { content }) => {
                assert_eq!(content, "test");
            }
            _ => panic!("Expected domain Submit event"),
        }
    }

    #[test]
    fn test_actor_handle_tell_ephemeral() {
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = ActorHandle {
            id: ActorId::InputActor,
            tx: Arc::new(tx),
        };

        handle.tell_ephemeral(EphemeralEvent::ScrollUp);

        let received = rx.recv_timeout(std::time::Duration::from_millis(100)).unwrap();
        match received {
            BusEventEnvelope::Ephemeral(EphemeralEvent::ScrollUp) => {}
            _ => panic!("Expected ephemeral ScrollUp event"),
        }
    }

    #[test]
    fn test_orchestrator_tell() {
        let mut orch = Orchestrator::new();
        let received = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let received_clone = received.clone();

        let _handle = orch.spawn(ActorId::AgentLoop, move |_bus, channel: ActorChannel<BusEventEnvelope>| {
            while channel.rx.recv().is_ok() {
                received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        });

        orch.tell(&ActorId::AgentLoop, DomainEvent::Submit { content: "test".into() });

        // Give thread time to receive
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(received.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_orchestrator_broadcast() {
        let mut orch = Orchestrator::new();
        let received = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let received_clone = received.clone();

        let _handle = orch.spawn(ActorId::AgentLoop, move |bus, _channel| {
            bus.subscribe(ActorId::AgentLoop, crate::event_bus::SubscriptionFilter::domain_only());
            if bus.broadcast_try_recv().is_ok() {
                received_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });

        orch.broadcast(DomainEvent::SpawnAgent);
    }

    #[test]
    fn test_orchestrator_actor_ids() {
        let mut orch = Orchestrator::new();
        assert!(orch.actor_ids().is_empty());

        let _h1 = orch.spawn_agent_loop(|_bus, _ch| {});
        let _h2 = orch.spawn_queue_agent(|_bus, _ch| {});

        let ids = orch.actor_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&ActorId::AgentLoop));
        assert!(ids.contains(&ActorId::QueueAgent));
    }

    #[test]
    fn test_multiple_tool_actors() {
        let mut orch = Orchestrator::new();
        let h1 = orch.spawn_tool_actor("bash".to_string(), |_bus, _ch| {});
        let h2 = orch.spawn_tool_actor("read".to_string(), |_bus, _ch| {});

        assert!(matches!(h1.id, ActorId::ToolActor { name } if name == "bash"));
        assert!(matches!(h2.id, ActorId::ToolActor { name } if name == "read"));
    }
}
