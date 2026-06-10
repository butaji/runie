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
            a.timestamp().partial_cmp(&b.timestamp())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut feed = Feed::new();
        for elem in entries {
            let ts = elem.timestamp();
            feed.elements.push(elem);
            feed.elements.push(Element::spacer().at(ts));
        }
        feed
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

        if let Some(started) = state.thinking_started_at {
            let max_ts = state.session.messages.iter().map(|m| m.timestamp).fold(0.0, f64::max);
            let turn_ts = state.session.messages.iter()
                .find(|m| m.role == Role::TurnComplete)
                .map(|m| m.timestamp);
            let turn_complete_for_current = state.agent.current_request_id.as_ref().and_then(|id| {
                state.session.messages.iter()
                    .find(|m| m.role == Role::TurnComplete && m.id == *id)
                    .map(|m| m.timestamp)
            });
            let ts = if let Some(tc_ts) = turn_complete_for_current {
                tc_ts - 1e-6
            } else {
                turn_ts.map(|t| t + 1e-6).unwrap_or(max_ts + 1e-6)
            };
            entries.push(Element::thinking(started).at(ts));
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
        state.thinking_started_at.is_some()
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
            Role::Assistant => Element::AgentMessage { content: crate::update::strip_tool_markers(&msg.content), timestamp: ts, provider: msg.provider.clone() },
            Role::Tool => Self::tool_elem(msg, state, ts),
            Role::TurnComplete => Element::turn_complete(Self::parse_dur(&msg.content)).at(ts), // filtered in collect_entries
            Role::System => Element::thought(msg.content.clone()).at(ts),
        }
    }

    fn thought_elem(msg: &ChatMessage, state: &AppState, ts: f64) -> Element {
        if state.all_collapsed {
            let first_line = msg.content.lines().next().unwrap_or(&msg.content).to_string();
            Element::thought_summary(first_line, Self::parse_thought_dur(&msg.content)).at(ts)
        } else {
            Element::thought(msg.content.clone()).at(ts)
        }
    }

    fn parse_thought_dur(content: &str) -> f64 {
        content.split_whitespace().last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }

    fn tool_elem(msg: &ChatMessage, state: &AppState, ts: f64) -> Element {
        if msg.content.contains("⠋ Running ") {
            let name = msg.content.trim_start_matches("⠋ Running ").trim_end_matches("...");
            return Element::tool_running(name, state.agent.tool_started_at.unwrap_or_else(std::time::Instant::now)).at(ts);
        }
        let (name, dur, output) = Self::parse_tool_content(&msg.content);
        if state.all_collapsed {
            Element::tool_summary(name, dur).at(ts)
        } else {
            Element::tool_done(name, dur, output).at(ts)
        }
    }

    fn parse_tool_content(content: &str) -> (String, f64, String) {
        let lines: Vec<&str> = content.lines().collect();
        let header = lines.first().copied().unwrap_or("");
        let output = lines.get(1..).map(|rest| rest.join("\n")).unwrap_or_default();
        let parts: Vec<&str> = header.split_whitespace().collect();
        let name = parts.get(1).unwrap_or(&"").to_string();
        let dur = parts.last().and_then(|s| s.trim_end_matches('s').parse().ok()).unwrap_or(0.0);
        (name, dur, output)
    }

    fn parse_dur(content: &str) -> f64 {
        content.split_whitespace().last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
pub mod format_test {
    use crate::labels::format_timestamp;
    use crate::ui::elements::{Element, Feed};

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
            Element::Spacer { .. } => vec![DisplayLine { spans: vec![] }],
            Element::UserMessage { content, .. } => render_user(content),
            Element::AgentMessage { content, .. } => render_agent(content),
            Element::Thinking { started, .. } => render_thinking(state, *started),
            Element::ThoughtMarker { content, .. } => render_thought_marker(content),
            Element::ThoughtSummary { content, .. } => render_thought_summary(content),
            Element::ToolRunning { name, started, .. } => render_tool_running(state, name, *started),
            Element::ToolDone { name, duration_secs, output, .. } => render_tool_done(name, *duration_secs, output),
            Element::ToolSummary { name, duration_secs, .. } => render_tool_summary(name, *duration_secs),
            Element::TurnComplete { duration_secs, .. } => render_turn_complete(*duration_secs),
        }
    }

    fn render_user(content: &str) -> Vec<DisplayLine> {
        let ts = format_timestamp(0.0);
        vec![DisplayLine { spans: vec![
            DisplaySpan { text: "$".to_string() },
            DisplaySpan { text: " ".to_string() },
            DisplaySpan { text: content.to_string() },
            DisplaySpan { text: format!(" {}", ts) },
        ]}]
    }

    fn render_agent(content: &str) -> Vec<DisplayLine> {
        let ts = format_timestamp(0.0);
        vec![DisplayLine { spans: vec![
            DisplaySpan { text: "→".to_string() },
            DisplaySpan { text: " ".to_string() },
            DisplaySpan { text: content.to_string() },
            DisplaySpan { text: format!(" {}", ts) },
        ]}]
    }

    fn render_thinking(state: &crate::model::AppState, started: std::time::Instant) -> Vec<DisplayLine> {
        vec![DisplayLine { spans: vec![DisplaySpan {
            text: crate::labels::action_text(state.spinner_frame(), "Thinking", started.elapsed().as_secs_f64()),
        }]}]
    }

    fn render_thought_marker(content: &str) -> Vec<DisplayLine> {
        vec![DisplayLine { spans: vec![DisplaySpan { text: content.to_string() }]}]
    }

    fn render_thought_summary(content: &str) -> Vec<DisplayLine> {
        vec![DisplayLine { spans: vec![DisplaySpan {
            text: format!("{} [+]", content.lines().next().unwrap_or(content)),
        }]}]
    }

    fn render_tool_running(state: &crate::model::AppState, name: &str, started: std::time::Instant) -> Vec<DisplayLine> {
        vec![DisplayLine { spans: vec![DisplaySpan {
            text: format!("{} Running {}... {:.1}s", state.spinner_frame(), name, started.elapsed().as_secs_f64()),
        }]}]
    }

    fn render_tool_done(name: &str, duration_secs: f64, output: &str) -> Vec<DisplayLine> {
        let mut lines = vec![DisplayLine { spans: vec![DisplaySpan {
            text: format!("✓ {} {:.1}s", name, duration_secs),
        }]}];
        if !output.is_empty() {
            for line in output.lines() {
                lines.push(DisplayLine { spans: vec![DisplaySpan { text: line.to_string() }]});
            }
        }
        lines
    }

    fn render_tool_summary(name: &str, duration_secs: f64) -> Vec<DisplayLine> {
        vec![DisplayLine { spans: vec![DisplaySpan {
            text: format!("✓ {} {:.1}s [+]", name, duration_secs),
        }]}]
    }

    fn render_turn_complete(duration_secs: f64) -> Vec<DisplayLine> {
        vec![DisplayLine { spans: vec![DisplaySpan {
            text: format!("Turn completed in {:.1}s", duration_secs),
        }]}]
    }
}
