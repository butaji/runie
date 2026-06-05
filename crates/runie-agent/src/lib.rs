//! Runie Agent - Agent loop with mock provider
//! 
//! The agent processes messages and returns responses via a provider.

/// Message types for agent communication
#[derive(Debug, Clone)]
pub enum Message {
    System { content: String },
    User { content: String },
    Assistant { content: String },
}

/// Response from provider
#[derive(Debug, Clone)]
pub struct Response {
    pub content: String,
}

/// Provider trait - implemented by LLM backends
pub trait Provider {
    fn generate(&self, messages: Vec<Message>) -> Response;
}

/// Mock provider - echoes back user messages
#[derive(Default)]
pub struct MockProvider;

impl Provider for MockProvider {
    fn generate(&self, messages: Vec<Message>) -> Response {
        // Get last user message
        let user_input = messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::User { content } => Some(content.clone()),
                _ => None,
            })
            .unwrap_or_default();

        Response {
            content: format!("Echo: {}", user_input),
        }
    }
}

/// Run the agent loop with a provider
pub fn run_agent<P: Provider>(provider: &P, messages: Vec<Message>) -> Response {
    provider.generate(messages)
}
