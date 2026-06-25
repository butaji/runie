use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::view::elements::{Element, Feed};

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
        let mut entries = Self::group_context_tools(entries, state.view().all_collapsed);
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
                crate::view::posts::PostBuilder::new(kind)
                    .with_element(elem)
                    .expanded(expanded)
                    .at(ts),
            );
        }
        feed
    }

    fn post_kind(elem: &Element) -> crate::view::elements::PostKind {
        use crate::view::elements::{Element as E, PostKind};
        match elem {
            E::Spacer { .. } => PostKind::System,
            E::UserMessage { .. } => PostKind::UserInput,
            E::AgentMessage { .. } => PostKind::AgentResponse,
            E::Thinking { .. } => PostKind::Thinking,
            E::ThoughtMarker { .. } | E::ThoughtSummary { .. } => PostKind::Thought,
            E::ToolRunning { .. } => PostKind::ToolRunning,
            E::ToolDone { .. } => PostKind::ToolDone,
            E::ToolSummary { .. } => PostKind::ToolSummary,
            E::ContextGroup { .. } => PostKind::ContextGroup,
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
        for msg in state.session().messages.iter() {
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
        let current = state.agent_state().current_request_id.as_ref().and_then(|id| {
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

        for msg in state.session().messages.iter() {
            if Self::should_skip_msg(msg, state) {
                continue;
            }
            if msg.role == Role::TurnComplete {
                let count = action_counts.get(&msg.id).copied().unwrap_or(0);
                if count <= 1 {
                    continue;
                }
            }
            entries.extend(Self::msg_to_elem(msg, state));
        }

        if let Some(started) = state.agent_state().thinking_started_at {
            entries.push(Element::thinking(started).at(Self::thinking_timestamp(state)));
        }

        entries
    }

    fn group_context_tools(entries: Vec<Element>, collapsed: bool) -> Vec<Element> {
        const CONTEXT_TOOLS: &[&str] = &["read_file", "list_dir", "grep", "find", "fetch_docs"];
        let mut out = Vec::with_capacity(entries.len());
        let mut group: Vec<Element> = Vec::new();

        for elem in entries {
            if Self::is_context_tool(&elem, CONTEXT_TOOLS) {
                group.push(elem);
                continue;
            }
            if !group.is_empty() {
                out.extend(Self::flush_context_group(std::mem::take(&mut group), collapsed));
            }
            out.push(elem);
        }
        if !group.is_empty() {
            out.extend(Self::flush_context_group(group, collapsed));
        }
        out
    }

    fn flush_context_group(group: Vec<Element>, collapsed: bool) -> Vec<Element> {
        if group.len() > 1 {
            let ts = group
                .iter()
                .map(|e| e.timestamp())
                .fold(0.0, f64::max);
            vec![Element::context_group(group, collapsed).at(ts)]
        } else {
            group
        }
    }

    fn is_context_tool(elem: &Element, context_tools: &[&str]) -> bool {
        let name = match elem {
            Element::ToolDone { name, .. } | Element::ToolSummary { name, .. } => name,
            _ => return false,
        };
        context_tools.contains(&name.as_str())
    }

    fn should_skip_msg(msg: &ChatMessage, state: &AppState) -> bool {
        if msg.role != Role::Assistant {
            return false;
        }
        // Skip if message has only tool calls (no text or reasoning content)
        let has_text_or_reasoning = msg.parts.iter().any(|p| {
            matches!(p, Part::Text { content } if !content.is_empty())
                || matches!(p, Part::Reasoning { content } if !content.is_empty())
        });
        let is_tool_only = !msg.tool_calls().is_empty() && !has_text_or_reasoning;
        let has_tool_markers = crate::update::content_has_tool_markers(&msg.content());
        let is_tool_call_msg = is_tool_only || has_tool_markers;
        is_tool_call_msg
            || (state.agent_state().thinking_started_at.is_some()
                && state.agent_state().current_request_id.as_deref() == Some(&msg.id))
    }

    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (start + take).min(cache.len());
        &cache[start..end]
    }

    fn msg_to_elem(msg: &ChatMessage, state: &AppState) -> Vec<Element> {
        let ts = msg.timestamp;
        match msg.role {
            Role::User => vec![Element::user(msg.content()).at(ts)],
            Role::Thought => vec![Self::thought_elem(msg, state, ts)],
            Role::Assistant => Self::assistant_elems(msg, state, ts),
            Role::Tool => vec![Self::tool_elem(msg, state, ts)],
            Role::TurnComplete => {
                vec![Element::turn_complete(Self::parse_dur(&msg.content())).at(ts)]
            } // filtered in collect_entries
            Role::System => vec![Element::thought(msg.content()).at(ts)],
        }
    }

    fn assistant_elems(msg: &ChatMessage, state: &AppState, ts: f64) -> Vec<Element> {
        if msg.parts.is_empty() {
            return vec![Element::AgentMessage {
                content: crate::update::strip_tool_markers(&msg.content()),
                timestamp: ts,
                provider: msg.provider.clone(),
            }];
        }

        msg.parts
            .iter()
            .filter_map(|part| Self::part_to_element(part, state, ts, &msg.provider))
            .collect()
    }

    fn part_to_element(
        part: &Part,
        state: &AppState,
        ts: f64,
        provider: &str,
    ) -> Option<Element> {
        match part {
            Part::Text { content } => Some(Self::text_elem(content, ts, provider)),
            Part::Reasoning { content } => Some(Self::reasoning_elem(content, state, ts)),
            Part::ToolCall { name, args, .. } => Some(Self::tool_call_elem(name, args, ts)),
            Part::ToolResult { output, .. } => Some(Self::tool_result_elem(output, ts)),
        }
    }

    fn text_elem(content: &str, ts: f64, provider: &str) -> Element {
        Element::AgentMessage {
            content: crate::update::strip_tool_markers(content),
            timestamp: ts,
            provider: provider.to_string(),
        }
    }

    fn reasoning_elem(content: &str, state: &AppState, ts: f64) -> Element {
        if state.view().all_collapsed {
            let first_line = content.lines().next().unwrap_or(content).to_string();
            Element::thought_summary(first_line, 0.0).at(ts)
        } else {
            Element::thought(content.to_string()).at(ts)
        }
    }

    fn tool_call_elem(name: &str, args: &serde_json::Value, ts: f64) -> Element {
        let args_compact = crate::tool::compact_json_args(args);
        Element::tool_done(name, args_compact, 0.0, String::new(), None, false).at(ts)
    }

    fn tool_result_elem(output: &str, ts: f64) -> Element {
        Element::tool_done("tool", String::new(), 0.0, output, None, false).at(ts)
    }

    fn thought_elem(msg: &ChatMessage, state: &AppState, ts: f64) -> Element {
        let content = msg.content();
        if state.view().all_collapsed {
            let first_line = content.lines().next().unwrap_or(&content).to_string();
            Element::thought_summary(first_line, Self::parse_thought_dur(&content)).at(ts)
        } else {
            Element::thought(content).at(ts)
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
        let content = msg.content();
        if content.contains("⠋ Running ") {
            let name = content
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
        let (name, dur, output) = Self::parse_tool_content(&content);
        if state.view().all_collapsed {
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
