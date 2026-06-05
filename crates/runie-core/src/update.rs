//! Update - State Transitions
use crate::labels::thought_with_time;
use crate::model::{AppState, ChatMessage};
use crate::Event;

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

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
            state.current_request_id = Some(id.clone());
            state.thinking_started_at = Some(std::time::Instant::now());
            state.thought_duration_secs = None;
            // Start turn timer if not already started
            if state.turn_started_at.is_none() {
                state.turn_started_at = Some(std::time::Instant::now());
            }
            state
        }
        Event::AgentResponse { id, content } => {
            let mut state = state;
            // Check if this request already has a thought (for tool flows with multiple thinking phases)
            let has_existing_thought = state.messages.iter().any(|m| 
                m.role == "thought" && m.id == id
            );
            
            // Add thought message on first response if not already created
            if state.thinking_started_at.is_some() 
                && state.thought_duration_secs.is_none() 
                && !has_existing_thought 
            {
                let duration = state.thinking_elapsed_secs().unwrap_or(0.0);
                state.messages.push(ChatMessage {
                    role: "thought".into(),
                    content: thought_with_time(duration),
                    timestamp: now(),
                    id: id.clone(),
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
            state.current_request_id = Some(id);
            state
        }
        Event::AgentToolStart { id, name } => {
            let mut state = state;
            state.current_request_id = Some(id.clone());
            state.messages.push(ChatMessage {
                role: "tool".into(),
                content: format!("🔧 Running {}...", name),
                timestamp: now(),
                id: id.clone(),
            });
            state
        }
        Event::AgentToolEnd { id, name: _, output } => {
            let mut state = state;
            state.messages.push(ChatMessage {
                role: "tool".into(),
                content: output,
                timestamp: now(),
                id: id.clone(),
            });
            state.current_request_id = Some(id);
            state
        }
        Event::AgentTurnComplete { id, duration_secs } => {
            let mut state = state;
            state.messages.push(ChatMessage {
                role: "turn_complete".into(),
                content: format!("✓ Turn completed in {:.1}s", duration_secs),
                timestamp: now(),
                id: id.clone(),
            });
            state.current_request_id = Some(id);
            state.turn_started_at = None;
            state
        }
        Event::AgentDone { id: _ } => {
            let mut state = state;
            state.current_request_id = None;
            if state.request_queue.is_empty() {
                state.streaming = false;
                state.thinking_started_at = None;
                state.thought_duration_secs = None;
            } else {
                state.streaming = true;
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
        Event::SpawnAgent => state,
    }
}

impl AppState {
    fn push_input(mut self, c: char) -> Self {
        self.input.push(c);
        self
    }
    
    fn pop_input(mut self) -> Self {
        self.input.pop();
        self
    }
    
    fn submit(self) -> Self {
        if self.input.is_empty() {
            return self;
        }
        
        let content = self.input.trim().to_string();
        let mut state = self;
        state.input.clear();
        
        if content == "/reset" {
            return AppState::default();
        }
        
        let id = state.next_id();
        
        state.messages.push(ChatMessage {
            role: "user".into(),
            content: content.clone(),
            timestamp: now(),
            id: id.clone(),
        });
        
        state.request_queue.push((content, id));
        state
    }
    
    pub fn peek_queue(&self) -> Option<(String, String)> {
        self.request_queue.first().cloned()
    }
    
    pub fn pop_queue(&mut self) -> Option<(String, String)> {
        if !self.request_queue.is_empty() {
            Some(self.request_queue.remove(0))
        } else {
            None
        }
    }
    
    fn scroll_up(mut self) -> Self {
        self.scroll = self.scroll.saturating_add(1);
        self
    }
    
    fn scroll_down(mut self) -> Self {
        self.scroll = self.scroll.saturating_sub(1);
        self
    }
}
