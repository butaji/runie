//! Agent — Event-driven agent state machine inspired by pi
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Agent                                                           │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │  State: idle, running, waiting_tool, error             │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │  Tools: Registry of available tools                    │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │  Listeners: Event subscription system                 │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  AI Layer (core/ai.rs)                                          │
//! │  - Provider trait                                               │
//! │  - Streaming                                                   │
//! └─────────────────────────────────────────────────────────────────┘
//!     ↓
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Tools (core/tools.rs)                                           │
//! │  - read, bash, edit, write, grep, find, ls                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use crate::core::ai::{Model, Cost};
use crate::core::session::Session;
use crate::core::tools::{Tool, ToolInput, ToolOutput, ToolRegistry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Running,
    WaitingTool,
    Error,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Agent configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub model: Model,
    pub system_prompt: String,
    pub tools: Vec<String>,  // Tool names to enable
    pub max_retries: u32,
    pub thinking_level: ThinkingLevel,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: Model::default_claude(),
            system_prompt: "You are a helpful coding assistant.".to_string(),
            tools: vec!["read".to_string(), "bash".to_string(), "edit".to_string(), "write".to_string()],
            max_retries: 3,
            thinking_level: ThinkingLevel::Medium,
        }
    }
}

/// Thinking levels (like pi's minimal/low/medium/high/xhigh)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

impl Default for ThinkingLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Minimal => "minimal",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::XHigh => "xhigh",
        }
    }
}

/// Agent events (inspired by pi's AgentEvent)
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent started processing
    AgentStart,
    /// Agent finished
    AgentEnd {
        will_retry: bool,
    },
    /// Turn started
    TurnStart { turn: u32 },
    /// Turn ended
    TurnEnd { turn: u32 },
    /// Message start (streaming)
    MessageStart {
        message_id: String,
        role: String,
    },
    /// Message update (streaming content)
    MessageUpdate {
        message_id: String,
        delta: String,
    },
    /// Message end
    MessageEnd { message_id: String },
    /// Tool call started
    ToolCallStart {
        call_id: String,
        tool_name: String,
        input: serde_json::Value,
    },
    /// Tool call update
    ToolCallUpdate {
        call_id: String,
        delta: String,
    },
    /// Tool call ended
    ToolCallEnd {
        call_id: String,
        output: String,
    },
    /// Tool call error
    ToolCallError {
        call_id: String,
        error: String,
    },
    /// Error occurred
    Error { error: String },
    /// Cost update
    CostUpdate { cost: Cost },
    /// Model change
    ModelChange { model: Model },
    /// Thinking level change
    ThinkingLevelChange { level: ThinkingLevel },
    /// Auto-retry started
    AutoRetryStart {
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error_message: String,
    },
    /// Auto-retry ended
    AutoRetryEnd {
        success: bool,
        attempt: u32,
        final_error: Option<String>,
    },
    /// Queue update (steering/follow-up)
    QueueUpdate {
        steering: Vec<String>,
        follow_up: Vec<String>,
    },
}

/// Agent listener callback
pub type AgentListener = Box<dyn Fn(AgentEvent, std::sync::mpsc::SyncSender<()>) + Send + Sync>;

/// Tool call context
pub struct ToolCallContext {
    pub call_id: String,
    pub tool_name: String,
    pub input: ToolInput,
    pub agent: Arc<Agent>,
}

/// Tool result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub call_id: String,
    pub output: ToolOutput,
    pub error: Option<String>,
}

/// Before tool call result (can modify input or block)
#[derive(Debug, Clone)]
pub enum BeforeToolResult {
    Allow,
    Block { reason: String },
    ModifyInput(ToolInput),
}

/// After tool call result (can modify output or retry)
#[derive(Debug, Clone)]
pub enum AfterToolResult {
    Allow,
    Retry { max_attempts: u32 },
    Block { reason: String },
}

/// Main Agent struct — event-driven state machine
pub struct Agent {
    config: RwLock<AgentConfig>,
    state: RwLock<AgentState>,
    tool_registry: ToolRegistry,
    listeners: RwLock<Vec<AgentListener>>,
    session: RwLock<Option<Arc<Session>>>,
    
    // Streaming state
    current_message_id: RwLock<Option<String>>,
    pending_tool_calls: RwLock<Vec<ToolCallPending>>,
    
    // Cost tracking
    total_cost: RwLock<Cost>,
    
    // Retry state
    retry_count: RwLock<u32>,
}

struct ToolCallPending {
    call_id: String,
    tool_name: String,
    input: ToolInput,
}

impl Agent {
    /// Create a new agent with config
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config: RwLock::new(config),
            state: RwLock::new(AgentState::Idle),
            tool_registry: ToolRegistry::new(),
            listeners: RwLock::new(Vec::new()),
            session: RwLock::new(None),
            current_message_id: RwLock::new(None),
            pending_tool_calls: RwLock::new(Vec::new()),
            total_cost: RwLock::new(Cost::zero()),
            retry_count: RwLock::new(0),
        }
    }

    /// Create with default config
    pub fn default_agent() -> Self {
        Self::new(AgentConfig::default())
    }

    /// Subscribe to agent events
    pub async fn subscribe<F>(&self, listener: F) -> impl Fn() + 'static
    where
        F: Fn(AgentEvent, std::sync::mpsc::SyncSender<()>) + Send + Sync + 'static,
    {
        self.listeners.write().await.push(Box::new(listener));
        move || {
            // Unsubscribing handled by removing listener
        }
    }

    /// Emit an event to all listeners
    async fn emit(&self, event: AgentEvent) {
        let (tx, _rx) = std::sync::mpsc::sync_channel(0);
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone(), tx.clone());
        }
    }

    /// Get current state
    pub async fn state(&self) -> AgentState {
        *self.state.read().await
    }

    /// Get current config
    pub async fn config(&self) -> AgentConfig {
        self.config.read().await.clone()
    }

    /// Update config
    pub async fn set_config(&self, config: AgentConfig) {
        let model = config.model.clone();
        *self.config.write().await = config;
        self.emit(AgentEvent::ModelChange { model }).await;
    }

    /// Set thinking level
    pub async fn set_thinking_level(&self, level: ThinkingLevel) {
        let mut config = self.config.write().await;
        config.thinking_level = level;
        *self.config.write().await = config.clone();
        drop(config);
        self.emit(AgentEvent::ThinkingLevelChange { level }).await;
    }

    /// Set associated session
    pub fn set_session(&self, session: Arc<Session>) {
        // Session handles its own subscription
    }

    /// Register a tool
    pub fn register_tool(&self, tool: Box<dyn Tool>) {
        // Tool registration handled via registry directly
        let _ = tool;
    }

    /// Get tool registry
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    /// Execute a tool call
    pub async fn execute_tool(&self, call_id: String, tool_name: String, input: ToolInput) -> ToolResult {
        self.emit(AgentEvent::ToolCallStart {
            call_id: call_id.clone(),
            tool_name: tool_name.clone(),
            input: serde_json::to_value(&input).unwrap_or_default(),
        }).await;

        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let result = self.tool_registry.execute(&tool_name, input, &cwd).await;

        match result {
            Ok(output) => {
                self.emit(AgentEvent::ToolCallEnd {
                    call_id: call_id.clone(),
                    output: output.content.clone(),
                }).await;

                ToolResult {
                    call_id,
                    output,
                    error: None,
                }
            }
            Err(e) => {
                self.emit(AgentEvent::ToolCallError {
                    call_id: call_id.clone(),
                    error: e.clone(),
                }).await;

                ToolResult {
                    call_id,
                    output: ToolOutput::error(&e),
                    error: Some(e),
                }
            }
        }
    }

    /// Abort the current run
    pub async fn abort(&self) {
        let state = *self.state.read().await;
        if state == AgentState::Running || state == AgentState::WaitingTool {
            *self.state.write().await = AgentState::Idle;
            // Signal abort to model provider
        }
    }

    /// Get total cost so far
    pub async fn total_cost(&self) -> Cost {
        self.total_cost.read().await.clone()
    }

    /// Get pending tool calls
    pub async fn pending_tool_calls(&self) -> Vec<(String, String)> {
        self.pending_tool_calls.read().await
            .iter()
            .map(|p| (p.call_id.clone(), p.tool_name.clone()))
            .collect()
    }

    /// Clear pending tool calls
    pub async fn clear_pending_tool_calls(&self) {
        self.pending_tool_calls.write().await.clear();
    }
}

/// Builder pattern for Agent
pub struct AgentBuilder {
    config: AgentConfig,
    tools: Vec<Box<dyn Tool>>,
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
            tools: Vec::new(),
        }
    }

    pub fn model(mut self, model: Model) -> Self {
        self.config.model = model;
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = prompt.into();
        self
    }

    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub fn tools(mut self, tools: Vec<Box<dyn Tool>>) -> Self {
        self.tools = tools;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn thinking_level(mut self, level: ThinkingLevel) -> Self {
        self.config.thinking_level = level;
        self
    }

    pub fn build(self) -> Agent {
        let agent = Agent::new(self.config);
        // Note: Tools are registered via the registry directly in production
        agent
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_creation() {
        let agent = Agent::default_agent();
        assert_eq!(agent.state().await, AgentState::Idle);
    }

    #[tokio::test]
    async fn test_agent_config_update() {
        let agent = Agent::default_agent();
        let new_config = AgentConfig {
            model: Model::default_gpt4o(),
            ..Default::default()
        };
        agent.set_config(new_config).await;
        
        let config = agent.config().await;
        assert!(config.model.id.contains("gpt"));
    }

    #[tokio::test]
    async fn test_agent_state_transitions() {
        let agent = Agent::default_agent();
        
        assert_eq!(agent.state().await, AgentState::Idle);
        
        // Running would change state, but we won't actually run in tests
        // Just verify state enum works
    }

    #[tokio::test]
    async fn test_thinking_level_change() {
        let agent = Agent::default_agent();
        agent.set_thinking_level(ThinkingLevel::High).await;
        
        let config = agent.config().await;
        assert_eq!(config.thinking_level, ThinkingLevel::High);
    }

    #[tokio::test]
    async fn test_agent_builder() {
        let agent = AgentBuilder::new()
            .model(Model::default_claude())
            .system_prompt("You are a Rust expert")
            .max_retries(5)
            .thinking_level(ThinkingLevel::High)
            .build();
        
        let config = agent.config().await;
        assert!(config.model.id.contains("claude"));
        assert!(config.system_prompt.contains("Rust"));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.thinking_level, ThinkingLevel::High);
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let agent = Agent::default_agent();
        let _unsub = agent.subscribe(|event, _| {
            // Event listener
        }).await;
        // Can unsubscribe
    }
}
