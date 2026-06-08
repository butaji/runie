use crate::labels::format_timestamp;
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
        let entries = Self::collect_entries(state);
        let mut feed = Feed::new();
        for (_, _, elem, _, _) in entries {
            feed.elements.push(elem);
            feed.elements.push(Element::Spacer);
        }
        feed
    }

    fn collect_entries(state: &AppState) -> Vec<(f64, usize, Element, String, String)> {
        let mut entries: Vec<(f64, usize, Element, String, String)> = Vec::new();

        for (idx, msg) in state.messages.iter().enumerate() {
            if Self::should_skip_msg(msg, state) {
                continue;
            }
            let elem = Self::msg_to_elem(msg, state);
            let request_id = msg.id.split('#').next().unwrap_or(&msg.id).to_string();
            entries.push((msg.timestamp, idx, elem, msg.id.clone(), request_id));
        }

        Self::add_thinking_if_active(state, &mut entries);

        entries.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
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
            && state.current_request_id.as_deref() == Some(&msg.id)
    }

    fn add_thinking_if_active(state: &AppState, entries: &mut Vec<(f64, usize, Element, String, String)>) {
        if let Some(started) = state.thinking_started_at {
            let max_ts = state.messages.iter().map(|m| m.timestamp).fold(0.0, f64::max);
            entries.push((max_ts + 1.0, usize::MAX, Element::Thinking { started }, String::new(), String::new()));
        }
    }

    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (start + take).min(cache.len());
        &cache[start..end]
    }

    fn msg_to_elem(msg: &ChatMessage, state: &AppState) -> Element {
        let ts = format_timestamp(msg.timestamp);
        match msg.role {
            Role::User => Element::UserMessage { content: msg.content.clone(), timestamp: ts },
            Role::Thought => {
                if state.all_collapsed {
                    let first_line = msg.content.lines().next().unwrap_or(&msg.content).to_string();
                    let dur = Self::parse_thought_dur(&msg.content);
                    Element::ThoughtSummary { content: first_line, duration_secs: dur }
                } else {
                    Element::ThoughtMarker { content: msg.content.clone() }
                }
            }
            Role::Assistant => Element::AgentMessage { content: crate::update::strip_tool_markers(&msg.content), timestamp: ts },
            Role::Tool => Self::tool_elem(msg, state),
            Role::TurnComplete => Element::TurnComplete { duration_secs: Self::parse_dur(&msg.content) },
            Role::System => Element::ThoughtMarker { content: msg.content.clone() },
        }
    }

    fn parse_thought_dur(content: &str) -> f64 {
        content.split_whitespace().last()
            .and_then(|s| s.trim_end_matches('s').parse().ok())
            .unwrap_or(0.0)
    }

    fn tool_elem(msg: &ChatMessage, state: &AppState) -> Element {
        if msg.content.contains("Running") {
            let name = msg.content.trim_start_matches("⠋ Running ").trim_end_matches("...");
            Element::ToolRunning { name: name.to_string(), started: state.tool_started_at.unwrap_or_else(std::time::Instant::now) }
        } else {
            let lines: Vec<&str> = msg.content.lines().collect();
            let header = lines.first().copied().unwrap_or("");
            let output = lines.get(1..).map(|rest| rest.join("\n")).unwrap_or_default();
            let parts: Vec<&str> = header.split_whitespace().collect();
            // Old format: "Ran {name} {duration}s" -> name at index 1
            // New format: "✓ {name} {duration}s" -> name at index 1
            let name = parts.get(1).unwrap_or(&"").to_string();
            let dur = parts.last().and_then(|s| s.trim_end_matches('s').parse().ok()).unwrap_or(0.0);
            if state.all_collapsed {
                Element::ToolSummary { name, duration_secs: dur }
            } else {
                Element::ToolDone { name, duration_secs: dur, output }
            }
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
            Element::Spacer => vec![DisplayLine { spans: vec![] }],
            Element::UserMessage { content, .. } => render_user(content),
            Element::AgentMessage { content, .. } => render_agent(content),
            Element::Thinking { started } => render_thinking(state, *started),
            Element::ThoughtMarker { content } => render_thought_marker(content),
            Element::ThoughtSummary { content, .. } => render_thought_summary(content),
            Element::ToolRunning { name, started } => render_tool_running(state, name, *started),
            Element::ToolDone { name, duration_secs, output } => render_tool_done(name, *duration_secs, output),
            Element::ToolSummary { name, duration_secs } => render_tool_summary(name, *duration_secs),
            Element::TurnComplete { duration_secs } => render_turn_complete(*duration_secs),
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
