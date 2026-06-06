//! DSL - Domain Specific Language for UI construction

use crate::model::{AppState, ChatMessage};
use crate::ui::elements::{Element, Feed};

pub struct Dsl;

impl Dsl {
    /// Get total element count (for scrollbar) - O(1) from cache
    pub fn count(state: &AppState) -> usize {
        state.element_count
    }
    
    /// Get visible elements only (virtual list) - O(visible + skip) not O(n)
    pub fn visible(state: &AppState, skip: usize, take: usize) -> Vec<Element> {
        let mut elements = Vec::with_capacity(take);
        let total = state.element_count;
        
        // If near the end, iterate backwards from messages
        if skip > total / 2 {
            return Self::visible_from_end(state, skip, take, total);
        }
        
        // Forward iteration
        let mut consumed = 0;
        for msg in &state.messages {
            match msg.role.as_str() {
                "user" | "thought" | "assistant" | "tool" | "turn_complete" => {
                    if consumed >= skip && elements.len() < take {
                        elements.push(Self::msg_to_element(msg, state));
                    }
                    consumed += 1;
                    
                    if consumed >= skip && elements.len() < take {
                        elements.push(Element::Spacer);
                    }
                    consumed += 1;
                }
                _ => {}
            }
        }
        
        if state.thinking_started_at.is_some() {
            if consumed >= skip && elements.len() < take {
                elements.push(Element::Thinking { 
                    elapsed: state.thinking_elapsed_secs().unwrap_or(0.0) 
                });
            }
        }
        
        elements
    }
    
    /// Iterate backwards from the end - for auto-scroll at bottom
    fn visible_from_end(state: &AppState, skip: usize, take: usize, total: usize) -> Vec<Element> {
        let mut elements = Vec::with_capacity(take);
        let mut target_start = total.saturating_sub(skip);
        let mut target_end = target_start + take;
        
        // Clamp
        target_start = target_start.min(total);
        target_end = target_end.min(total);
        
        // Build element list and iterate from end
        let mut cur_pos = 0;
        let msgs_len = state.messages.len();
        
        // Iterate messages in reverse
        for i in (0..msgs_len).rev() {
            let msg = &state.messages[i];
            match msg.role.as_str() {
                "user" | "thought" | "assistant" | "tool" | "turn_complete" => {
                    // This message contributes 2 elements (content + spacer)
                    let msg_end = cur_pos + 2;
                    let msg_start = cur_pos;
                    
                    // Check overlap with target range
                    if msg_end > target_start && msg_start < target_end {
                        // Spacer (if not first element we're collecting)
                        if elements.len() < take && msg_end > target_start {
                            if elements.len() < take {
                                elements.push(Element::Spacer);
                            }
                        }
                        // Content
                        if elements.len() < take && msg_end > target_start {
                            elements.push(Self::msg_to_element(msg, state));
                        }
                    }
                    
                    cur_pos = msg_end;
                }
                _ => {}
            }
        }
        
        // Handle thinking element at the very end
        if state.thinking_started_at.is_some() {
            let thinking_pos = cur_pos;
            if thinking_pos >= target_start && elements.len() < take {
                elements.push(Element::Thinking { 
                    elapsed: state.thinking_elapsed_secs().unwrap_or(0.0) 
                });
            }
        }
        
        elements.reverse();
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
