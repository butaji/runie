//! QueueAgent — Manages the message queue and spawns agent when idle
//!
//! The QueueAgent holds pending requests and emits SpawnAgent when the
//! agent becomes idle and has capacity for more work.

use crate::event_bus::{ActorChannel, ActorId, BusEventEnvelope, DomainEvent, EventBus};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Queue delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueMode {
    /// Process one request at a time
    Sequential,
    /// Process multiple requests concurrently (default)
    Concurrent,
}

impl Default for QueueMode {
    fn default() -> Self {
        QueueMode::Concurrent
    }
}

/// Request entry in the queue
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub content: String,
    pub request_id: String,
    pub provider: String,
    pub model: String,
}

/// Queue agent state
#[derive(Debug, Default)]
pub struct QueueState {
    /// Pending requests
    pub queue: VecDeque<QueuedRequest>,
    /// Delivery mode
    pub mode: QueueMode,
    /// Number of in-flight requests
    pub inflight: usize,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
}

impl QueueState {
    /// Add a request to the queue
    pub fn enqueue(&mut self, request: QueuedRequest) {
        self.queue.push_back(request);
    }

    /// Get the next request if allowed by concurrency limits
    pub fn peek_next(&self) -> Option<&QueuedRequest> {
        let has_capacity = self.inflight < self.max_concurrent;
        let has_pending = !self.queue.is_empty();
        let allowed_by_mode = match self.mode {
            QueueMode::Sequential => self.inflight == 0,
            QueueMode::Concurrent => true,
        };

        if has_capacity && has_pending && allowed_by_mode {
            self.queue.front()
        } else {
            None
        }
    }

    /// Pop and return the next request
    pub fn dequeue(&mut self) -> Option<QueuedRequest> {
        if self.peek_next().is_some() {
            self.inflight += 1;
            self.queue.pop_front()
        } else {
            None
        }
    }

    /// Mark an in-flight request as complete
    pub fn complete(&mut self) {
        if self.inflight > 0 {
            self.inflight -= 1;
        }
    }

    /// Set the queue mode
    pub fn set_mode(&mut self, mode: QueueMode) {
        self.mode = mode;
    }

    /// Clear all pending requests
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

/// Run the QueueAgent actor loop.
///
/// The actor:
/// - Receives Submit events and queues them
/// - Monitors agent idle state via InflightDecremented
/// - Emits SpawnAgent when queue has work and agent is idle
pub fn run_queue_agent(
    bus: EventBus,
    channel: ActorChannel<BusEventEnvelope>,
    shutdown: Arc<AtomicBool>,
    max_concurrent: usize,
) {
    let mut state = QueueState {
        max_concurrent,
        ..Default::default()
    };

    // Subscribe to domain events for queue management
    bus.subscribe(ActorId::QueueAgent, crate::event_bus::SubscriptionFilter::domain_only());

    while !shutdown.load(Ordering::SeqCst) {
        // Process any queued events
        while let Ok(event) = channel.try_recv() {
            match event {
                BusEventEnvelope::Domain(domain) => {
                    handle_domain_event(&domain, &mut state, &bus);
                }
                BusEventEnvelope::Ephemeral(_) => {
                    // Ignore ephemeral events
                }
            }
        }

        // Check if we should spawn
        try_spawn(&mut state, &bus);

        // Small sleep to avoid busy loop
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Handle a domain event for queue management
fn handle_domain_event(event: &DomainEvent, state: &mut QueueState, bus: &EventBus) {
    match event {
        DomainEvent::Submit { content } => {
            // Generate request ID
            let id = format!("req.{}", state.len() + state.inflight);
            let request = QueuedRequest {
                content: content.clone(),
                request_id: id,
                provider: String::new(), // Will be filled by agent
                model: String::new(),
            };
            state.enqueue(request);
        }
        DomainEvent::AgentDone { .. } | DomainEvent::AgentError { .. } => {
            state.complete();
            try_spawn(state, bus);
        }
        DomainEvent::AgentTurnComplete { .. } => {
            state.complete();
            try_spawn(state, bus);
        }
        _ => {}
    }
}

/// Try to spawn agent if there's work and capacity
fn try_spawn(state: &QueueState, bus: &EventBus) {
    if state.peek_next().is_some() {
        bus.publish_domain(DomainEvent::SpawnAgent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_state_default() {
        let state = QueueState::default();
        assert!(state.is_empty());
        assert_eq!(state.inflight, 0);
        assert_eq!(state.mode, QueueMode::Concurrent);
    }

    #[test]
    fn enqueue_and_dequeue() {
        let mut state = QueueState {
            max_concurrent: 10,
            ..Default::default()
        };

        state.enqueue(QueuedRequest {
            content: "hello".to_string(),
            request_id: "req.0".to_string(),
            provider: "test".to_string(),
            model: "model".to_string(),
        });

        assert_eq!(state.len(), 1);
        assert!(state.peek_next().is_some());

        let req = state.dequeue();
        assert!(req.is_some());
        assert_eq!(req.unwrap().content, "hello");
        assert_eq!(state.inflight, 1);
    }

    #[test]
    fn concurrent_mode_allows_multiple() {
        let mut state = QueueState {
            max_concurrent: 3,
            ..Default::default()
        };

        for i in 0..3 {
            state.enqueue(QueuedRequest {
                content: format!("task {}", i),
                request_id: format!("req.{}", i),
                provider: String::new(),
                model: String::new(),
            });
        }

        // Should be able to dequeue all 3 in concurrent mode
        assert!(state.peek_next().is_some());
        state.dequeue();
        assert!(state.peek_next().is_some());
        state.dequeue();
        assert!(state.peek_next().is_some());
        state.dequeue();
        assert!(state.peek_next().is_none());
    }

    #[test]
    fn sequential_mode_blocks_second() {
        let mut state = QueueState {
            max_concurrent: 3,
            mode: QueueMode::Sequential,
            ..Default::default()
        };

        state.enqueue(QueuedRequest {
            content: "task 1".to_string(),
            request_id: "req.0".to_string(),
            provider: String::new(),
            model: String::new(),
        });
        state.enqueue(QueuedRequest {
            content: "task 2".to_string(),
            request_id: "req.1".to_string(),
            provider: String::new(),
            model: String::new(),
        });

        // First dequeue should work
        let req = state.dequeue();
        assert!(req.is_some());
        assert_eq!(req.unwrap().content, "task 1");

        // Second should be blocked (inflight = 1, sequential mode)
        assert!(state.peek_next().is_none());
    }

    #[test]
    fn complete_decrements_inflight() {
        let mut state = QueueState {
            max_concurrent: 10,
            ..Default::default()
        };

        state.enqueue(QueuedRequest {
            content: "task".to_string(),
            request_id: "req.0".to_string(),
            provider: String::new(),
            model: String::new(),
        });
        state.enqueue(QueuedRequest {
            content: "task2".to_string(),
            request_id: "req.1".to_string(),
            provider: String::new(),
            model: String::new(),
        });

        // Dequeue first task
        let req = state.dequeue();
        assert!(req.is_some());
        assert_eq!(state.inflight, 1);
        assert_eq!(state.len(), 1); // One item left in queue

        state.complete();
        assert_eq!(state.inflight, 0);

        // Now should be able to dequeue again (second task)
        let req2 = state.dequeue();
        assert!(req2.is_some());
        assert_eq!(req2.unwrap().content, "task2");
    }

    #[test]
    fn clear_removes_all() {
        let mut state = QueueState::default();

        for i in 0..5 {
            state.enqueue(QueuedRequest {
                content: format!("task {}", i),
                request_id: format!("req.{}", i),
                provider: String::new(),
                model: String::new(),
            });
        }

        assert_eq!(state.len(), 5);
        state.clear();
        assert!(state.is_empty());
    }

    #[test]
    fn set_mode_changes_behavior() {
        let mut state = QueueState {
            max_concurrent: 10,
            ..Default::default()
        };

        // Add two items
        state.enqueue(QueuedRequest {
            content: "task 1".to_string(),
            request_id: "req.0".to_string(),
            provider: String::new(),
            model: String::new(),
        });
        state.enqueue(QueuedRequest {
            content: "task 2".to_string(),
            request_id: "req.1".to_string(),
            provider: String::new(),
            model: String::new(),
        });

        // Switch to sequential
        state.set_mode(QueueMode::Sequential);

        // Only first should be available
        let req = state.dequeue();
        assert_eq!(req.unwrap().content, "task 1");
        assert!(state.peek_next().is_none());
    }
}
