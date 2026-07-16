use crate::message::Part;
use crate::model::{AppState, ChatMessage, Role};
use crate::view::elements::{Element, Feed, PostKind};

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
            a.0.timestamp()
                .partial_cmp(&b.0.timestamp())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut feed = Feed::new();
        for i in 0..entries.len() {
            let (elem, collapsible) = entries.get_mut(i).expect("entries bound by loop");
            let ts = elem.timestamp();
            // Only agent thoughts/reasoning collapse to one-line summaries
            // (grok parity). System messages (trust banner, /sessions,
            // compaction summaries) reuse the thought element for styling
            // but must always render in full.
            let elem = if *collapsible {
                Self::maybe_collapse_thought(elem.clone(), state, feed.post_count())
            } else {
                elem.clone()
            };
            let elem = Self::maybe_expand_subagent(elem, state, feed.post_count());
            let kind = Self::post_kind(&elem);
            let expanded = match &elem {
                Element::ThoughtSummary { .. } | Element::ToolSummary { .. } => false,
                // Running workers have no body, so they are never collapsible;
                // finished workers collapse to the one-line summary unless the
                // post was individually expanded.
                Element::SubagentRow {
                    status, expanded, ..
                } => *expanded || matches!(status, crate::model::PatternWorkerStatus::Running),
                _ => true,
            };

            // Stack consecutive sub-agent lifecycle rows without blank lines
            // between them (GROK.md §26).
            let next_is_subagent = entries
                .get(i + 1)
                .map(|(e, _)| Self::post_kind(e) == PostKind::SubagentRow)
                .unwrap_or(false);
            let trailing_spacer =
                !(kind == PostKind::SubagentRow && next_is_subagent);

            feed.push_post(
                crate::view::posts::PostBuilder::new(kind)
                    .with_element(elem)
                    .expanded(expanded)
                    .trailing_spacer(trailing_spacer)
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
            E::ThoughtMarker { .. } | E::ThoughtSummary { .. } | E::AnthropicThinking { .. } => PostKind::Thought,
            E::ToolRunning { .. } => PostKind::ToolRunning,
            E::ToolDone { .. } => PostKind::ToolDone,
            E::ToolSummary { .. } => PostKind::ToolSummary,
            E::ToolConfirmation { .. } => PostKind::ToolRunning,
            E::ContextGroup { .. } => PostKind::ContextGroup,
            E::SubagentRow { .. } => PostKind::SubagentRow,
            E::TurnComplete { .. } => PostKind::TurnComplete,
            E::Image { .. } => PostKind::AgentResponse,
            E::DataPart { .. } => PostKind::AgentResponse,
            E::MarkdownTable { .. } => PostKind::AgentResponse,
            E::DiffOutput { .. } => PostKind::AgentResponse,
            E::WebSearchCall { .. } => PostKind::AgentResponse,
            E::AnsiStyled { .. } => PostKind::AgentResponse,
        }
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
        let current = state
            .agent_state()
            .current_request_id
            .as_ref()
            .and_then(|id| {
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

    /// Timestamp for swarm worker lifecycle rows.
    ///
    /// Worker rows belong to the current turn and must stay above the final
    /// assistant response. Anchoring to the last user message (the one that
    /// started the turn) guarantees that; falling back to `thinking_timestamp`
    /// keeps the existing behavior when no user message is present.
    fn worker_row_timestamp(state: &AppState) -> f64 {
        let last_user_ts = state
            .session
            .messages
            .iter()
            .filter(|m| m.role == Role::User)
            .map(|m| m.timestamp)
            .last();
        last_user_ts
            .map(|t| t + 1e-7)
            .unwrap_or_else(|| Self::thinking_timestamp(state))
    }

    /// Collect renderable entries. Each element carries a `collapsible`
    /// flag: true only for agent thoughts and streamed reasoning (the
    /// grok-style one-line summary applies to them exclusively).
    fn collect_entries(state: &AppState) -> Vec<(Element, bool)> {
        let mut entries: Vec<(Element, bool)> = Vec::new();

        for msg in state.session().messages.iter() {
            if Self::should_skip_msg(msg, state) {
                continue;
            }
            // Always show TurnComplete (like Grok does)
            entries.extend(Self::msg_to_elem(msg, state));
        }

        // Swarm worker lifecycle rows for the current turn, injected before
        // the thinking row (same sort timestamp; the stable sort keeps them
        // above "Waiting for response…" in spawn order).
        for row in &state.agent_state().pattern_workers {
            entries.push((
                Element::subagent_row(
                    row.id.clone(),
                    row.description.clone(),
                    row.model.clone(),
                    row.status,
                    matches!(row.status, crate::model::PatternWorkerStatus::Running)
                        .then_some(row.started),
                    row.duration_ms,
                    row.activity.clone(),
                    row.output.clone(),
                )
                .at(Self::worker_row_timestamp(state)),
                false,
            ));
        }

        if let Some(started) = state.agent_state().thinking_started_at {
            entries.push((
                Element::thinking(started).at(Self::thinking_timestamp(state)),
                false,
            ));
        }

        entries
    }

    fn group_context_tools(entries: Vec<(Element, bool)>, collapsed: bool) -> Vec<(Element, bool)> {
        const CONTEXT_TOOLS: &[&str] = &["read_file", "list_dir", "grep", "find", "fetch_docs"];
        let mut out = Vec::with_capacity(entries.len());
        let mut group: Vec<Element> = Vec::new();

        for (elem, collapsible) in entries {
            if Self::is_context_tool(&elem, CONTEXT_TOOLS) {
                group.push(elem);
                continue;
            }
            if !group.is_empty() {
                out.push((
                    Self::flush_context_group(std::mem::take(&mut group), collapsed),
                    false,
                ));
            }
            out.push((elem, collapsible));
        }
        if !group.is_empty() {
            out.push((Self::flush_context_group(group, collapsed), false));
        }
        out
    }

    fn flush_context_group(group: Vec<Element>, collapsed: bool) -> Element {
        if group.len() > 1 {
            let ts = group.iter().map(|e| e.timestamp()).fold(0.0, f64::max);
            Element::context_group(group, collapsed).at(ts)
        } else {
            group.into_iter().next().unwrap()
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
        let skip = is_tool_call_msg
            || (state.agent_state().thinking_started_at.is_some()
                && state.agent_state().current_request_id.as_deref() == Some(&msg.id));
        skip
    }

    pub fn visible(cache: &[Element], skip: usize, take: usize) -> &[Element] {
        let start = skip.min(cache.len());
        let end = (start + take).min(cache.len());
        &cache[start..end]
    }

    fn msg_to_elem(msg: &ChatMessage, state: &AppState) -> Vec<(Element, bool)> {
        let ts = msg.timestamp;
        match msg.role {
            Role::User => vec![(Element::user(msg.content()).at(ts), false)],
            Role::Thought => vec![(Self::thought_elem(msg, state, ts), true)],
            Role::Assistant => Self::assistant_elems(msg, state, ts),
            Role::Tool => vec![(Self::tool_elem(msg, state, ts), false)],
            Role::TurnComplete => {
                vec![(
                    Element::turn_complete(Self::parse_dur(&msg.content())).at(ts),
                    false,
                )]
            } // filtered in collect_entries
            // System messages (trust banner, /sessions, compaction summary)
            // reuse the thought element for styling but are never collapsed.
            Role::System => vec![(Element::thought(msg.content()).at(ts), false)],
        }
    }

    fn assistant_elems(msg: &ChatMessage, state: &AppState, ts: f64) -> Vec<(Element, bool)> {
        if msg.parts.is_empty() {
            return vec![(
                Element::AgentMessage {
                    content: crate::update::strip_tool_markers(&msg.content()),
                    timestamp: ts,
                    provider: msg.provider.clone(),
                },
                false,
            )];
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
    ) -> Option<(Element, bool)> {
        match part {
            Part::Text { content } => Some((Self::text_elem(content, ts, provider), false)),
            Part::Reasoning { content } => Some((Self::reasoning_elem(content, state, ts), true)),
            Part::ToolCall { name, args, .. } => {
                Some((Self::tool_call_elem(name, args, ts), false))
            }
            Part::ReasoningEncrypted { data, signature } => {
                Some((Self::redacted_thinking_elem(data, signature.as_deref(), ts), true))
            }
            Part::AnthropicThinking { content, signature } => {
                Some((Self::anthropic_thinking_elem(content, signature.as_deref(), ts), true))
            }
            Part::ToolResult { output, .. } => Some((Self::tool_result_elem(output, ts), false)),
            Part::ToolConfirmation { id, name, args, description, .. } => {
                Some((Self::tool_confirmation_elem(id, name, args, description.as_deref(), ts), false))
            }
            Part::Image { data, mime_type } => {
                Some((Self::image_elem(data, mime_type, ts), false))
            }
            Part::Data { data, format } => {
                Some((Self::data_part_elem(data, format.as_deref(), ts), false))
            }
            Part::WebSearch { query } => {
                Some((Self::web_search_elem(query, ts), false))
            }
            Part::Diff { content, diff_type } => {
                let dt = match diff_type.as_str() {
                    "side_by_side" => crate::view::elements::DiffType::SideBySide,
                    "context" => crate::view::elements::DiffType::Context,
                    _ => crate::view::elements::DiffType::Unified,
                };
                Some((Self::diff_output_elem(content, dt, ts), false))
            }
            Part::Ansi { raw, plain } => {
                Some((Self::ansi_styled_elem(raw, plain, ts), false))
            }
        }
    }

    fn text_elem(content: &str, ts: f64, provider: &str) -> Element {
        Element::AgentMessage {
            content: crate::update::strip_tool_markers(content),
            timestamp: ts,
            provider: provider.to_owned(),
        }
    }

    fn reasoning_elem(content: &str, _state: &AppState, ts: f64) -> Element {
        // Always emit the full body here; collapsing to a one-line summary
        // happens per-post in `build()` so individually expanded posts can
        // keep their reasoning visible.
        Element::thought(content.to_owned()).at(ts)
    }

    fn tool_call_elem(name: &str, args: &serde_json::Value, ts: f64) -> Element {
        let args_compact = crate::tool::compact_json_args(args);
        Element::tool_done(name, args_compact, 0.0, String::new(), None, false).at(ts)
    }

    fn tool_result_elem(output: &str, ts: f64) -> Element {
        Element::tool_done("tool", String::new(), 0.0, output, None, false).at(ts)
    }

    fn redacted_thinking_elem(data: &str, _signature: Option<&str>, ts: f64) -> Element {
        Element::redacted_thinking(data.to_owned()).at(ts)
    }

    fn anthropic_thinking_elem(content: &str, signature: Option<&str>, ts: f64) -> Element {
        Element::anthropic_thinking(content, signature.map(|s| s.to_owned())).at(ts)
    }

    fn tool_confirmation_elem(
        request_id: &str,
        name: &str,
        args: &serde_json::Value,
        description: Option<&str>,
        ts: f64,
    ) -> Element {
        let args_compact = crate::tool::compact_json_args(args);
        let desc = description.map(|s| s.to_owned()).unwrap_or_else(|| format!("Run {} with args", name));
        Element::tool_confirmation(request_id, name, args_compact, desc).at(ts)
    }

    fn image_elem(data: &str, mime_type: &str, ts: f64) -> Element {
        Element::image(data, mime_type).at(ts)
    }

    fn data_part_elem(data: &str, format_string: Option<&str>, ts: f64) -> Element {
        Element::data_part(data, format_string.map(|s| s.to_owned())).at(ts)
    }

    fn web_search_elem(query: &str, ts: f64) -> Element {
        Element::web_search_call(query, vec![]).at(ts)
    }

    fn diff_output_elem(content: &str, diff_type: crate::view::elements::DiffType, ts: f64) -> Element {
        Element::diff_output(content, diff_type).at(ts)
    }

    fn ansi_styled_elem(raw: &str, plain: &str, ts: f64) -> Element {
        Element::ansi_styled(raw, plain).at(ts)
    }

    fn thought_elem(msg: &ChatMessage, _state: &AppState, ts: f64) -> Element {
        let content = msg.content();
        // Show "Thought for Xs" as a summary (like Grok does)
        if Self::is_duration_only_thought(&content) {
            // Duration-only thoughts have no body: show the summary line
            // without a (dead) expand affordance, even when collapsed.
            return Element::thought_summary_static(
                content.clone(),
                Self::parse_thought_dur(&content),
            )
            .at(ts);
        }
        // Full body here; `build()` collapses per-post as needed.
        Element::thought(content).at(ts)
    }

    /// Collapse a full thought element to its one-line summary. Thoughts are
    /// collapsed BY DEFAULT (grok parity: "Thinking Block — collapsed by
    /// default, toggle with Ctrl+E"); a post individually expanded with
    /// Enter in feed navigation keeps its full body.
    fn maybe_collapse_thought(elem: Element, state: &AppState, post_index: usize) -> Element {
        if state.view().expanded_posts.contains(&post_index) {
            return elem;
        }
        match elem {
            Element::ThoughtMarker { content, timestamp } => {
                let first_line = content.lines().next().unwrap_or(&content).to_owned();
                Element::thought_summary(first_line, Self::parse_thought_dur(&content))
                    .at(timestamp)
            }
            Element::AnthropicThinking { content, signature, redacted, timestamp } => {
                let first_line = content.lines().next().unwrap_or(&content).to_owned();
                let summary = if redacted {
                    format!("[Redacted thinking - {} chars]", content.len())
                } else {
                    first_line
                };
                Element::AnthropicThinking {
                    content: summary,
                    signature,
                    redacted,
                    timestamp,
                }
            }
            _ => elem,
        }
    }

    /// Mark a subagent row expanded when its post was individually expanded
    /// with Enter in feed navigation. Running rows never expand (no output).
    fn maybe_expand_subagent(mut elem: Element, state: &AppState, post_index: usize) -> Element {
        if let Element::SubagentRow {
            expanded, status, ..
        } = &mut elem
        {
            *expanded = !matches!(status, crate::model::PatternWorkerStatus::Running)
                && state.view().expanded_posts.contains(&post_index);
        }
        elem
    }

    /// Check if the thought is just a duration marker like "Thought for 0.2s"
    fn is_duration_only_thought(content: &str) -> bool {
        let trimmed = content.trim();
        trimmed.starts_with("◆ Thought for ") && !trimmed.contains('\n')
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
