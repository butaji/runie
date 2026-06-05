use runie_core::{Session, WorkingMemory, Message};

#[derive(Debug, Clone)]
pub struct AgentState {
    pub session: Session,
    pub working_memory: WorkingMemory,
    pub turn_count: usize,
}

impl AgentState {

    #[must_use]
    pub fn new(session: Session) -> Self {
        Self {
            session,
            working_memory: WorkingMemory::default(),
            turn_count: 0,
        }
    }

    pub fn add_message(&mut self, parent_id: Option<String>, message: Message) -> String {
        self.session.add_message(parent_id, message)
    }
}
