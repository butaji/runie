//! OpenAI streaming protocol implementation.
//!
//! Transforms OpenAI SSE frames (`OpenAiFrame`) into `ProviderEvent`s.

use super::types::{ChunkJson, ToolCallJson};
use crate::protocol::ProviderProtocol;
use runie_core::provider_event::{ProviderEvent, StopReason};
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
                let mut events = flush_tool_calls(&mut new_state);
                events.extend(new_state.lifecycle.finish(StopReason::Stop));
                (new_state, events)
            }
        }
    }

    fn on_halt(&self, state: Self::State) -> Vec<ProviderEvent> {
        let mut new_state = state;
        flush_tool_calls(&mut new_state)
    }

    fn terminal(&self, frame: &Self::Frame) -> bool {
        matches!(frame, OpenAiFrame::Done)
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

    if let Some(text) = chunk.delta.content {
        events.extend(state.lifecycle.text_delta("text", &text));
    }
    if let Some(reasoning) = chunk.delta.reasoning {
        events.extend(state.lifecycle.thinking_delta("reasoning", &reasoning));
    }

    for tool_delta in chunk.delta.tool_calls {
        events.extend(process_tool_call_delta(tool_delta, state));
    }

    if chunk.finish_reason.is_some() {
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
