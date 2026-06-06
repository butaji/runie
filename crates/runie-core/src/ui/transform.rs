//! Transform layer — Pure state → element transformations
//!
//! Performance contract:
//!   build_elements :: O(n) but only when dirty
//!   visible        :: O(1) slice (no allocation)
//!   count          :: O(1) cached
//!
//! Invariant: after build_elements(), count() and visible() are O(1)

use crate::model::{AppState, ChatMessage};
use crate::ui::elements::{Element, Feed};

/// Lazy cache — rebuilds only when dirty
pub struct LazyCache;

impl LazyCache {
    /// O(n) — rebuild entire cache from messages
    /// Called only when AppState.dirty == true
    pub fn rebuild(state: &AppState) -> Vec<Element> {
        let cap = state.messages.len() * 2;
        let mut elems = Vec::with_capacity(cap);

        for msg in &state.messages {
            if Self::is_renderable(msg) {
                elems.push(Self::message_to_element(msg, state));
                elems.push(Element::Spacer);
            }
        }

        if state.thinking_started_at.is_some() {
            elems.push(Element::Thinking {
                elapsed: state.thinking_elapsed_secs().unwrap_or(0.0),
            });
            elems.push(Element::Spacer);
        }

        elems
    }

    /// O(1) — slice into cached elements
    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (skip + take).min(cache.len());
        &cache[start..end]
    }

    fn is_renderable(msg: &ChatMessage) -> bool {
        matches!(
            msg.role.as_str(),
            "user" | "thought" | "assistant" | "tool" | "turn_complete"
        )
    }

    fn message_to_element(msg: &ChatMessage, state: &AppState) -> Element {
        match msg.role.as_str() {
            "user" => Element::UserMessage {
                content: msg.content.clone(),
            },
            "thought" => Element::ThoughtMarker {
                content: msg.content.clone(),
            },
            "assistant" => Element::AgentMessage {
                content: msg.content.clone(),
            },
            "tool" => Self::tool_element(msg, state),
            "turn_complete" => Element::TurnComplete {
                duration_secs: Self::parse_duration(&msg.content),
            },
            _ => Element::Spacer,
        }
    }

    fn tool_element(msg: &ChatMessage, state: &AppState) -> Element {
        if msg.content.contains("Running") {
            let name = msg
                .content
                .trim_start_matches("⠋ Running ")
                .trim_end_matches("...");
            Element::ToolRunning {
                name: name.to_string(),
                elapsed: state.tool_elapsed_secs().unwrap_or(0.0),
            }
        } else {
            let name = msg
                .content
                .trim_start_matches("◆ Ran ")
                .split(' ')
                .next()
                .unwrap_or("");
            let dur = msg
                .content
                .split_whitespace()
                .last()
                .map(|s| s.trim_end_matches('s').parse().unwrap_or(0.0))
                .unwrap_or(0.0);
            Element::ToolDone {
                name: name.to_string(),
                duration_secs: dur,
            }
        }
    }

    fn parse_duration(content: &str) -> f64 {
        content
            .split_whitespace()
            .last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }

    /// Build Feed with streaming chunk merging
    pub fn feed(state: &AppState) -> Feed {
        let mut feed = Feed::new();
        let mut last_id = String::new();

        for msg in &state.messages {
            Self::push_message(&mut feed, msg, state, &mut last_id);
        }
        Self::push_thinking(&mut feed, state);
        feed
    }

    fn push_message(feed: &mut Feed, msg: &ChatMessage, state: &AppState, last_id: &mut String) {
        match msg.role.as_str() {
            "user" => Self::push_elem(feed, Element::UserMessage { content: msg.content.clone() }, last_id, msg),
            "thought" => Self::push_elem(feed, Element::ThoughtMarker { content: msg.content.clone() }, last_id, msg),
            "assistant" => Self::push_assistant(feed, msg, last_id),
            "tool" => Self::push_elem(feed, Self::message_to_element(msg, state), last_id, msg),
            "turn_complete" => Self::push_elem(feed, Element::TurnComplete { duration_secs: Self::parse_duration(&msg.content) }, last_id, msg),
            _ => {}
        }
    }

    fn push_elem(feed: &mut Feed, elem: Element, last_id: &mut String, msg: &ChatMessage) {
        feed.elements.push(elem);
        feed.elements.push(Element::Spacer);
        *last_id = msg.id.clone();
    }

    fn push_assistant(feed: &mut Feed, msg: &ChatMessage, last_id: &mut String) {
        if !StreamingMerge::merge(&mut feed.elements, msg, last_id) {
            feed.elements.push(Element::AgentMessage { content: msg.content.clone() });
            feed.elements.push(Element::Spacer);
        }
        *last_id = msg.id.clone();
    }

    fn push_thinking(feed: &mut Feed, state: &AppState) {
        if let Some(elapsed) = state.thinking_started_at.map(|_| state.thinking_elapsed_secs().unwrap_or(0.0)) {
            feed.elements.push(Element::Thinking { elapsed });
            feed.elements.push(Element::Spacer);
        }
    }
}

/// Streaming merge — combines chunks with same id
pub struct StreamingMerge;

impl StreamingMerge {
    /// Merges assistant chunks into single element
    pub fn merge(elements: &mut Vec<Element>, msg: &ChatMessage, last_id: &str) -> bool {
        let len = elements.len();
        let prev_was_agent = len >= 2
            && matches!(elements[len - 2], Element::AgentMessage { .. });

        if last_id == msg.id && prev_was_agent {
            let idx = len - 2;
            if let Element::AgentMessage { content, .. } = &mut elements[idx] {
                content.push_str(&msg.content);
            }
            true
        } else {
            false
        }
    }
}
