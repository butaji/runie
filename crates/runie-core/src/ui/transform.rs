use crate::model::{AppState, ChatMessage, Role};
use crate::ui::elements::{Element, Feed};

pub struct LazyCache;

impl LazyCache {
    pub fn rebuild(state: &AppState) -> Vec<Element> {
        Self::build(state).elements
    }

    pub fn feed(state: &AppState) -> Feed {
        Self::build(state)
    }

    fn build(state: &AppState) -> Feed {
        let mut entries: Vec<(f64, usize, Element, String)> = Vec::new();

        for (idx, msg) in state.messages.iter().enumerate() {
            let elem = Self::msg_to_elem(msg, state);
            entries.push((msg.timestamp, idx, elem, msg.id.clone()));
        }

        let max_ts = state.messages.iter().map(|m| m.timestamp).fold(0.0, f64::max);
        if let Some(started) = state.thinking_started_at {
            let elapsed = started.elapsed().as_secs_f64();
            entries.push((max_ts + 1.0, usize::MAX, Element::Thinking { elapsed }, String::new()));
        }

        entries.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });

        let mut moves = Vec::new();
        for (i, (_, _, elem, id)) in entries.iter().enumerate() {
            if matches!(elem, Element::ThoughtMarker { .. }) {
                if let Some(target) = (0..i).position(|j| {
                    matches!(entries[j].2, Element::AgentMessage { .. }) && entries[j].3 == *id
                }) {
                    moves.push((i, target));
                }
            }
        }
        for (from, to) in moves.into_iter().rev() {
            let entry = entries.remove(from);
            entries.insert(to, entry);
        }

        if let Some(current_id) = state.current_request_id.as_ref() {
            if let Some(thinking_idx) = entries.iter().position(|(_, _, elem, _)| matches!(elem, Element::Thinking { .. })) {
                if let Some(agent_idx) = entries.iter().position(|(_, _, elem, id)| {
                    matches!(elem, Element::AgentMessage { .. }) && *id == *current_id
                }) {
                    if thinking_idx > agent_idx {
                        let entry = entries.remove(thinking_idx);
                        entries.insert(agent_idx, entry);
                    }
                }
            }
        }

        let mut feed = Feed::new();
        for (_, _, elem, _) in entries {
            feed.elements.push(elem);
            feed.elements.push(Element::Spacer);
        }
        feed
    }

    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (start + take).min(cache.len());
        &cache[start..end]
    }

    fn msg_to_elem(msg: &ChatMessage, state: &AppState) -> Element {
        match msg.role {
            Role::User => Element::UserMessage { content: msg.content.clone() },
            Role::Thought => Element::ThoughtMarker { content: msg.content.clone() },
            Role::Assistant => Element::AgentMessage { content: crate::update::strip_tool_markers(&msg.content) },
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
            let name = parts.get(2).unwrap_or(&"");
            let dur = parts.last().and_then(|s| s.trim_end_matches('s').parse().ok()).unwrap_or(0.0);
            Element::ToolDone { name: name.to_string(), duration_secs: dur }
        }
    }

    fn parse_dur(content: &str) -> f64 {
        content.split_whitespace().last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
pub mod format_test {
    use crate::ui::elements::{Element, Feed};
    use crate::labels::{PREFIX_USER, PREFIX_AGENT};

    #[derive(Debug, Clone)]
    pub struct DisplayLine { pub spans: Vec<DisplaySpan> }

    #[derive(Debug, Clone)]
    pub struct DisplaySpan { pub text: String }

    pub fn format_messages(state: &crate::model::AppState) -> Vec<DisplayLine> {
        let feed = super::LazyCache::feed(state);
        render_feed(&feed, state)
    }

    pub fn render_feed(feed: &Feed, state: &crate::model::AppState) -> Vec<DisplayLine> {
        feed.elements.iter().flat_map(|e| render_element(e, state)).collect()
    }

    fn render_element(element: &Element, state: &crate::model::AppState) -> Vec<DisplayLine> {
        match element {
            Element::Spacer => vec![DisplayLine { spans: vec![] }],
            Element::UserMessage { content } => vec![DisplayLine {
                spans: vec![
                    DisplaySpan { text: PREFIX_USER.to_string() },
                    DisplaySpan { text: content.clone() },
                ],
            }],
            Element::AgentMessage { content } => vec![DisplayLine {
                spans: vec![
                    DisplaySpan { text: PREFIX_AGENT.to_string() },
                    DisplaySpan { text: content.clone() },
                ],
            }],
            Element::Thinking { elapsed } => vec![DisplayLine {
                spans: vec![DisplaySpan {
                    text: format!("{} Thinking... {:.1}s", state.spinner_frame(), elapsed),
                }],
            }],
            Element::ThoughtMarker { content } => vec![DisplayLine {
                spans: vec![DisplaySpan { text: content.clone() }],
            }],
            Element::ToolRunning { name, elapsed } => vec![DisplayLine {
                spans: vec![DisplaySpan {
                    text: format!("{} Running {}... {:.1}s", state.spinner_frame(), name, elapsed),
                }],
            }],
            Element::ToolDone { name, duration_secs } => vec![DisplayLine {
                spans: vec![DisplaySpan {
                    text: format!("◆ Ran {} {:.1}s", name, duration_secs),
                }],
            }],
            Element::TurnComplete { duration_secs } => vec![DisplayLine {
                spans: vec![DisplaySpan {
                    text: format!("Turn completed in {:.1}s", duration_secs),
                }],
            }],
        }
    }
}
