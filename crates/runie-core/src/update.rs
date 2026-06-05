//! Update - State Transitions
use crate::labels::thought_with_time;
use crate::model::{AppState, ChatMessage};
use crate::Event;

/// Get current timestamp in seconds
fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

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
            state.thought_duration_secs = None; // Reset
            state
        }
        Event::AgentResponse { content } => {
            let mut state = state;
            // Add thought message on first response (before assistant)
            if state.thinking_started_at.is_some() && state.thought_duration_secs.is_none() {
                let duration = state.thinking_elapsed_secs().unwrap_or(0.0);
                state.messages.push(ChatMessage {
                    role: "thought".into(),
                    content: thought_with_time(duration),
                    timestamp: now(),
                });
                state.thought_duration_secs = Some(duration);
            }
            // Add assistant message
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content,
                timestamp: now(),
            });
            state
        }
        Event::AgentDone => {
            let mut state = state;
            state.streaming = false;
            state.thinking_started_at = None;
            state
        }
        Event::AgentError { message } => {
            let mut state = state;
            state.streaming = false;
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content: format!("Error: {}", message),
                timestamp: now(),
            });
            state
        }
        // === Internal Events ===
        Event::SpawnAgent => state,
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
    
    /// Submit current input - adds to queue
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
            timestamp: now(),
        });
        
        // Add to queue
        state.request_queue.push(content);
        
        // Set streaming if not already
        state.streaming = true;
        state
    }
    
    /// Get next request from queue without removing
    pub fn peek_queue(&self) -> Option<String> {
        self.request_queue.first().cloned()
    }
    
    /// Remove first item from queue
    pub fn pop_queue(&mut self) -> Option<String> {
        if !self.request_queue.is_empty() {
            Some(self.request_queue.remove(0))
        } else {
            None
        }
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
