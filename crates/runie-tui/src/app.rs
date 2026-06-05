use runie_agent::types::AgentEvent;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub struct App {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub scroll: usize,
    pub streaming: bool,
    pub stream_buffer: String,
    pub quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            messages: vec![ChatMessage {
                role: "system".into(),
                content: "Welcome! Type a message and press Enter.".into(),
            }],
            input: String::new(),
            scroll: 0,
            streaming: false,
            stream_buffer: String::new(),
            quit: false,
        }
    }

    pub fn push_user_message(&mut self) {
        if !self.input.is_empty() {
            self.messages.push(ChatMessage {
                role: "user".into(),
                content: self.input.clone(),
            });
            self.input.clear();
            self.scroll = self.messages.len().saturating_sub(1);
        }
    }

    pub fn handle_event(&mut self, event: &AgentEvent) {
        match event {
            AgentEvent::MessageStart { role } => {
                self.streaming = true;
                self.stream_buffer.clear();
                self.messages.push(ChatMessage {
                    role: role.clone(),
                    content: String::new(),
                });
            }
            AgentEvent::MessageDelta { content } => {
                self.stream_buffer.push_str(content);
                if let Some(last) = self.messages.last_mut() {
                    last.content = self.stream_buffer.clone();
                }
            }
            AgentEvent::MessageEnd => {
                self.streaming = false;
                self.stream_buffer.clear();
                self.scroll = self.messages.len().saturating_sub(1);
            }
            AgentEvent::ToolCallStart { id, name } => {
                self.messages.push(ChatMessage {
                    role: "tool".into(),
                    content: format!("Calling tool: {} ({})", name, id),
                });
            }
            AgentEvent::ToolCallEnd { id, result } => {
                self.messages.push(ChatMessage {
                    role: "tool_result".into(),
                    content: format!("Result for {}: {}", id, result),
                });
            }
            AgentEvent::Error { message } => {
                self.messages.push(ChatMessage {
                    role: "error".into(),
                    content: format!("Error: {}", message),
                });
                self.streaming = false;
            }
        }
    }
}
