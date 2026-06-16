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
        let mut entries = Self::collect_entries(state);
        entries.sort_by(|a, b| {
            a.timestamp()
                .partial_cmp(&b.timestamp())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut feed = Feed::new();
        for elem in entries.into_iter() {
            let ts = elem.timestamp();
            let kind = Self::post_kind(&elem);
            let expanded = !matches!(
                elem,
                Element::ThoughtSummary { .. } | Element::ToolSummary { .. }
            );
            feed.push_post(
                crate::ui::posts::PostBuilder::new(kind)
                    .with_element(elem)
                    .expanded(expanded)
                    .at(ts),
            );
        }
        feed
    }

    fn post_kind(elem: &Element) -> crate::ui::elements::PostKind {
        use crate::ui::elements::{Element as E, PostKind};
        match elem {
            E::Spacer { .. } => PostKind::System,
            E::UserMessage { .. } => PostKind::UserInput,
            E::AgentMessage { .. } => PostKind::AgentResponse,
            E::Thinking { .. } => PostKind::Thinking,
            E::ThoughtMarker { .. } | E::ThoughtSummary { .. } => PostKind::Thought,
            E::ToolRunning { .. } => PostKind::ToolRunning,
            E::ToolDone { .. } => PostKind::ToolDone,
            E::ToolSummary { .. } => PostKind::ToolSummary,
            E::TurnComplete { .. } => PostKind::TurnComplete,
        }
    }

    fn action_turn_id(msg: &crate::model::ChatMessage) -> Option<String> {
        match msg.role {
            Role::Thought => msg.id.split_once('#').map(|(prefix, _)| prefix.to_string()),
            Role::Tool => {
                let rest = msg.id.strip_prefix("tool.")?;
                let idx = rest.rfind('.')?;
                Some(rest[..idx].to_string())
            }
            _ => None,
        }
    }

    fn action_counts(state: &AppState) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for msg in state.session.messages.iter() {
            if let Some(turn_id) = Self::action_turn_id(msg) {
                *counts.entry(turn_id).or_insert(0) += 1;
            }
        }
        counts
    }

    fn thinking_timestamp(state: &AppState) -> f64 {
        let max_ts = state
            .session
            .messages
            .iter()
            .map(|m| m.timestamp)
            .fold(0.0, f64::max);
        let turn_ts = state
            .session
            .messages
            .iter()
            .find(|m| m.role == Role::TurnComplete)
            .map(|m| m.timestamp);
        let current = state.agent.current_request_id.as_ref().and_then(|id| {
            state
                .session
                .messages
                .iter()
                .find(|m| m.role == Role::TurnComplete && m.id == *id)
                .map(|m| m.timestamp)
        });
        current
            .map(|t| t - 1e-6)
            .unwrap_or_else(|| turn_ts.map(|t| t + 1e-6).unwrap_or(max_ts + 1e-6))
    }

    fn collect_entries(state: &AppState) -> Vec<Element> {
        let mut entries: Vec<Element> = Vec::new();
        let action_counts = Self::action_counts(state);

        for msg in state.session.messages.iter() {
            if Self::should_skip_msg(msg, state) {
                continue;
            }
            if msg.role == Role::TurnComplete {
                let count = action_counts.get(&msg.id).copied().unwrap_or(0);
                if count <= 1 {
                    continue;
                }
            }
            entries.push(Self::msg_to_elem(msg, state));
        }

        if let Some(started) = state.agent.thinking_started_at {
            entries.push(Element::thinking(started).at(Self::thinking_timestamp(state)));
        }

        entries
    }

    fn should_skip_msg(msg: &ChatMessage, state: &AppState) -> bool {
        if msg.role != Role::Assistant {
            return false;
        }
        if crate::update::content_has_tool_markers(&msg.content) {
            return true;
        }
        state.agent.thinking_started_at.is_some()
            && state.agent.current_request_id.as_deref() == Some(&msg.id)
    }

    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (start + take).min(cache.len());
        &cache[start..end]
    }

    fn msg_to_elem(msg: &ChatMessage, state: &AppState) -> Element {
        let ts = msg.timestamp;
        match msg.role {
            Role::User => Element::user(msg.content.clone()).at(ts),
            Role::Thought => Self::thought_elem(msg, state, ts),
            Role::Assistant => Element::AgentMessage {
                content: crate::update::strip_tool_markers(&msg.content),
                timestamp: ts,
                provider: msg.provider.clone(),
            },
            Role::Tool => Self::tool_elem(msg, state, ts),
            Role::TurnComplete => Element::turn_complete(Self::parse_dur(&msg.content)).at(ts), // filtered in collect_entries
            Role::System => Element::thought(msg.content.clone()).at(ts),
        }
    }

    fn thought_elem(msg: &ChatMessage, state: &AppState, ts: f64) -> Element {
        if state.view.all_collapsed {
            let first_line = msg
                .content
                .lines()
                .next()
                .unwrap_or(&msg.content)
                .to_string();
            Element::thought_summary(first_line, Self::parse_thought_dur(&msg.content)).at(ts)
        } else {
            Element::thought(msg.content.clone()).at(ts)
        }
    }

    fn parse_thought_dur(content: &str) -> f64 {
        content
            .split_whitespace()
            .last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }

    fn tool_elem(msg: &ChatMessage, state: &AppState, ts: f64) -> Element {
        if msg.content.contains("⠋ Running ") {
            let name = msg
                .content
                .trim_start_matches("⠋ Running ")
                .trim_end_matches("...");
            return Element::tool_running(
                name,
                "",
                state
                    .agent
                    .tool_started_at
                    .unwrap_or_else(std::time::Instant::now),
            )
            .at(ts);
        }
        let (name, dur, output) = Self::parse_tool_content(&msg.content);
        if state.view.all_collapsed {
            Element::tool_summary(name, dur).at(ts)
        } else {
            Element::tool_done(name, String::new(), dur, output, None, false).at(ts)
        }
    }

    fn parse_tool_content(content: &str) -> (String, f64, String) {
        let lines: Vec<&str> = content.lines().collect();
        let header = lines.first().copied().unwrap_or("");
        let output = lines
            .get(1..)
            .map(|rest| rest.join("\n"))
            .unwrap_or_default();
        let parts: Vec<&str> = header.split_whitespace().collect();
        let name = parts.get(1).unwrap_or(&"").to_string();
        let dur = parts
            .last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0);
        (name, dur, output)
    }

    fn parse_dur(content: &str) -> f64 {
        content
            .split_whitespace()
            .last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }
}
