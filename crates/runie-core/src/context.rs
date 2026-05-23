use serde::{Deserialize, Serialize};
use crate::{Session, ToolSchema};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Context {
    pub session: Session,
    pub working_memory: WorkingMemory,
    pub system_prompt: String,
    pub tool_schemas: Vec<ToolSchema>,
    pub max_tokens: usize,
    pub temperature: f32,
}

impl Context {
    pub fn new() -> Self {
        Self {
            session: Session::new(uuid::Uuid::new_v4().to_string()),
            working_memory: WorkingMemory::default(),
            system_prompt: String::new(),
            tool_schemas: Vec::new(),
            max_tokens: 8192,
            temperature: 0.7,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            session: Session::new(String::new()),
            working_memory: WorkingMemory::default(),
            system_prompt: String::new(),
            tool_schemas: Vec::new(),
            max_tokens: 8192,
            temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkingMemory {
    pub current_task: String,
    pub key_files: Vec<String>,
    pub recent_notes: Vec<String>,
    pub custom_data: serde_json::Value,
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self {
            current_task: String::new(),
            key_files: Vec::new(),
            recent_notes: Vec::new(),
            custom_data: serde_json::Value::Null,
        }
    }
}
