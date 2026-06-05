//! Update - State Transitions
use crate::model::{AppState, ChatMessage};
use crate::Event;

/// Updates the state based on an event
/// Returns a new state (immutable update)
pub fn update(state: AppState, event: Event) -> AppState {
    match event {
        // === UI Events ===
        Event::Input(c) => state.push_input(c),
        Event::Backspace => state.pop_input(),
        Event::Submit => state.submit(),
        Event::ScrollUp => state.scroll_up(),
        Event::ScrollDown => state.scroll_down(),
        
        // === System Events ===
        Event::Quit => state,
        Event::Reset => AppState::default(),
        
        // === Agent Events ===
        Event::AgentThinking => {
            let mut state = state;
            state.streaming = true;
            state.thinking_started_at = Some(std::time::Instant::now());
            // Add thinking indicator message
            state.messages.push(ChatMessage {
                role: "thinking".into(),
                content: "Thinking...".into(),
            });
            state
        }
        Event::AgentResponse { content } => {
            let mut state = state;
            // Remove thinking message
            state.messages.retain(|m| m.role != "thinking");
            // Append to last assistant message or create new one
            if let Some(last) = state.messages.last_mut() {
                if last.role == "assistant" {
                    last.content.push_str(&content);
                } else {
                    state.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content,
                    });
                }
            } else {
                state.messages.push(ChatMessage {
                    role: "assistant".into(),
                    content,
                });
            }
            state
        }
        Event::AgentDone => {
            let mut state = state;
            state.streaming = false;
            state.thinking_started_at = None;
            // Remove any remaining thinking message
            state.messages.retain(|m| m.role != "thinking");
            state
        }
        Event::AgentError { message } => {
            let mut state = state;
            state.streaming = false;
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content: format!("Error: {}", message),
            });
            state
        }
    }
}

impl AppState {
    /// Add a character to input
    fn push_input(mut self, c: char) -> Self {
        self.input.push(c);
        self
    }
    
    /// Remove last character from input
    fn pop_input(mut self) -> Self {
        self.input.pop();
        self
    }
    
    /// Submit current input - sends to agent channel
    fn submit(self) -> Self {
        if self.input.is_empty() {
            return self;
        }
        
        let content = self.input.trim().to_string();
        let mut state = self;
        state.input.clear();
        
        // Check for /reset command
        if content == "/reset" {
            return AppState::default();
        }
        
        // Add user message
        state.messages.push(ChatMessage {
            role: "user".into(),
            content: content.clone(),
        });
        
        // Add placeholder for agent response (will be updated by agent events)
        state.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
        });
        
        state.streaming = true;
        state
    }
    
    /// Scroll chat up
    fn scroll_up(mut self) -> Self {
        self.scroll = self.scroll.saturating_add(1);
        self
    }
    
    /// Scroll chat down
    fn scroll_down(mut self) -> Self {
        self.scroll = self.scroll.saturating_sub(1);
        self
    }
}
