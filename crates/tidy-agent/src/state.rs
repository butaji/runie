use std::collections::VecDeque;
use tidy_core::{Session, WorkingMemory, Message};

#[derive(Debug, Clone)]
pub struct AgentState {
    pub session: Session,
    pub working_memory: WorkingMemory,
    pub turn_count: usize,
    pub steering_queue: VecDeque<String>,
    pub follow_up_queue: VecDeque<String>,
    pub is_running: bool,
}

impl AgentState {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            working_memory: WorkingMemory::default(),
            turn_count: 0,
            steering_queue: VecDeque::new(),
            follow_up_queue: VecDeque::new(),
            is_running: false,
        }
    }

    pub fn queue_steering(&mut self, message: String) {
        self.steering_queue.push_back(message);
    }

    pub fn queue_follow_up(&mut self, message: String) {
        self.follow_up_queue.push_back(message);
    }

    pub fn add_message(&mut self, parent_id: Option<String>, message: Message) -> String {
        self.session.add_message(parent_id, message)
    }
}
