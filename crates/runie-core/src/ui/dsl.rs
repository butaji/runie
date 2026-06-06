//! DSL - Domain Specific Language for UI construction

use crate::model::{AppState, ChatMessage};
use crate::ui::elements::{Element, Feed};

pub struct Dsl;

impl Dsl {
    /// Get total element count - O(1) from cache
    pub fn count(state: &AppState) -> usize {
        state.element_count
    }
    
    /// Get visible elements as slice - O(1), zero allocation
    pub fn visible(state: &AppState, skip: usize, take: usize) -> &[Element] {
        state.visible(skip, take)
    }
    
    /// Build elements cache - O(n), called lazily
    pub fn build_elements(state: &AppState) -> Vec<Element> {
        let mut elements = Vec::with_capacity(state.messages.len() * 2);
        
        for msg in &state.messages {
            let role = msg.role.as_str();
            if !matches!(role, "user" | "thought" | "assistant" | "tool" | "turn_complete") {
                continue;
            }
            elements.push(Self::msg_to_element(msg, state));
            elements.push(Element::Spacer);
        }
        
        if state.thinking_started_at.is_some() {
            elements.push(Element::Thinking { 
                elapsed: state.thinking_elapsed_secs().unwrap_or(0.0) 
            });
            elements.push(Element::Spacer);
        }
        
        elements
    }
    
    fn msg_to_element(msg: &ChatMessage, state: &AppState) -> Element {
        match msg.role.as_str() {
            "user" => Element::UserMessage { content: msg.content.clone() },
            "thought" => Element::ThoughtMarker { content: msg.content.clone() },
            "assistant" => Element::AgentMessage { content: msg.content.clone() },
            "tool" => {
                if msg.content.contains("Running") {
                    let name = msg.content.trim_start_matches("⠋ Running ").trim_end_matches("...");
                    Element::ToolRunning { 
                        name: name.to_string(),
                        elapsed: state.tool_elapsed_secs().unwrap_or(0.0),
                    }
                } else {
                    let name = msg.content.trim_start_matches("◆ Ran ").split(' ').next().unwrap_or("");
                    let dur = msg.content.split_whitespace().last()
                        .map(|s| s.trim_end_matches('s').parse().unwrap_or(0.0))
                        .unwrap_or(0.0);
                    Element::ToolDone { 
                        name: name.to_string(),
                        duration_secs: dur,
                    }
                }
            }
            "turn_complete" => {
                let duration = Self::parse_duration(&msg.content);
                Element::TurnComplete { duration_secs: duration }
            }
            _ => Element::Spacer,
        }
    }
    
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
                    if msg.content.contains("Running") {
                        let name = msg.content.trim_start_matches("⠋ Running ").trim_end_matches("...");
                        let elapsed = state.tool_elapsed_secs().unwrap_or(0.0);
                        feed.elements.push(Element::ToolRunning { 
                            name: name.to_string(),
                            elapsed,
                        });
                    } else {
                        let name = msg.content.trim_start_matches("◆ Ran ").split(' ').next().unwrap_or("");
                        let dur = msg.content.split_whitespace().last().map(|s| s.trim_end_matches('s').parse().unwrap_or(0.0)).unwrap_or(0.0);
                        feed.elements.push(Element::ToolDone { 
                            name: name.to_string(),
                            duration_secs: dur,
                        });
                    }
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
}
