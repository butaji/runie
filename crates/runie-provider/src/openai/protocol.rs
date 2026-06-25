//! OpenAI streaming protocol implementation.
//!
//! Transforms OpenAI SSE frames (`OpenAiFrame`) into `LLMEvent`s.

use super::stream::{Chunk, Delta, ToolCallDelta};
use crate::protocol::ProviderProtocol;
use runie_core::llm_event::{LLMEvent, StopReason};
use runie_core::lifecycle::LifecycleState;
use std::collections::{BTreeMap, HashSet};

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
    pub tools: BTreeMap<usize, ToolAccum>,
    pub started: HashSet<String>,
    pub ended: HashSet<String>,
    pub lifecycle: LifecycleState,
}

#[derive(Debug, Default, Clone)]
pub struct ToolAccum {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl From<ToolAccum> for runie_core::message::ToolCall {
    fn from(acc: ToolAccum) -> Self {
        let args: serde_json::Value =
            serde_json::from_str(&acc.arguments).unwrap_or(serde_json::Value::Null);
        runie_core::message::ToolCall {
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

    fn step(&self, state: Self::State, frame: Self::Frame) -> (Self::State, Vec<LLMEvent>) {
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

    fn on_halt(&self, state: Self::State) -> Vec<LLMEvent> {
        let mut new_state = state;
        flush_tool_calls(&mut new_state)
    }

    fn terminal(&self, frame: &Self::Frame) -> bool {
        matches!(frame, OpenAiFrame::Done)
    }
}

fn parse_chunk(json: &serde_json::Value) -> Option<Chunk> {
    let choice = json.get("choices")?.get(0)?;
    let delta = choice.get("delta")?;

    Some(Chunk {
        delta: Delta {
            content: delta
                .get("content")
                .and_then(|v| v.as_str())
                .map(String::from),
            reasoning: extract_reasoning(delta),
            tool_calls: parse_tool_call_deltas(delta),
        },
        finish_reason: choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .map(String::from),
        usage: json.get("usage").and_then(parse_usage),
    })
}

fn parse_usage(value: &serde_json::Value) -> Option<(usize, usize)> {
    Some((
        value.get("prompt_tokens").and_then(|v| v.as_u64())? as usize,
        value.get("completion_tokens").and_then(|v| v.as_u64())? as usize,
    ))
}

fn extract_reasoning(delta: &serde_json::Value) -> Option<String> {
    delta
        .get("reasoning_content")
        .or_else(|| delta.get("reasoning"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn parse_tool_call_deltas(delta: &serde_json::Value) -> Vec<ToolCallDelta> {
    delta
        .get("tool_calls")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_tool_call_delta).collect())
        .unwrap_or_default()
}

fn parse_tool_call_delta(value: &serde_json::Value) -> Option<ToolCallDelta> {
    let index = value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let function = value.get("function").unwrap_or(value);
    Some(ToolCallDelta {
        index,
        id: value.get("id").and_then(|v| v.as_str()).map(String::from),
        name: function
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from),
        arguments: function
            .get("arguments")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

fn process_openai_chunk(chunk: Chunk, state: &mut OpenAiState) -> Vec<LLMEvent> {
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
        events.extend(state.lifecycle.finish(map_finish_reason(
            chunk.finish_reason.as_deref(),
        )));
    }

    if let Some((input, output)) = chunk.usage {
        events.push(LLMEvent::Usage {
            input_tokens: input,
            output_tokens: output,
        });
    }

    events
}

fn process_tool_call_delta(delta: ToolCallDelta, state: &mut OpenAiState) -> Vec<LLMEvent> {
    let acc = state.tools.entry(delta.index).or_default();
    // Buffer id and name; only proceed when we have both
    if !update_tool_acc(acc, &delta) { return vec![]; }
    // Buffer args until the tool is "started" (first args delta arrives)
    if state.started.contains(&acc.id) { return emit_args_delta(acc); }
    // Only emit ToolCallStart when the first args chunk arrives
    if acc.arguments.is_empty() {
        return vec![]; // Still waiting for args
    }
    state.started.insert(acc.id.clone());
    let id = acc.id.clone();
    let name = acc.name.clone();
    let args = std::mem::take(&mut acc.arguments);
    let mut events = vec![LLMEvent::ToolCallStart { id: id.clone(), name: name.clone() }];
    if !args.is_empty() {
        events.push(LLMEvent::ToolCallInputDelta { id, delta: args });
    }
    events
}

fn update_tool_acc(acc: &mut ToolAccum, delta: &ToolCallDelta) -> bool {
    if let Some(ref id) = delta.id { if !id.is_empty() { acc.id.clone_from(id); } }
    if let Some(ref name) = delta.name { if !name.is_empty() { acc.name.clone_from(name); } }
    if let Some(ref args) = delta.arguments { acc.arguments.push_str(args); }
    !acc.id.is_empty() && !acc.name.is_empty()
}

fn emit_args_delta(acc: &mut ToolAccum) -> Vec<LLMEvent> {
    if acc.arguments.is_empty() { return vec![]; }
    let id = acc.id.clone();
    let delta_str = acc.arguments.clone();
    acc.arguments.clear();
    vec![LLMEvent::ToolCallInputDelta { id, delta: delta_str }]
}

fn flush_tool_calls(state: &mut OpenAiState) -> Vec<LLMEvent> {
    let mut events = Vec::new();
    for acc in state.tools.values() {
        if acc.id.is_empty() || state.ended.contains(&acc.id) {
            continue;
        }
        if !state.started.contains(&acc.id) {
            state.started.insert(acc.id.clone());
            events.push(LLMEvent::ToolCallStart {
                id: acc.id.clone(),
                name: acc.name.clone(),
            });
        }
        if !acc.arguments.is_empty() {
            events.push(LLMEvent::ToolCallInputDelta {
                id: acc.id.clone(),
                delta: acc.arguments.clone(),
            });
        }
        state.ended.insert(acc.id.clone());
        events.push(LLMEvent::ToolCallEnd { id: acc.id.clone() });
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
        let (new_state, events) = protocol.step(state, frame);

        assert!(events.iter().any(|e| matches!(e, LLMEvent::TextDelta(t) if t == "hi")));
        // Verify lifecycle state was updated by checking for TextStart event
        assert!(events.iter().any(|e| matches!(e, LLMEvent::TextStart { id } if id == "text")));
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
            e, LLMEvent::ToolCallStart { id, name } if id == "call_1" && name == "read_file"
        )));
        assert!(state.started.contains("call_1"));

        // Third chunk: more args - emit as delta
        let (state, events) = protocol.step(state, chunk_with_tool_args(r#"Cargo.toml"}"#));
        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::ToolCallInputDelta { id, delta } if id == "call_1"
        )));

        // Finish reason flushes remaining args and ends tool
        let (final_state, events) = protocol.step(state, chunk_with_finish("tool_calls"));
        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::ToolCallEnd { id } if id == "call_1"
        )));
        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::Finish { reason: StopReason::ToolCalls }
        )));
        assert!(final_state.ended.contains("call_1"));
    }

    #[test]
    fn openai_protocol_step_reasoning_delta() {
        let protocol = OpenAiProtocol::new();
        let state = OpenAiState::default();
        let frame = chunk_with_reasoning("thinking...");
        let (new_state, events) = protocol.step(state, frame);

        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::ThinkingDelta(t) if t == "thinking..."
        )));
        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::ThinkingStart { id } if id == "reasoning"
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
            e, LLMEvent::ToolCallEnd { id } if id == "call_1"
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
        let (state, events) = protocol.step(state, chunk_with_tool_start("call_2", "read_file"));
        // Should emit ToolCallStart
        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::ToolCallStart { id, name } if id == "call_2"
        )));
        // And buffered args as delta
        assert!(events.iter().any(|e| matches!(
            e, LLMEvent::ToolCallInputDelta { id, delta } if id == "call_2" && delta.contains("path")
        )));
    }
}
