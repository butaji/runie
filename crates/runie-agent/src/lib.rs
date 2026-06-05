//! Runie Agent - Agent loop with event-based communication
//! 
//! Works with std threads and channels.

#[cfg(test)]
mod tests;

use runie_core::Event;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

// === Event Types ===

/// Events sent from agent to UI
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Thinking { id: String },
    Response { id: String, content: String },
    Done { id: String },
    Error { id: String, message: String },
}

/// Command to start an agent
#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub content: String,
    pub id: String,
}

// === Message Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
}

pub struct ResponseChunk {
    pub content: String,
}

// === Provider Trait ===

pub trait Provider: Send {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk>;
}

// === Mock Provider ===

#[derive(Default, Clone)]
pub struct MockProvider;

impl Provider for MockProvider {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk> {
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        user_input
            .split_whitespace()
            .map(|word| ResponseChunk {
                content: format!("{} ", word),
            })
            .collect()
    }
}

// === Agent Actor (for tokio, not used in std thread version) ===

#[cfg(feature = "tokio")]
pub mod tokio_actor {
    use super::*;
    use tokio::sync::mpsc as tokio_mpsc;

    /// Agent actor that processes commands from a tokio channel
    pub async fn agent_actor<P>(
        mut cmd_rx: tokio_mpsc::Receiver<AgentCommand>,
        app_tx: tokio_mpsc::Sender<Event>,
        provider: P,
    ) where
        P: Provider + Clone + Send + Sync + 'static,
    {
        while let Some(cmd) = cmd_rx.recv().await {
            run_agent_async(&provider, cmd, &app_tx).await;
        }
    }

    async fn run_agent_async<P>(
        provider: &P,
        cmd: AgentCommand,
        app_tx: &tokio_mpsc::Sender<Event>,
    ) where
        P: Provider,
    {
        // Random delay for manual UI testing (skip in tests)
        if std::env::var("RUNIE_TEST").is_err() {
            let delay_ms = 500 + (rand_u32() % 2500);
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms as u64)).await;
        }

        // Send thinking
        let _ = app_tx
            .send(Event::AgentThinking { id: cmd.id.clone() })
            .await;

        // Get response chunks
        let messages = vec![Message::User { content: cmd.content }];
        let chunks = provider.generate(messages);

        // Send each chunk
        for chunk in chunks {
            let _ = app_tx
                .send(Event::AgentResponse {
                    id: cmd.id.clone(),
                    content: chunk.content,
                })
                .await;

            if std::env::var("RUNIE_TEST").is_err() {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }

        // Done
        let _ = app_tx.send(Event::AgentDone { id: cmd.id }).await;
    }

    fn rand_u32() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u32
    }
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u32
}
