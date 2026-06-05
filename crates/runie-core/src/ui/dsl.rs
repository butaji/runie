//! DSL - Domain Specific Language for UI construction

use crate::model::AppState;
use crate::ui::elements::{Element, Feed};

const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

pub struct Dsl;

impl Dsl {
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
                    let len = feed.elements.len();
                    let prev_was_agent = len >= 2
                        && matches!(feed.elements[len - 2], Element::AgentMessage { .. });
                    
                    if last_id == msg.id && prev_was_agent {
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
                "tool" => {
                    // Format: "🔧 Ran name Xs"
                    feed.elements.push(Element::ToolRun { 
                        content: msg.content.clone() 
                    });
                    feed.elements.push(Element::Spacer);
                    last_id = msg.id.clone();
                }
                "turn_complete" => {
                    let duration = Self::parse_duration(&msg.content);
                    feed.elements.push(Element::TurnComplete { 
                        duration_secs: duration 
                    });
                    feed.elements.push(Element::Spacer);
                    last_id = msg.id.clone();
                }
                _ => {}
            }
        }
        
        // Show Thinking spinner if thinking in progress
        if state.thinking_started_at.is_some() {
            let elapsed = state.thinking_elapsed_secs().unwrap_or(0.0);
            feed.elements.push(Element::Thinking { elapsed });
            feed.elements.push(Element::Spacer);
        }
        
        feed
    }
    
    fn parse_duration(content: &str) -> f64 {
        content.split_whitespace()
            .last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }
    
    pub fn spinner(elapsed: f64) -> &'static str {
        let frame_idx = ((elapsed * 10.0) as usize) % SPINNER_FRAMES.len();
        SPINNER_FRAMES[frame_idx]
    }
}
