//! Transform layer — State → elements (O(n) only when dirty)
use crate::model::{AppState, ChatMessage, Role};
use crate::ui::elements::{Element, Feed};

pub struct LazyCache;

impl LazyCache {
    /// O(n) — rebuild cache only when dirty (caller must check dirty flag)
    pub fn rebuild(state: &AppState) -> Vec<Element> {
        let msg_count = state.messages.iter().filter(|m| Self::is_renderable(m)).count();
        let cap = msg_count * 2 + 2; // elements + spacers + thinking
        let mut elems = Vec::with_capacity(cap);

        for msg in &state.messages {
            if Self::is_renderable(msg) {
                elems.push(Self::msg_to_elem(msg, state));
                elems.push(Element::Spacer);
            }
        }

        if let Some(elapsed) = state.thinking_started_at {
            elems.push(Element::Thinking { elapsed: elapsed.elapsed().as_secs_f64() });
            elems.push(Element::Spacer);
        }

        elems
    }

    /// O(1) — slice into cached elements
    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (start + take).min(cache.len());
        &cache[start..end]
    }

    fn is_renderable(msg: &ChatMessage) -> bool {
        matches!(msg.role, Role::User | Role::Thought | Role::Assistant | Role::Tool | Role::TurnComplete | Role::System)
    }

    fn msg_to_elem(msg: &ChatMessage, state: &AppState) -> Element {
        match msg.role {
            Role::User => Element::UserMessage { content: msg.content.clone() },
            Role::Thought => Element::ThoughtMarker { content: msg.content.clone() },
            Role::Assistant => Element::AgentMessage { content: msg.content.clone() },
            Role::Tool => Self::tool_elem(msg, state),
            Role::TurnComplete => Element::TurnComplete { duration_secs: Self::parse_dur(&msg.content) },
            Role::System => Element::ThoughtMarker { content: msg.content.clone() },
        }
    }

    fn tool_elem(msg: &ChatMessage, state: &AppState) -> Element {
        if msg.content.contains("Running") {
            let name = msg.content.trim_start_matches("⠋ Running ").trim_end_matches("...");
            Element::ToolRunning { name: name.to_string(), elapsed: state.tool_elapsed_secs().unwrap_or(0.0) }
        } else {
            let parts: Vec<&str> = msg.content.split_whitespace().collect();
            let name = parts.get(2).unwrap_or(&"");  // "list_files" is at index 2
            let dur = parts.last().and_then(|s| s.trim_end_matches('s').parse().ok()).unwrap_or(0.0);
            Element::ToolDone { name: name.to_string(), duration_secs: dur }
        }
    }

    fn parse_dur(content: &str) -> f64 {
        content.split_whitespace().last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }

    /// Build Feed with streaming chunk merging
    pub fn feed(state: &AppState) -> Feed {
        let mut feed = Feed::new();
        let mut last_id = String::new();

        for msg in &state.messages {
            match msg.role {
                Role::User => Self::push_user(&mut feed, msg, &mut last_id),
                Role::Thought | Role::System => Self::push_thought(&mut feed, msg, &mut last_id),
                Role::Assistant => Self::push_agent(&mut feed, msg, &mut last_id),
                Role::Tool => Self::push_tool(&mut feed, msg, state, &mut last_id),
                Role::TurnComplete => Self::push_turn(&mut feed, msg, &mut last_id),
            }
        }
        if let Some(elapsed) = state.thinking_started_at {
            feed.elements.push(Element::Thinking { elapsed: elapsed.elapsed().as_secs_f64() });
            feed.elements.push(Element::Spacer);
        }
        feed
    }

    fn push_user(feed: &mut Feed, msg: &ChatMessage, last: &mut String) {
        feed.elements.push(Element::UserMessage { content: msg.content.clone() });
        feed.elements.push(Element::Spacer);
        *last = msg.id.clone();
    }

    fn push_thought(feed: &mut Feed, msg: &ChatMessage, last: &mut String) {
        feed.elements.push(Element::ThoughtMarker { content: msg.content.clone() });
        feed.elements.push(Element::Spacer);
        *last = msg.id.clone();
    }

    fn push_agent(feed: &mut Feed, msg: &ChatMessage, last: &mut String) {
        if !StreamingMerge::merge(&mut feed.elements, msg, last) {
            feed.elements.push(Element::AgentMessage { content: msg.content.clone() });
            feed.elements.push(Element::Spacer);
        }
        *last = msg.id.clone();
    }

    fn push_tool(feed: &mut Feed, msg: &ChatMessage, state: &AppState, last: &mut String) {
        feed.elements.push(Self::msg_to_elem(msg, state));
        feed.elements.push(Element::Spacer);
        *last = msg.id.clone();
    }

    fn push_turn(feed: &mut Feed, msg: &ChatMessage, last: &mut String) {
        feed.elements.push(Element::TurnComplete { duration_secs: Self::parse_dur(&msg.content) });
        feed.elements.push(Element::Spacer);
        *last = msg.id.clone();
    }
}

pub struct StreamingMerge;

impl StreamingMerge {
    pub fn merge(elems: &mut Vec<Element>, msg: &ChatMessage, last_id: &str) -> bool {
        let len = elems.len();
        if last_id == msg.id && len >= 2 && matches!(elems[len - 2], Element::AgentMessage { .. }) {
            if let Element::AgentMessage { content, .. } = &mut elems[len - 2] {
                content.push_str(&msg.content);
            }
            true
        } else {
            false
        }
    }
}
