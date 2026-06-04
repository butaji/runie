use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    /// Simple, non-streaming message for pre-agent UI messages (e.g., onboarding/welcome).
    Message { role: String, content: String },
    
    MessageStart { 
        message: AgentMessage,
        turn: usize,
    },
    
    /// Streaming message update. If `replace` is true, `delta` is the
    /// absolute content (used by tests and for re-snapshots). If false,
    /// `delta` is appended to the last assistant message.
    MessageUpdate {
        message: AgentMessage,
        delta: String,
        /// When true, replace last assistant text with `delta` (test snapshot mode).
        /// When false (default for live streaming), append `delta` to last assistant.
        replace: bool,
        turn: usize,
    },
    
    MessageEnd { 
        message: AgentMessage,
        turn: usize,
    },
    
    /// Agent has started reasoning/thinking (e.g., extended thinking, reasoning models).
    /// The agent is responsible for detecting thinking patterns, not the TUI.
    ThinkingStart {
        turn: usize,
    },
    
    /// Agent reasoning/thinking update with NEW text since the last update.
    /// Consumer should `push_str(&delta)` rather than overwrite.
    ThinkingUpdate {
        /// New characters accumulated since last update; full snapshot = sum of all deltas.
        delta: String,
        /// Total accumulated length so far (consumer can ignore this if it tracks length itself).
        total_len: usize,
        turn: usize,
    },
    
    /// Agent reasoning/thinking has ended.
    ThinkingEnd {
        duration_ms: u64,
        turn: usize,
    },
    
    ToolExecutionStart { 
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
        turn: usize,
    },
    
    ToolExecutionEnd { 
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
        result: ToolResult,
        duration_ms: u64,
        turn: usize,
    },
    
    TurnEnd {
        turn: usize,
        message_count: usize,
        tool_results_count: usize,
        token_usage: TokenUsage,
        /// Optional explicit total turn duration in milliseconds.
        /// When provided, this overrides the wall-clock-derived
        /// `elapsed` value in the "Turn completed in Xs." footer —
        /// useful for tests/replays where wall-clock is ~0 and the
        /// scenario author wants to specify a precise duration.
        turn_duration_ms: Option<u64>,
    },
    
    AgentEnd { 
        messages: Vec<AgentMessage>,
        total_turns: usize,
        final_token_usage: TokenUsage,
    },
    
    TokenUsage { 
        prompt_tokens: usize, 
        completion_tokens: usize, 
        total_tokens: usize,
        context_window: usize,
    },
    
    Error { 
        message: String,
        error_type: String,
        recoverable: bool,
        context: String,
    },
    
    /// Notification that context was compacted/summarized
    ContextCompacted {
        original_count: usize,
        compacted_count: usize,
        summary_preview: String,
    },
    
    PermissionRequest { 
        tool_call_id: String, 
        tool_name: String, 
        tool_args: String,
        tool_description: String,
        turn: usize,
        context_window_usage: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    Allow { 
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
    },
    Deny { 
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
    },
    AllowAlways { 
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
    },
    Skip { 
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
    },
}

impl Display for PermissionDecision {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PermissionDecision::Allow { tool_name, .. } => {
                write!(f, "Allow({})", tool_name)
            }
            PermissionDecision::Deny { tool_name, .. } => {
                write!(f, "Deny({})", tool_name)
            }
            PermissionDecision::AllowAlways { tool_name, .. } => {
                write!(f, "AllowAlways({})", tool_name)
            }
            PermissionDecision::Skip { tool_name, .. } => {
                write!(f, "Skip({})", tool_name)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String, // "user", "assistant", "tool", "system"
    pub content: Vec<ContentPart>,
    pub timestamp: i64,
    pub usage: Option<TokenUsage>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
    pub tool_calls: Vec<runie_core::ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: Vec<ContentPart>, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    pub media_type: String,
    pub data: String, // base64
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub cache_read: u32,
    pub cache_write: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub content: Vec<ContentPart>,
    pub is_error: bool,
}
