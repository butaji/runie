use crate::events::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub created_at: String,
    pub cwd: String,
    pub path: String,
    pub parent_session_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionTreeEntry {
    Message { id: String, parent_id: Option<String>, timestamp: String, message: AgentMessage },
    Compaction { id: String, parent_id: Option<String>, timestamp: String, summary: String, first_kept_entry_id: String, tokens_before: u32 },
    BranchSummary { id: String, parent_id: Option<String>, timestamp: String, from_id: String, summary: String },
    Label { id: String, parent_id: Option<String>, timestamp: String, target_id: String, label: Option<String> },
    Leaf { id: String, parent_id: Option<String>, timestamp: String, target_id: Option<String> },
}

#[derive(Debug, Clone)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u32,
    pub keep_recent_tokens: u32,
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reserve_tokens: 4000,
            keep_recent_tokens: 2000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentHarnessOptions {
    pub cwd: String,
    pub model: String,
    pub system_prompt: String,
    pub compaction_settings: CompactionSettings,
}