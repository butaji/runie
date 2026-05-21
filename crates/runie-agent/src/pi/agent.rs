use crate::events::*;
use crate::pi::AgentState;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

pub type EventListener = Box<dyn Fn(AgentEvent) + Send + Sync>;

pub struct Agent {
    state: Arc<Mutex<AgentState>>,
    listeners: Vec<EventListener>,
    steering_queue: Arc<Mutex<VecDeque<AgentMessage>>>,
    follow_up_queue: Arc<Mutex<VecDeque<AgentMessage>>>,
}

impl Agent {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AgentState::default())),
            listeners: Vec::new(),
            steering_queue: Arc::new(Mutex::new(VecDeque::new())),
            follow_up_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn subscribe(&mut self, listener: EventListener) {
        self.listeners.push(listener);
    }

    pub fn steer(&self, message: AgentMessage) {
        self.steering_queue.lock().unwrap().push_back(message);
    }

    pub fn follow_up(&self, message: AgentMessage) {
        self.follow_up_queue.lock().unwrap().push_back(message);
    }

    pub fn state(&self) -> AgentState {
        self.state.lock().unwrap().clone()
    }

    pub fn prompt(&self, message: impl Into<AgentMessage>) {
        // Start agent loop
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}
