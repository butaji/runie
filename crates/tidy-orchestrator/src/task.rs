use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub max_turns: usize,
    pub read_only: bool,
    pub allowed_tools: Vec<String>,
    pub parent_session_id: Option<String>,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: String::new(),
            max_turns: 25,
            read_only: false,
            allowed_tools: Vec::new(),
            parent_session_id: None,
            priority: TaskPriority::Medium,
        }
    }
}
