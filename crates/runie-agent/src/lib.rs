//! Runie Agent - Agent loop with event-based communication
//! 
//! The agent processes messages and sends responses via events to a channel.

use runie_core::Event;
use serde::{Deserialize, Serialize};

/// Message types for agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
}

/// Response chunk from provider
#[derive(Debug, Clone)]
pub struct ResponseChunk {
    pub content: String,
}

/// Provider trait - implemented by LLM backends
pub trait Provider {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk>;
}

/// Mock provider - echoes back user messages word by word
#[derive(Default)]
pub struct MockProvider;

impl Provider for MockProvider {
    fn generate(&self, messages: Vec<Message>) -> Vec<ResponseChunk> {
        // Random delay between 0.5 and 3 seconds
        let delay_ms = 500 + (rand_u32() % 2500);
        std::thread::sleep(std::time::Duration::from_millis(delay_ms as u64));
        
        // Get last user message
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        // Echo word by word for streaming effect
        let echo = format!("Echo: {}", user_input);
        echo.split_whitespace()
            .scan(String::new(), |acc, word| {
                if acc.is_empty() {
                    *acc = word.to_string();
                } else {
                    *acc = format!("{} {}", acc, word);
                }
                Some(ResponseChunk {
                    content: format!("{} ", word),
                })
            })
            .collect()
    }
}

/// Simple random u32 using system time
fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u32
}

/// Run the agent with a provider and send events to the channel
pub fn run_agent<P, F>(provider: &P, messages: Vec<Message>, send_event: F)
where
    P: Provider,
    F: Fn(Event),
{
    // Simulate thinking
    send_event(Event::AgentThinking);
    
    // Get response chunks
    let chunks = provider.generate(messages);
    
    // Send each chunk
    for chunk in chunks {
        send_event(Event::AgentResponse {
            content: chunk.content,
        });
        // Small delay for streaming effect
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    // Done
    send_event(Event::AgentDone);
}
