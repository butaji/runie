use runie_core::Event;
use crate::Task;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubagentHandle {
    pub id: String,
    pub task: Task,
    pub status: SubagentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubagentResult {
    pub handle: SubagentHandle,
    pub events: Vec<Event>,
    pub final_output: String,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}
