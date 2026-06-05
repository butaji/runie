//! DSL - Domain Specific Language for UI construction
//! 
//! Provides declarative operations to build UI from state.

use crate::model::AppState;
use crate::ui::elements::{Element, Feed};

/// Bailer spinner frames
const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

/// DSL operations for building UI
pub struct Dsl;

impl Dsl {
    /// Build feed from state
    pub fn feed(state: &AppState) -> Feed {
        let mut feed = Feed::new();
        let mut last_id = String::new();
        
        for msg in &state.messages {
            match msg.role.as_str() {
                "user" => {
                    feed.elements.push(Element::UserMessage { 
                        content: msg.content.clone() 
                    });
                    feed.elements.push(Element::Spacer);
                    last_id = msg.id.clone();
                }
                "thought" => {
                    feed.elements.push(Element::ThoughtMarker { 
                        content: msg.content.clone() 
                    });
                    feed.elements.push(Element::Spacer);
                    last_id = msg.id.clone();
                }
                "assistant" => {
                    // Check if we can combine with previous AgentMessage (penultimate element)
                    let len = feed.elements.len();
                    let prev_was_agent = len >= 2
                        && matches!(feed.elements[len - 2], Element::AgentMessage { .. });
                    
                    if last_id == msg.id && prev_was_agent {
                        // Append to existing AgentMessage (penultimate)
                        let idx = len - 2;
                        if let Element::AgentMessage { content, .. } = &mut feed.elements[idx] {
                            content.push_str(&msg.content);
                        }
                    } else {
                        feed.elements.push(Element::AgentMessage { 
                            content: msg.content.clone() 
                        });
                        feed.elements.push(Element::Spacer);
                    }
                    last_id = msg.id.clone();
                }
                _ => {}
            }
        }
        
        // Add thinking indicator if current request has no thought yet
        let has_thought_for_current = state.current_request_id
            .as_ref()
            .map(|current_id| state.messages.iter().any(|m| m.role == "thought" && m.id == *current_id))
            .unwrap_or(false);
        
        if state.streaming && !has_thought_for_current {
            let elapsed = state.thinking_elapsed_secs().unwrap_or(0.0);
            feed.elements.push(Element::Thinking { elapsed });
            feed.elements.push(Element::Spacer);
        }
        
        feed
    }
    
    /// Get spinner character based on elapsed time
    pub fn spinner(elapsed: f64) -> &'static str {
        let frame_idx = ((elapsed * 10.0) as usize) % SPINNER_FRAMES.len();
        SPINNER_FRAMES[frame_idx]
    }
}
