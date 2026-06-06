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
    let state = match event {
        // === UI Events ===
        Event::Input(c) => return state.push_input(c),
        Event::Backspace => return state.pop_input(),
        Event::Submit => {
            let mut state = state.submit();
            state.mark_dirty();
            state
        }
        Event::ScrollUp => return state.scroll_up(),
        Event::ScrollDown => return state.scroll_down(),
        
        // === System Events ===
        Event::Quit => return state,
        Event::Reset => {
            let mut state = AppState::default();
            state.mark_dirty();
            return state;
        }
        
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
            state.mark_dirty();
            state
        }
        Event::AgentThoughtDone { id } => {
            let mut state = state;
            let duration = state.thinking_elapsed_secs().unwrap_or(0.0);
            state.current_action = None;
            state.thinking_started_at = None;
            
            state.messages.push(ChatMessage {
                role: "thought".into(),
                content: thought_with_time(duration),
                timestamp: now(),
                id,
            });
            state.mark_dirty();
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
            state.mark_dirty();
            state
        }
        Event::AgentToolEnd { duration_secs } => {
            let mut state = state;
            state.current_action = None;
            state.tool_started_at = None;
            if let Some(name) = state.current_tool_name.take() {
                if let Some(last_msg) = state.messages.iter_mut().rev().find(|m| m.role == "tool") {
                    last_msg.content = tool_done(&name, duration_secs);
                }
            }
            state.mark_dirty();
            state
        }
        Event::AgentResponse { id, content } => {
            let mut state = state;
            
            // Append to last assistant message if exists, else create new
            if let Some(last) = state.messages.last_mut() {
                if last.role == "assistant" && last.id == id {
                    last.content.push_str(&content);
                    state.mark_dirty();
                    return state;
                }
            }
            
            state.messages.push(ChatMessage {
                role: "assistant".into(),
                content,
                timestamp: now(),
                id: id.clone(),
            });
            state.mark_dirty();
            state.current_request_id = Some(id);
            state
        }
        Event::AgentTurnComplete { id, duration_secs } => {
            let mut state = state;
            if state.has_intermediate_steps {
                state.messages.push(ChatMessage {
                    role: "turn_complete".into(),
                    content: format!("Turn completed in {:.1}s", duration_secs),
                    timestamp: now(),
                    id,
                });
                state.mark_dirty();
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
            state.mark_dirty();
            state
        }
        Event::SwitchModel { provider, model } => {
            let mut state = state;
            state.current_provider = provider.clone();
            state.current_model = model.clone();
            state.messages.push(ChatMessage {
                role: "system".into(),
                content: format!("Switched to {}/{}", provider, model),
                timestamp: now(),
                id: "switch".to_string(),
            });
            state.mark_dirty();
            state
        }
        Event::ShowHelp => {
            let mut state = state;
            state.messages.push(ChatMessage {
                role: "system".into(),
                content: "Commands:\n/model provider/model — switch model\n/reset — clear state\n/help — show this".to_string(),
                timestamp: now(),
                id: "help".to_string(),
            });
            state.mark_dirty();
            state
        }
        Event::SpawnAgent => return state,
    };
    
    state
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
        
        // Slash commands
        if content == "/reset" {
            return AppState::default();
        }
        if content == "/help" {
            return update(state, Event::ShowHelp);
        }
        if let Some(rest) = content.strip_prefix("/model ") {
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() == 2 {
                return update(state, Event::SwitchModel {
                    provider: parts[0].to_string(),
                    model: parts[1].to_string(),
                });
            } else {
                state.messages.push(ChatMessage {
                    role: "system".into(),
                    content: "Usage: /model provider/model".to_string(),
                    timestamp: now(),
                    id: "err".to_string(),
                });
                return state;
            }
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
