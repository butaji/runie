//! Agent loop engine.

pub mod context;
pub mod permissions;
pub mod streaming;
pub mod tools;
pub mod run;

mod tests;

// Re-export for tests (only needed in test builds)
#[cfg(test)]
pub(crate) use streaming::start_chat_with_retry;
pub use run::run_agent_loop;

use crate::config::AgentConfig;
use crate::events::{AgentEvent, AgentMessage, PermissionDecision};
use crate::tools::AgentTool;
use crate::Hook;
use runie_ai::Provider;
use runie_tools::ToolRegistry;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use tokio::sync::{mpsc, Mutex};

/// Calculate estimated context window usage as a percentage.
pub(crate) fn calculate_context_window_usage(messages: &[AgentMessage], context_window: usize) -> f32 {
    let total_chars: usize = messages.iter()
        .map(|m| context::format_message_content(&m.content, &m.tool_calls).len())
        .sum();
    let estimated_tokens = total_chars / 4;
    if context_window > 0 {
        (estimated_tokens as f32 / context_window as f32) * 100.0
    } else {
        0.0
    }
}

#[derive(Debug, Clone)]
pub enum AgentLoopError {
    ProviderError(String),
    ToolError(String),
    SendError(String),
    MaxTurnsExceeded,
}

impl std::fmt::Display for AgentLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLoopError::ProviderError(s) => write!(f, "Provider error: {}", s),
            AgentLoopError::ToolError(s) => write!(f, "Tool error: {}", s),
            AgentLoopError::SendError(s) => write!(f, "Send error: {}", s),
            AgentLoopError::MaxTurnsExceeded => write!(f, "Max turns exceeded"),
        }
    }
}

/// Classify an error into type and recoverability.
pub(crate) fn classify_error(error: &AgentLoopError) -> (String, bool, String) {
    match error {
        AgentLoopError::ProviderError(msg) => (
            "provider".to_string(),
            true,
            format!("Provider error: {}", msg),
        ),
        AgentLoopError::ToolError(msg) => (
            "tool".to_string(),
            true,
            format!("Tool error: {}", msg),
        ),
        AgentLoopError::SendError(msg) => (
            "send".to_string(),
            true,
            format!("Send error: {}", msg),
        ),
        AgentLoopError::MaxTurnsExceeded => (
            "max_turns".to_string(),
            false,
            "Maximum number of turns exceeded".to_string(),
        ),
    }
}

pub struct AgentLoopConfig {
    pub system_prompt: String,
    pub model: String,
    pub thinking_level: String,
    pub max_turns: usize,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            model: String::new(),
            thinking_level: String::new(),
            max_turns: AgentConfig::default().max_turns,
        }
    }
}

/// Event stream for consuming agent loop events and sending permission decisions.
pub struct AgentEventStream {
    rx: mpsc::Receiver<AgentEvent>,
    perm_tx: mpsc::Sender<PermissionDecision>,
    result: Option<Vec<AgentMessage>>,
}

impl AgentEventStream {
    /// Send a permission decision back to the agent loop.
    pub async fn send_permission(
        &self,
        decision: PermissionDecision,
    ) -> Result<(), mpsc::error::SendError<PermissionDecision>> {
        self.perm_tx.send(decision).await
    }

    /// Consume the stream and collect the final result (messages from AgentEnd event).
    pub async fn result(mut self) -> Vec<AgentMessage> {
        while let Ok(event) = self.rx.try_recv() {
            if let AgentEvent::AgentEnd { messages, .. } = event {
                return messages;
            }
        }
        self.result.unwrap_or_default()
    }
}

impl futures::Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Convenience wrapper that runs the agent loop and returns an event stream.
pub fn agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> AgentEventStream {
    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(128);
    let (perm_tx, perm_rx) = mpsc::channel::<PermissionDecision>(1);
    let permission_state = Arc::new(Mutex::new(None));
    let permission_state_clone = permission_state.clone();

    tokio::spawn(async move {
        let mut perm_rx = perm_rx;
        while let Some(decision) = perm_rx.recv().await {
            let mut state = permission_state_clone.lock().await;
            *state = Some(decision);
        }
    });

    tokio::spawn(async move {
        let _ = run_agent_loop(
            initial_messages,
            config,
            provider,
            tools,
            event_tx,
            permission_state,
            registry,
            hooks,
        ).await;
    });

    AgentEventStream {
        rx: event_rx,
        perm_tx,
        result: None,
    }
}
