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
        Event::AgentThinking { id } => {
            let mut state = state;
            state.streaming = true;
            state.current_request_id = Some(id);
            state.thinking_started_at = Some(std::time::Instant::now());
            state.thought_duration_secs = None;
            state
        }
        Event::AgentResponse { id, content } => {
            let mut state = state;
            // Add thought message on first response (before assistant)
            if state.thinking_started_at.is_some() && state.thought_duration_secs.is_none() {
                let duration = state.thinking_elapsed_secs().unwrap_or(0.0);
                state.messages.push(ChatMessage {
                    role: "thought".into(),
                    content: thought_with_time(duration),
                    timestamp: now(),
                    id: id.clone(),  // Same ID for grouping
                });
                state.thought_duration_secs = Some(duration);
            }
            // Add assistant message
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content,
                timestamp: now(),
                id: id.clone(),
            });
            // Track current request
            state.current_request_id = Some(id);
            state
        }
        Event::AgentDone { id: _ } => {
            let mut state = state;
            state.current_request_id = None;
            // Keep streaming if queue has more requests
            if state.request_queue.is_empty() {
                state.streaming = false;
                state.thinking_started_at = None;
                state.thought_duration_secs = None;
            } else {
                state.streaming = true;
                // Keep thinking_started_at to allow next response to create thought
                state.thought_duration_secs = None;
            }
            state
        }
        Event::AgentError { id, message } => {
            let mut state = state;
            state.streaming = false;
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content: format!("Error: {}", message),
                timestamp: now(),
                id: format!("error.{}", id),
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
    
    /// Submit current input - adds to queue with ID
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
        
        // Generate ID
        let id = state.next_id();
        
        // Add user message with ID
        state.messages.push(ChatMessage {
            role: "user".into(),
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        
        // Add to queue with ID
        state.request_queue.push((content, id));
        // Don't set streaming=true here - it gets set when agent actually starts
        state
    }
    
    /// Get next request from queue without removing
    pub fn peek_queue(&self) -> Option<(String, String)> {
        self.request_queue.first().cloned()
    }
    
    /// Remove first item from queue
    pub fn pop_queue(&mut self) -> Option<(String, String)> {
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
