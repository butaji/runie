//! Update - State Transitions
use crate::model::{AppState, ChatMessage};

/// Events that can occur in the application
#[derive(Debug, Clone)]
pub enum Event {
    // Input events
    Input(char),
    Backspace,
    Submit,
    ScrollUp,
    ScrollDown,
    
    // Control events
    Quit,
    Reset,
}

/// Updates the state based on an event
/// Returns a new state (immutable update)
pub fn update(state: AppState, event: Event) -> AppState {
    match event {
        Event::Input(c) => state.push_input(c),
        Event::Backspace => state.pop_input(),
        Event::Submit => state.submit(),
        Event::ScrollUp => state.scroll_up(),
        Event::ScrollDown => state.scroll_down(),
        Event::Quit | Event::Reset => AppState::default(), // Reset returns default state
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
    
    /// Submit current input as user message
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
            content,
        });
        
        // Add echo response (placeholder for agent)
        state.messages.push(ChatMessage {
            role: "assistant".into(),
            content: "Echo: ...".into(),
        });
        
        state.streaming = false;
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
