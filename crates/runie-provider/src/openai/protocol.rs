//! OpenAI streaming protocol implementation.
//!
//! Transforms OpenAI SSE frames (`OpenAiFrame`) into `ProviderEvent`s.

use super::types::{ChunkJson, ToolCallJson};
use crate::protocol::ProviderProtocol;
use runie_core::provider_event::{ProviderEvent, StopReason};
use runie_core::tool::shim::json::find_object_end;
use runie_core::tool::shim::{is_tool_call_value, parse_minimax_tool_calls};
use runie_core::tool::ParsedToolCall;
use std::collections::{BTreeMap, HashSet};

// ============================================================================
// Lifecycle state machine
// ============================================================================

/// Tracks open text/thinking blocks and emits lifecycle events.
///
/// This is the OpenAI-specific lifecycle tracker. It emits `TextStart`/`TextEnd`
/// and `ThinkingStart`/`ThinkingEnd` events so downstream consumers can build
/// proper `Vec<Part>` content during streaming.
#[derive(Debug, Default)]
pub(crate) struct LifecycleState {
    open_text_blocks: HashSet<String>,
    open_thinking_blocks: HashSet<String>,
}

impl LifecycleState {
    /// Process a text delta and emit lifecycle events.
    ///
    /// Returns `TextStart` on first delta for this `id`, then `TextDelta`.
    fn text_delta(&mut self, id: &str, delta: &str) -> Vec<ProviderEvent> {
        let is_new = self.open_text_blocks.insert(id.to_owned());
        let mut events = Vec::new();
        if is_new {
            events.push(ProviderEvent::TextStart { id: id.to_owned() });
        }
        events.push(ProviderEvent::TextDelta(delta.to_owned()));
        events
    }

    /// Explicitly close a text block and emit `TextEnd`.
    #[allow(dead_code)]
    fn text_end(&mut self, id: &str) -> Vec<ProviderEvent> {
        self.open_text_blocks.remove(id);
        vec![ProviderEvent::TextEnd { id: id.to_owned() }]
    }

    /// Process a thinking delta and emit lifecycle events.
    ///
    /// Returns `ThinkingStart` on first delta for this `id`, then `ThinkingDelta`.
    fn thinking_delta(&mut self, id: &str, delta: &str) -> Vec<ProviderEvent> {
        let is_new = self.open_thinking_blocks.insert(id.to_owned());
        let mut events = Vec::new();
        if is_new {
            events.push(ProviderEvent::ThinkingStart { id: id.to_owned() });
        }
        events.push(ProviderEvent::ThinkingDelta(delta.to_owned()));
        events
    }

    /// Explicitly close a thinking block and emit `ThinkingEnd`.
    #[allow(dead_code)]
    fn thinking_end(&mut self, id: &str) -> Vec<ProviderEvent> {
        self.open_thinking_blocks.remove(id);
        vec![ProviderEvent::ThinkingEnd { id: id.to_owned() }]
    }

    /// Close all open blocks and emit their end events, plus `Finish`.
    fn finish(&mut self, reason: StopReason) -> Vec<ProviderEvent> {
        let mut events = Vec::new();
        for id in self.open_text_blocks.drain() {
            events.push(ProviderEvent::TextEnd { id });
        }
        for id in self.open_thinking_blocks.drain() {
            events.push(ProviderEvent::ThinkingEnd { id });
        }
        events.push(ProviderEvent::Finish { reason });
        events
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[test]
    fn lifecycle_emits_start_on_first_delta() {
        let mut state = LifecycleState::default();
        let events = state.text_delta("b1", "hi");
        assert_eq!(
            events,
            vec![
                ProviderEvent::TextStart { id: "b1".into() },
                ProviderEvent::TextDelta("hi".into())
            ]
        );
    }

    #[test]
    fn lifecycle_skips_start_on_continuation() {
        let mut state = LifecycleState::default();
        state.text_delta("b1", "hi");
        let events = state.text_delta("b1", " world");
        assert_eq!(events, vec![ProviderEvent::TextDelta(" world".into())]);
    }

    #[test]
    fn lifecycle_finish_closes_all_open_blocks() {
        let mut state = LifecycleState::default();
        state.text_delta("t1", "hello");
        state.text_delta("t2", "world");
        state.thinking_delta("r1", "thinking");
        let events = state.finish(StopReason::Stop);
        assert!(events.contains(&ProviderEvent::TextEnd { id: "t1".into() }));
        assert!(events.contains(&ProviderEvent::TextEnd { id: "t2".into() }));
        assert!(events.contains(&ProviderEvent::ThinkingEnd { id: "r1".into() }));
        assert!(events.contains(&ProviderEvent::Finish {
            reason: StopReason::Stop
        }));
        assert_eq!(events.len(), 4); // 3 End + 1 Finish
    }

    #[test]
    fn lifecycle_text_end_removes_from_open_set() {
        let mut state = LifecycleState::default();
        state.text_delta("b1", "hi");
        state.text_end("b1");
        let events = state.text_delta("b1", "x");
        assert_eq!(
            events,
            vec![
                ProviderEvent::TextStart { id: "b1".into() },
                ProviderEvent::TextDelta("x".into())
            ]
        );
    }

    #[test]
    fn lifecycle_thinking_delta_emits_thinking_start() {
        let mut state = LifecycleState::default();
        let events = state.thinking_delta("r1", "reasoning");
        assert_eq!(
            events,
            vec![
                ProviderEvent::ThinkingStart { id: "r1".into() },
                ProviderEvent::ThinkingDelta("reasoning".into())
            ]
        );
    }

    #[test]
    fn lifecycle_multiple_text_blocks_independent() {
        let mut state = LifecycleState::default();
        let e1 = state.text_delta("a", "hello");
        let e2 = state.text_delta("b", "world");
        // Each delta should emit exactly 2 events (Start + Delta)
        assert_eq!(e1.len(), 2);
        assert_eq!(e2.len(), 2);
        // First event of each should be TextStart
        match &e1[0] {
            ProviderEvent::TextStart { id } => assert_eq!(id, "a"),
            _ => panic!("Expected TextStart"),
        }
        match &e2[0] {
            ProviderEvent::TextStart { id } => assert_eq!(id, "b"),
            _ => panic!("Expected TextStart"),
        }
    }
}

// ============================================================================
// Streaming content filter (MiniMax reasoning + tool calls inside content)
// ============================================================================
//
// MiniMax puts reasoning (`<think>...</think>`) and tool calls
// (`<minimax:tool_call>...`, `<tool_call>...`, or inline
// `{"name","arguments"}` JSON) INSIDE `delta.content`. This filter routes those
// to the thinking / structured-tool paths so the live feed never renders the
// raw markup. It is a pure passthrough until a MiniMax marker is seen, so other
// OpenAI-compatible providers are unaffected.

/// A segment produced by [`ContentFilter`].
#[derive(Debug)]
enum ContentSegment {
    /// Plain visible text.
    Text(String),
    /// Reasoning extracted from a `<think>...</think>` block.
    Thinking(String),
    /// A tool call parsed from in-content markup or inline JSON.
    ToolCall(ParsedToolCall),
}

const OPEN_THINK: &str = "<think>";
const CLOSE_THINK: &str = "</think>";
const OPEN_MINIMAX: &str = "<minimax:tool_call>";
const CLOSE_MINIMAX: &str = "</minimax:tool_call>";
const OPEN_TOOL_CALL: &str = "<tool_call>";
const CLOSE_TOOL_CALL: &str = "</tool_call>";

/// Tags that activate the filter when seen inside `delta.content`.
const ACTIVATION_OPENERS: &[&str] = &[OPEN_THINK, OPEN_MINIMAX, OPEN_TOOL_CALL];

#[derive(Debug, Clone, Copy)]
enum Marker {
    Think,
    Minimax,
    ToolCall,
    Json,
}

/// Streaming filter for MiniMax-style `delta.content`. Buffers partial tags
/// across chunks and reuses the core complete-span parsers for the actual
/// tool-call parsing.
#[derive(Debug, Default)]
struct ContentFilter {
    buffer: String,
    activated: bool,
    tool_seq: usize,
}

impl ContentFilter {
    fn next_tool_id(&mut self) -> String {
        let id = format!("content_tool_{}", self.tool_seq);
        self.tool_seq += 1;
        id
    }

    /// Feed a content delta. `suppress_tools` strips (rather than emits) any
    /// in-content tool-call markup — used when the stream already carries the
    /// same call via the structured `tool_calls` field.
    fn feed(&mut self, delta: &str, suppress_tools: bool) -> Vec<ContentSegment> {
        self.buffer.push_str(delta);
        if !self.activated {
            if contains_activation_opener(&self.buffer) {
                self.activated = true;
            } else {
                // Pure passthrough: preserve the original behavior where any
                // (possibly empty) content delta opens the text block, holding
                // back only a trailing suffix that might be a partial opener.
                return self.emit_safe_prefix();
            }
        }
        self.process_activated(suppress_tools)
    }

    /// Flush remaining buffered content at end of stream.
    fn flush(&mut self, suppress_tools: bool) -> Vec<ContentSegment> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        if !self.activated {
            // A trailing partial-opener suffix that never completed: emit verbatim.
            return vec![ContentSegment::Text(std::mem::take(&mut self.buffer))];
        }
        // Drain any complete spans first; whatever remains is an incomplete span.
        let mut out = self.process_activated(suppress_tools);
        if !self.buffer.is_empty() {
            let buf = std::mem::take(&mut self.buffer);
            if let Some(rest) = buf.strip_prefix(OPEN_THINK) {
                // Unclosed `<think>`: the tail is reasoning to end of stream.
                if !rest.is_empty() {
                    out.push(ContentSegment::Thinking(rest.to_owned()));
                }
            } else {
                // Drop unterminated tool-call markup; emit whatever clean text
                // remains so nothing leaks into the live feed.
                let text = runie_core::tool_markers::strip_tool_markers(&buf);
                if !text.is_empty() {
                    out.push(ContentSegment::Text(text));
                }
            }
        }
        out
    }

    /// Not-yet-activated fast path: hold back only a trailing suffix that might
    /// be a partial opener; emit everything else as plain text. Always emits a
    /// (possibly empty) `Text` segment so the text block still opens on the
    /// first content chunk, matching the original passthrough behavior.
    fn emit_safe_prefix(&mut self) -> Vec<ContentSegment> {
        let hold = trailing_partial_opener_len(&self.buffer);
        let emit_len = self.buffer.len() - hold;
        let text = self.buffer[..emit_len].to_owned();
        self.buffer.drain(..emit_len);
        vec![ContentSegment::Text(text)]
    }

    fn process_activated(&mut self, suppress_tools: bool) -> Vec<ContentSegment> {
        let mut out = Vec::new();
        loop {
            let marker = self.earliest_marker();
            let Some((pos, kind)) = marker else {
                // No marker in buffer: emit safe prefix, hold trailing partial.
                let hold = trailing_partial_opener_len(&self.buffer);
                let emit_len = self.buffer.len() - hold;
                if emit_len > 0 {
                    let text = self.buffer[..emit_len].to_owned();
                    self.buffer.drain(..emit_len);
                    if !text.is_empty() {
                        out.push(ContentSegment::Text(text));
                    }
                }
                return out;
            };
            // Emit any plain text before the marker.
            if pos > 0 {
                let text = self.buffer[..pos].to_owned();
                self.buffer.drain(..pos);
                if !text.is_empty() {
                    out.push(ContentSegment::Text(text));
                }
            }
            // Buffer now starts with the marker.
            let consumed = match kind {
                Marker::Think => self.consume_think(&mut out),
                Marker::Minimax => {
                    self.consume_xml_tool(OPEN_MINIMAX, CLOSE_MINIMAX, suppress_tools, &mut out)
                }
                Marker::ToolCall => self.consume_xml_tool(
                    OPEN_TOOL_CALL,
                    CLOSE_TOOL_CALL,
                    suppress_tools,
                    &mut out,
                ),
                Marker::Json => self.consume_json_tool(suppress_tools, &mut out),
            };
            if !consumed {
                return out; // incomplete span — hold and await more data
            }
        }
    }

    /// Earliest marker position + kind in the current buffer.
    fn earliest_marker(&self) -> Option<(usize, Marker)> {
        let think = self.buffer.find(OPEN_THINK).map(|p| (p, Marker::Think));
        let minimax = self.buffer.find(OPEN_MINIMAX).map(|p| (p, Marker::Minimax));
        let tool = self
            .buffer
            .find(OPEN_TOOL_CALL)
            .map(|p| (p, Marker::ToolCall));
        let json = self.buffer.find('{').map(|p| (p, Marker::Json));
        [think, minimax, tool, json]
            .into_iter()
            .flatten()
            .min_by_key(|(p, _)| *p)
    }

    /// Consume a `<think>` block at the front of the buffer. Returns false if
    /// the block is incomplete (caller should hold and await more data).
    fn consume_think(&mut self, out: &mut Vec<ContentSegment>) -> bool {
        let after = OPEN_THINK.len();
        let close = self.buffer.find(CLOSE_THINK);
        let minimax = self.buffer.find(OPEN_MINIMAX);
        let tool = self.buffer.find(OPEN_TOOL_CALL);
        // A tool-call opener implicitly closes an unterminated `<think>`.
        let end = [close, minimax, tool].into_iter().flatten().min();
        let Some(end_pos) = end else {
            return false;
        };
        let inner = self.buffer[after..end_pos].to_owned();
        let consume = if Some(end_pos) == close {
            end_pos + CLOSE_THINK.len()
        } else {
            end_pos // leave the tool opener for re-processing
        };
        self.buffer.drain(..consume);
        if !inner.is_empty() {
            out.push(ContentSegment::Thinking(inner));
        }
        true
    }

    /// Consume an XML tool-call block (`<minimax:tool_call>` or `<tool_call>`)
    /// at the front of the buffer. Returns false if incomplete.
    fn consume_xml_tool(
        &mut self,
        open: &str,
        close: &str,
        suppress_tools: bool,
        out: &mut Vec<ContentSegment>,
    ) -> bool {
        let Some(rel_end) = self.buffer[open.len()..].find(close) else {
            return false;
        };
        let end = open.len() + rel_end + close.len();
        let span = self.buffer[..end].to_owned();
        self.buffer.drain(..end);
        if !suppress_tools {
            for result in parse_minimax_tool_calls(&span) {
                if let Ok(call) = result {
                    out.push(ContentSegment::ToolCall(call));
                }
            }
        }
        true
    }

    /// Consume an inline `{...}` JSON object at the front of the buffer. Returns
    /// false if the object is incomplete.
    fn consume_json_tool(
        &mut self,
        suppress_tools: bool,
        out: &mut Vec<ContentSegment>,
    ) -> bool {
        let Some(obj_end) = find_object_end(self.buffer.as_bytes(), 0) else {
            return false;
        };
        let slice = self.buffer[..=obj_end].to_owned();
        self.buffer.drain(..=obj_end);
        let parsed = serde_json::from_str::<serde_json::Value>(&slice).ok();
        let is_tool = parsed.as_ref().is_some_and(is_tool_call_value);
        match parsed {
            Some(v) if is_tool && !suppress_tools => {
                let name = v
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_owned();
                let args = v
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                if !name.is_empty() {
                    out.push(ContentSegment::ToolCall(ParsedToolCall {
                        name,
                        args,
                        id: None,
                    }));
                }
            }
            Some(_) if is_tool => {
                // Suppressed: the structured tool_calls field carries this call.
            }
            _ => out.push(ContentSegment::Text(slice)),
        }
        true
    }
}

fn contains_activation_opener(s: &str) -> bool {
    ACTIVATION_OPENERS.iter().any(|o| s.contains(o))
}

/// Length of the longest suffix of `s` that is a proper prefix of an opener.
fn trailing_partial_opener_len(s: &str) -> usize {
    let max = ACTIVATION_OPENERS
        .iter()
        .map(|o| o.chars().count().saturating_sub(1))
        .max()
        .unwrap_or(0)
        .min(s.chars().count());
    for len in (1..=max).rev() {
        let suffix: String = s.chars().rev().take(len).collect::<String>().chars().rev().collect();
        if ACTIVATION_OPENERS
            .iter()
            .any(|o| o.chars().count() > suffix.chars().count() && o.starts_with(&suffix))
        {
            return len;
        }
    }
    0
}

/// Map a content segment to provider events via the lifecycle tracker.
fn segment_to_events(seg: ContentSegment, state: &mut OpenAiState) -> Vec<ProviderEvent> {
    match seg {
        ContentSegment::Text(t) => state.lifecycle.text_delta("text", &t),
        // Route `<think>` reasoning into the same "reasoning" block used by the
        // `reasoning_content` field so there is a single, deterministic
        // thinking block (a HashSet-drained second block would be flaky).
        ContentSegment::Thinking(t) => state.lifecycle.thinking_delta("reasoning", &t),
        ContentSegment::ToolCall(tc) => {
            let id = state.content_filter.next_tool_id();
            let args = serde_json::to_string(&tc.args).unwrap_or_else(|_| "{}".to_owned());
            vec![
                ProviderEvent::ToolCallStart {
                    id: id.clone(),
                    name: tc.name,
                },
                ProviderEvent::ToolCallInputDelta {
                    id: id.clone(),
                    delta: args,
                },
                ProviderEvent::ToolCallEnd { id },
            ]
        }
    }
}

/// Flush any buffered content through the filter at end of stream.
fn finalize_content(state: &mut OpenAiState) -> Vec<ProviderEvent> {
    let suppress = state.has_structured_tools;
    let segs = state.content_filter.flush(suppress);
    let mut events = Vec::new();
    for seg in segs {
        // See process_openai_chunk: native reasoning wins over inline markup.
        if state.native_reasoning_seen && matches!(seg, ContentSegment::Thinking(_)) {
            continue;
        }
        events.extend(segment_to_events(seg, state));
    }
    events
}

/// A delta of content in an OpenAI chunk.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Delta {
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallDelta>,
}

/// A delta for a tool call.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

impl From<ToolCallDelta> for runie_core::proto::message::ToolCall {
    fn from(delta: ToolCallDelta) -> Self {
        let args: serde_json::Value = delta
            .arguments
            .as_ref()
            .and_then(|a| serde_json::from_str(a).ok())
            .unwrap_or(serde_json::Value::Null);
        runie_core::proto::message::ToolCall {
            id: delta.id.unwrap_or_default(),
            name: delta.name.unwrap_or_default(),
            args,
        }
    }
}

/// An OpenAI SSE chunk.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Chunk {
    pub delta: Delta,
    pub finish_reason: Option<String>,
    pub usage: Option<(usize, usize)>,
}

/// OpenAI protocol state machine.
#[derive(Debug)]
pub struct OpenAiProtocol;

impl OpenAiProtocol {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenAiProtocol {
    fn default() -> Self {
        Self::new()
    }
}

/// Protocol state for OpenAI streaming.
#[derive(Debug, Default)]
pub struct OpenAiState {
    /// Tool call accumulators indexed by call index.
    pub tools: BTreeMap<usize, ToolAccum>,
    /// IDs of tools that have started (received first args delta).
    pub started: HashSet<String>,
    /// IDs of tools that have ended (emitted ToolCallEnd).
    pub ended: HashSet<String>,
    lifecycle: LifecycleState,
    /// Streaming filter for MiniMax-style reasoning/tool-call markup in content.
    content_filter: ContentFilter,
    /// Whether the stream emitted any structured `tool_calls` (so redundant
    /// in-content tool-call markup is suppressed rather than duplicated).
    has_structured_tools: bool,
    /// Whether any chunk carried a native `reasoning`/`reasoning_content`
    /// field. MiniMax sends the same reasoning text twice per chunk — inline
    /// in `content` wrapped in `<think>` tags and in the native field — so
    /// once the native field appears, content-derived thinking segments are
    /// dropped to avoid duplicated reasoning.
    native_reasoning_seen: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ToolAccum {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl From<ToolAccum> for runie_core::proto::message::ToolCall {
    fn from(acc: ToolAccum) -> Self {
        let args: serde_json::Value =
            serde_json::from_str(&acc.arguments).unwrap_or(serde_json::Value::Null);
        runie_core::proto::message::ToolCall {
            id: acc.id,
            name: acc.name,
            args,
        }
    }
}

/// A parsed OpenAI SSE frame.
#[derive(Debug, Clone, PartialEq)]
pub enum OpenAiFrame {
    Chunk(Chunk),
    Done,
    /// Provider error delivered as an SSE data frame (HTTP 200), e.g.
    /// MiniMax/Anthropic `{"type":"error","error":{"type":"overloaded_error",…}}`
    /// or OpenAI `{"error":{…}}`. Terminal; retryable variants are retried
    /// at the whole-request level by the stream driver.
    Error(runie_core::provider_event::ModelError),
}

impl OpenAiFrame {
    pub fn from_line(line: &str) -> Option<Self> {
        // Accept both raw SSE lines ("data: {...}") and already-stripped JSON.
        let json_str = if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                return Some(OpenAiFrame::Done);
            }
            data
        } else {
            line
        };
        let json: serde_json::Value = serde_json::from_str(json_str).ok()?;
        // Error payloads arrive without `choices`: Anthropic-style
        // `{"type":"error",…}` or OpenAI-style `{"error":{…}}`.
        let is_error = json.get("type").and_then(|t| t.as_str()) == Some("error")
            || (json.get("error").is_some() && json.get("choices").is_none());
        if is_error {
            return Some(OpenAiFrame::Error(
                super::stream::classify_error_value(&json),
            ));
        }
        parse_chunk(&json).map(OpenAiFrame::Chunk)
    }
}

/// OpenAI protocol implementation.
impl ProviderProtocol for OpenAiProtocol {
    type Frame = OpenAiFrame;
    type State = OpenAiState;

    fn step(&self, state: Self::State, frame: Self::Frame) -> (Self::State, Vec<ProviderEvent>) {
        match frame {
            OpenAiFrame::Chunk(chunk) => {
                let mut new_state = state;
                let events = process_openai_chunk(chunk, &mut new_state);
                (new_state, events)
            }
            OpenAiFrame::Done => {
                let mut new_state = state;
                let mut events = finalize_content(&mut new_state);
                events.extend(flush_tool_calls(&mut new_state));
                events.extend(new_state.lifecycle.finish(StopReason::Stop));
                (new_state, events)
            }
            OpenAiFrame::Error(err) => (state, vec![ProviderEvent::Error(err)]),
        }
    }

    fn on_halt(&self, state: Self::State) -> Vec<ProviderEvent> {
        let mut new_state = state;
        let mut events = finalize_content(&mut new_state);
        events.extend(flush_tool_calls(&mut new_state));
        // Emit Finish to ensure the stream always terminates properly.
        // This handles cases where SSE stream ends without a terminal frame (e.g., data: [DONE]).
        events.extend(new_state.lifecycle.finish(StopReason::Stop));
        events
    }

    fn terminal(&self, frame: &Self::Frame) -> bool {
        matches!(frame, OpenAiFrame::Done | OpenAiFrame::Error(_))
    }
}

fn parse_chunk(json: &serde_json::Value) -> Option<Chunk> {
    let json_chunk: ChunkJson = serde_json::from_value(json.clone()).ok()?;
    let choice = json_chunk.choices.into_iter().next()?;
    let tool_calls = choice
        .delta
        .tool_calls
        .into_iter()
        .map(tool_call_json_to_delta)
        .collect();

    Some(Chunk {
        delta: Delta {
            content: choice.delta.content,
            reasoning: choice.delta.reasoning_content,
            tool_calls,
        },
        finish_reason: choice.finish_reason,
        usage: json_chunk
            .usage
            .and_then(|u| Some((u.prompt_tokens? as usize, u.completion_tokens? as usize))),
    })
}

fn tool_call_json_to_delta(tc: ToolCallJson) -> ToolCallDelta {
    ToolCallDelta {
        index: tc.index,
        id: tc.id,
        name: tc.function.name,
        arguments: tc.function.arguments,
    }
}

fn process_openai_chunk(chunk: Chunk, state: &mut OpenAiState) -> Vec<ProviderEvent> {
    let mut events = Vec::new();

    // Structured tool_calls first: when present, the content filter suppresses
    // the redundant in-content tool-call markup so it is not duplicated.
    for tool_delta in chunk.delta.tool_calls {
        let evs = process_tool_call_delta(tool_delta, state);
        if evs
            .iter()
            .any(|e| matches!(e, ProviderEvent::ToolCallStart { .. }))
        {
            state.has_structured_tools = true;
        }
        events.extend(evs);
    }

    if let Some(text) = chunk.delta.content {
        let suppress = state.has_structured_tools;
        let segs = state.content_filter.feed(&text, suppress);
        for seg in segs {
            // MiniMax duplicates reasoning: inline `<think>` markup in content
            // AND the native `reasoning` field. Once the native field has been
            // seen, drop content-derived thinking segments (still consumed by
            // the filter, so the markup never reaches the visible text).
            if state.native_reasoning_seen && matches!(seg, ContentSegment::Thinking(_)) {
                continue;
            }
            events.extend(segment_to_events(seg, state));
        }
    }

    if let Some(reasoning) = chunk.delta.reasoning {
        state.native_reasoning_seen = true;
        events.extend(state.lifecycle.thinking_delta("reasoning", &reasoning));
    }

    if chunk.finish_reason.is_some() {
        events.extend(finalize_content(state));
        events.extend(flush_tool_calls(state));
        events.extend(
            state
                .lifecycle
                .finish(map_finish_reason(chunk.finish_reason.as_deref())),
        );
    }

    if let Some((input, output)) = chunk.usage {
        events.push(ProviderEvent::Usage {
            input_tokens: input,
            output_tokens: output,
        });
    }

    events
}

fn process_tool_call_delta(delta: ToolCallDelta, state: &mut OpenAiState) -> Vec<ProviderEvent> {
    let acc = state.tools.entry(delta.index).or_default();
    // Buffer id and name; only proceed when we have both
    if !update_tool_acc(acc, &delta) {
        return vec![];
    }
    // Buffer args until the tool is "started" (first args delta arrives)
    if state.started.contains(&acc.id) {
        return emit_args_delta(acc);
    }
    // Only emit ToolCallStart when the first args chunk arrives
    if acc.arguments.is_empty() {
        return vec![]; // Still waiting for args
    }
    state.started.insert(acc.id.clone());
    let id = acc.id.clone();
    let name = acc.name.clone();
    let args = std::mem::take(&mut acc.arguments);
    let mut events = vec![ProviderEvent::ToolCallStart {
        id: id.clone(),
        name,
    }];
    if !args.is_empty() {
        events.push(ProviderEvent::ToolCallInputDelta { id, delta: args });
    }
    events
}

fn update_tool_acc(acc: &mut ToolAccum, delta: &ToolCallDelta) -> bool {
    if let Some(ref id) = delta.id {
        if !id.is_empty() {
            acc.id.clone_from(id);
        }
    }
    if let Some(ref name) = delta.name {
        if !name.is_empty() {
            acc.name.clone_from(name);
        }
    }
    if let Some(ref args) = delta.arguments {
        acc.arguments.push_str(args);
    }
    !acc.id.is_empty() && !acc.name.is_empty()
}

fn emit_args_delta(acc: &mut ToolAccum) -> Vec<ProviderEvent> {
    if acc.arguments.is_empty() {
        return vec![];
    }
    let id = acc.id.clone();
    let delta_str = acc.arguments.clone();
    acc.arguments.clear();
    vec![ProviderEvent::ToolCallInputDelta {
        id,
        delta: delta_str,
    }]
}

fn flush_tool_calls(state: &mut OpenAiState) -> Vec<ProviderEvent> {
    let mut events = Vec::new();
    for acc in state.tools.values() {
        if acc.id.is_empty() || state.ended.contains(&acc.id) {
            continue;
        }
        if !state.started.contains(&acc.id) {
            state.started.insert(acc.id.clone());
            events.push(ProviderEvent::ToolCallStart {
                id: acc.id.clone(),
                name: acc.name.clone(),
            });
        }
        if !acc.arguments.is_empty() {
            events.push(ProviderEvent::ToolCallInputDelta {
                id: acc.id.clone(),
                delta: acc.arguments.clone(),
            });
        }
        state.ended.insert(acc.id.clone());
        events.push(ProviderEvent::ToolCallEnd { id: acc.id.clone() });
    }
    events
}

fn map_finish_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("stop") => StopReason::Stop,
        Some("length") => StopReason::Length,
        Some("content_filter") => StopReason::ContentFilter,
        Some("tool_calls") => StopReason::ToolCalls,
        Some("stop_sequence") => StopReason::StopSequence,
        _ => StopReason::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::provider_event::ModelError;

    fn chunk_with_content(text: &str) -> OpenAiFrame {
        let json: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"choices":[{{"delta":{{"content":"{}"}}}}]}}"#,
            text
        ))
        .unwrap();
        OpenAiFrame::Chunk(parse_chunk(&json).unwrap())
    }

    fn chunk_with_reasoning(text: &str) -> OpenAiFrame {
        let json: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"choices":[{{"delta":{{"reasoning_content":"{}"}}}}]}}"#,
            text
        ))
        .unwrap();
        OpenAiFrame::Chunk(parse_chunk(&json).unwrap())
    }

    fn chunk_with_tool_start(id: &str, name: &str) -> OpenAiFrame {
        let json: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"choices":[{{"delta":{{"tool_calls":[{{"index":0,"id":"{}","function":{{"name":"{}"}}}}]}}}}]}}"#,
            id, name
        ))
        .unwrap();
        OpenAiFrame::Chunk(parse_chunk(&json).unwrap())
    }

    fn chunk_with_tool_args(args: &str) -> OpenAiFrame {
        let json = serde_json::json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {
                            "arguments": args
                        }
                    }]
                }
            }]
        });
        OpenAiFrame::Chunk(parse_chunk(&json).unwrap())
    }

    fn chunk_with_finish(reason: &str) -> OpenAiFrame {
        let json: serde_json::Value = serde_json::from_str(&format!(
            r#"{{"choices":[{{"delta":{{}},"finish_reason":"{}"}}]}}"#,
            reason
        ))
        .unwrap();
        OpenAiFrame::Chunk(parse_chunk(&json).unwrap())
    }

    fn chunk_with_done() -> OpenAiFrame {
        OpenAiFrame::Done
    }

    #[test]
    fn gemini_tool_call_chunk_without_index_parses_and_completes() {
        // Gemini's OpenAI-compatible stream sends the whole tool call in one
        // delta, omits the `index` field, adds `extra_content.google.
        // thought_signature`, and ends tool turns with finish_reason "stop".
        let protocol = OpenAiProtocol::new();
        let state = OpenAiState::default();
        let json = serde_json::json!({
            "choices": [{
                "delta": {
                    "role": "assistant",
                    "tool_calls": [{
                        "extra_content": {"google": {"thought_signature": "sig"}},
                        "function": {"arguments": "{\"city\":\"Paris\"}", "name": "get_weather"},
                        "id": "call_1",
                        "type": "function"
                    }]
                },
                "index": 0
            }]
        });
        let frame =
            OpenAiFrame::Chunk(parse_chunk(&json).expect("gemini tool-call chunk must parse"));
        let (state, events) = protocol.step(state, frame);
        assert!(
            events.iter().any(
                |e| matches!(e, ProviderEvent::ToolCallStart { name, .. } if name == "get_weather")
            ),
            "ToolCallStart missing: {events:?}"
        );
        // finish_reason "stop" must still flush the pending tool call.
        let (_state, events) = protocol.step(state, chunk_with_finish("stop"));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ProviderEvent::ToolCallEnd { .. })),
            "ToolCallEnd missing: {events:?}"
        );
    }

    #[test]
    fn openai_protocol_step_text_delta() {
        let protocol = OpenAiProtocol::new();
        let state = OpenAiState::default();
        let frame = chunk_with_content("hi");
        let (_new_state, events) = protocol.step(state, frame);

        assert!(events
            .iter()
            .any(|e| matches!(e, ProviderEvent::TextDelta(t) if t == "hi")));
        // Verify lifecycle state was updated by checking for TextStart event
        assert!(events
            .iter()
            .any(|e| matches!(e, ProviderEvent::TextStart { id } if id == "text")));
    }

    #[test]
    fn openai_protocol_step_tool_call_accumulation() {
        let protocol = OpenAiProtocol::new();

        // First chunk: tool id and name (no args yet)
        let state = OpenAiState::default();
        let (state, events) = protocol.step(state, chunk_with_tool_start("call_1", "read_file"));
        // Tool not started yet - waiting for args
        assert!(events.is_empty());
        assert!(!state.started.contains("call_1"));

        // Second chunk: args arrive - tool starts with buffered args
        let (state, events) = protocol.step(state, chunk_with_tool_args(r#"{"path":"#));
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ToolCallStart { id, name } if id == "call_1" && name == "read_file"
        )));
        assert!(state.started.contains("call_1"));

        // Third chunk: more args - emit as delta
        let (state, events) = protocol.step(state, chunk_with_tool_args(r#"Cargo.toml"}"#));
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ToolCallInputDelta { id, delta } if id == "call_1"
        )));

        // Finish reason flushes remaining args and ends tool
        let (final_state, events) = protocol.step(state, chunk_with_finish("tool_calls"));
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ToolCallEnd { id } if id == "call_1"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::Finish {
                reason: StopReason::ToolCalls
            }
        )));
        assert!(final_state.ended.contains("call_1"));
    }

    #[test]
    fn openai_protocol_step_reasoning_delta() {
        let protocol = OpenAiProtocol::new();
        let state = OpenAiState::default();
        let frame = chunk_with_reasoning("thinking...");
        let (_new_state, events) = protocol.step(state, frame);

        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ThinkingDelta(t) if t == "thinking..."
        )));
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ThinkingStart { id } if id == "reasoning"
        )));
    }

    /// MiniMax-shaped chunk: the same reasoning text arrives twice — inline in
    /// `content` wrapped in `<think>` tags, and in the native `reasoning`
    /// field. The protocol must emit it exactly once (native field wins) while
    /// still stripping the markup from the visible text.
    fn minimax_dual_chunk(content: &str, reasoning: Option<&str>) -> OpenAiFrame {
        let mut delta = serde_json::json!({ "content": content });
        if let Some(r) = reasoning {
            delta["reasoning"] = serde_json::Value::String(r.to_owned());
        }
        let json = serde_json::json!({ "choices": [{ "delta": delta }] });
        OpenAiFrame::Chunk(parse_chunk(&json).unwrap())
    }

    #[test]
    fn minimax_dual_source_reasoning_emitted_exactly_once() {
        let protocol = OpenAiProtocol::new();
        let state = OpenAiState::default();
        let mut all = Vec::new();
        let (state, ev) = protocol.step(
            state,
            minimax_dual_chunk("<think>\nLet me think", Some("Let me think")),
        );
        all.extend(ev);
        let (state, ev) = protocol.step(
            state,
            minimax_dual_chunk(" about this.", Some(" about this.")),
        );
        all.extend(ev);
        let (state, ev) =
            protocol.step(state, minimax_dual_chunk("\n</think>\n\nThe answer", None));
        all.extend(ev);
        let (_state, ev) = protocol.step(state, chunk_with_finish("stop"));
        all.extend(ev);

        let thinking: String = all
            .iter()
            .filter_map(|e| match e {
                ProviderEvent::ThinkingDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            thinking.trim(),
            "Let me think about this.",
            "reasoning must appear exactly once, without the <think> markup"
        );
        let visible: String = all
            .iter()
            .filter_map(|e| match e {
                ProviderEvent::TextDelta(t) => Some(t.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !visible.contains("<think>") && !visible.contains("</think>"),
            "think markup must never reach the visible text: {visible:?}"
        );
        assert!(
            visible.contains("The answer"),
            "answer text must be visible: {visible:?}"
        );
    }

    #[test]
    fn openai_protocol_on_halt_flushes_pending() {
        let protocol = OpenAiProtocol::new();

        // Id and name arrive (tool not started yet)
        let state = OpenAiState::default();
        let (state, _events) = protocol.step(state, chunk_with_tool_start("call_1", "read_file"));

        // Args arrive - tool starts
        let (state, _events) = protocol.step(state, chunk_with_tool_args(r#"{"path":"README"}"#));

        // on_halt should flush remaining args and end tool
        let events = protocol.on_halt(state);
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ToolCallEnd { id } if id == "call_1"
        )));
    }

    #[test]
    fn openai_protocol_terminal_on_done() {
        let protocol = OpenAiProtocol::new();
        assert!(protocol.terminal(&chunk_with_done()));
        assert!(!protocol.terminal(&chunk_with_content("hi")));
    }

    #[test]
    fn openai_frame_from_line_text() {
        let frame = OpenAiFrame::from_line(r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#);
        match frame {
            Some(OpenAiFrame::Chunk(chunk)) => {
                assert_eq!(chunk.delta.content, Some("hi".to_string()));
            }
            other => panic!("expected Chunk, got {:?}", other),
        }
    }

    #[test]
    fn openai_frame_from_line_done() {
        let frame = OpenAiFrame::from_line("data: [DONE]");
        assert_eq!(frame, Some(OpenAiFrame::Done));
    }

    #[test]
    fn openai_frame_from_line_invalid() {
        assert!(OpenAiFrame::from_line("not sse").is_none());
    }

    #[test]
    fn openai_frame_from_line_anthropic_style_error_is_overloaded() {
        // MiniMax/Anthropic-style overload payload (HTTP 200 + SSE error frame).
        let frame = OpenAiFrame::from_line(
            r#"data: {"type":"error","error":{"type":"overloaded_error","message":"The server cluster is currently under high load. Please retry after a short wait and thank you for your patience. (2064) (529)"},"request_id":"06a4daca9074419a500d1208c1f4fa0a"}"#,
        );
        match frame {
            Some(OpenAiFrame::Error(err)) => {
                assert!(
                    matches!(err, ModelError::Overloaded { .. }),
                    "expected Overloaded, got {err:?}"
                );
                assert!(err.is_retryable());
            }
            other => panic!("expected Error frame, got {other:?}"),
        }
    }

    #[test]
    fn openai_frame_from_line_openai_style_error() {
        let frame = OpenAiFrame::from_line(
            r#"data: {"error":{"message":"Invalid API key","type":"authentication_error","code":"401"}}"#,
        );
        match frame {
            Some(OpenAiFrame::Error(err)) => {
                assert!(!err.is_retryable(), "auth errors must not be retried");
            }
            other => panic!("expected Error frame, got {other:?}"),
        }
    }

    #[test]
    fn error_frame_is_terminal() {
        let protocol = OpenAiProtocol::new();
        let frame = OpenAiFrame::Error(ModelError::Overloaded {
            retry_after_secs: None,
        });
        assert!(protocol.terminal(&frame));
    }

    #[test]
    fn tool_call_with_delayed_id() {
        let protocol = OpenAiProtocol::new();

        // First: args without id
        let state = OpenAiState::default();
        let (state, _events) = protocol.step(state, chunk_with_tool_args(r#"{"path":"file"}"#));
        assert!(state.tools[&0].arguments.contains("path"));
        assert!(state.tools[&0].id.is_empty());

        // Then: id arrives
        let (_state, events) = protocol.step(state, chunk_with_tool_start("call_2", "read_file"));
        // Should emit ToolCallStart
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ToolCallStart { id, name } if id == "call_2"
        )));
        // And buffered args as delta
        assert!(events.iter().any(|e| matches!(
            e, ProviderEvent::ToolCallInputDelta { id, delta } if id == "call_2" && delta.contains("path")
        )));
    }
}
