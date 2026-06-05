//! Update - State Transitions
use crate::labels::{thought_with_time, tool_running, tool_done};
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
            state.turn_active = true;
            state.current_action = Some("Thinking".to_string());
            
            if state.turn_started_at.is_none() {
                state.turn_started_at = Some(std::time::Instant::now());
            }
            state
        }
        Event::AgentThoughtDone { id } => {
            let mut state = state;
            state.current_action = None;  // Clear action when thought done
            state.thinking_started_at = None;
            let mut state = state;
            let duration = state.thinking_elapsed_secs().unwrap_or(0.0);
            state.thinking_started_at = None;
            
            state.messages.push(ChatMessage {
                role: "thought".into(),
                content: thought_with_time(duration),
                timestamp: now(),
                id,
            });
            state
        }
        Event::AgentToolStart { id, name } => {
            let mut state = state;
            state.current_request_id = Some(id.clone());
            state.current_tool_name = Some(name.clone());
            state.tool_started_at = Some(std::time::Instant::now());
            state.has_intermediate_steps = true;
            state.current_action = Some(format!("Running {}", name));
            
            state.messages.push(ChatMessage {
                role: "tool".into(),
                content: tool_running(&name),
                timestamp: now(),
                id,
            });
            state
        }
        Event::AgentToolEnd { duration_secs } => {
            let mut state = state;
            state.current_action = None;  // Clear action when tool done
            state.tool_started_at = None;
            let mut state = state;
            // Replace "Running" with "Ran Xs" in the last tool message
            if let Some(name) = state.current_tool_name.take() {
                if let Some(last_msg) = state.messages.iter_mut().rev().find(|m| m.role == "tool") {
                    last_msg.content = tool_done(&name, duration_secs);
                }
            }
            state
        }
        Event::AgentResponse { id, content } => {
            let mut state = state;
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content,
                timestamp: now(),
                id: id.clone(),
            });
            state.current_request_id = Some(id);
            state
        }
        Event::AgentTurnComplete { id, duration_secs } => {
            let mut state = state;
            // Only show "Turn completed" if there were intermediate steps
            if state.has_intermediate_steps {
                state.messages.push(ChatMessage {
                    role: "turn_complete".into(),
                    content: format!("Turn completed in {:.1}s", duration_secs),
                    timestamp: now(),
                    id,
                });
            }
            state.turn_started_at = None;
            state
        }
        Event::AgentDone { id: _ } => {
            let mut state = state;
            state.current_request_id = None;
            state.current_tool_name = None;
            state.has_intermediate_steps = false;
            state.turn_active = false;
            state.turn_started_at = None;
            if state.request_queue.is_empty() {
                state.streaming = false;
                state.thinking_started_at = None;
            } else {
                state.streaming = true;
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
