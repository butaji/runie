//! Small deterministic replay provider for recorded SSE traces.

use std::{fs, path::Path};

use futures::stream;

use crate::types::{AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, ToolCall, Usage};

use super::stream_fn::{AssistantMessageEventStream, StreamError, StreamFn};

/// Replays text/reasoning events from an OpenAI-, Anthropic-, or Gemini-style
/// SSE capture. Provider-specific transport details stay in the fixture; the
/// core only receives its normal `AssistantMessageEvent` stream.
pub struct ReplayProvider {
    events: Vec<AssistantMessageEvent>,
}

impl ReplayProvider {
    pub fn from_sse(path: impl AsRef<Path>) -> Result<Self, StreamError> {
        let input = fs::read_to_string(path).map_err(|e| StreamError::Invalid(e.to_string()))?;
        let mut events = vec![AssistantMessageEvent::Start];
        let mut finished = false;
        let mut tool_calls = std::collections::BTreeMap::<usize, (String, String, String)>::new();
        for line in input.lines() {
            let Some(raw) = line.strip_prefix("data:").map(str::trim_start) else { continue };
            if raw == "[DONE]" { finished = true; break; }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else { continue };
            if value.get("type").and_then(|v| v.as_str()) == Some("error") {
                return Err(StreamError::Api(value.to_string()));
            }
            if let Some(text) = value.pointer("/choices/0/delta/content").and_then(|v| v.as_str()) {
                if !text.is_empty() { events.push(AssistantMessageEvent::TextDelta { delta: text.into() }); }
            }
            if let Some(text) = value.pointer("/choices/0/delta/reasoning_content").and_then(|v| v.as_str()) {
                if !text.is_empty() { events.push(AssistantMessageEvent::ThinkingDelta { delta: text.into() }); }
            }
            if let Some(text) = value.pointer("/delta/text").and_then(|v| v.as_str()) {
                if !text.is_empty() { events.push(AssistantMessageEvent::TextDelta { delta: text.into() }); }
            }
            if let Some(partial) = value.pointer("/delta/partial_json").and_then(|v| v.as_str()) {
                let entry = tool_calls.entry(0).or_insert((String::new(), String::new(), String::new()));
                entry.2.push_str(partial);
            }
            if let Some(block) = value.pointer("/content_block").filter(|v| v.get("type").and_then(|x| x.as_str()) == Some("tool_use")) {
                let entry = tool_calls.entry(value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
                    .or_insert((String::new(), String::new(), String::new()));
                entry.0 = block.get("id").and_then(|v| v.as_str()).unwrap_or("trace-tool").into();
                entry.1 = block.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").into();
            }
            if let Some(calls) = value.pointer("/choices/0/delta/tool_calls").and_then(|v| v.as_array()) {
                for call in calls {
                    let index = call.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let entry = tool_calls.entry(index).or_insert((String::new(), String::new(), String::new()));
                    if let Some(id) = call.get("id").and_then(|v| v.as_str()) { entry.0 = id.into(); }
                    if let Some(name) = call.pointer("/function/name").and_then(|v| v.as_str()) { entry.1 = name.into(); }
                    if let Some(args) = call.pointer("/function/arguments").and_then(|v| v.as_str()) { entry.2.push_str(args); }
                }
            }
            if value.get("type").and_then(|v| v.as_str()) == Some("message_stop") {
                finished = true;
            }
            if value.pointer("/delta/stop_reason").is_some_and(|v| !v.is_null()) {
                finished = true;
            }
            if value.pointer("/choices/0/finish_reason").is_some_and(|v| !v.is_null()) {
                finished = true;
            }
        }
        if !finished { return Err(StreamError::Invalid("trace has no terminal event".into())); }
        for (_, (id, name, arguments)) in tool_calls {
            let args = serde_json::from_str(&arguments).unwrap_or(serde_json::Value::Object(Default::default()));
            events.push(AssistantMessageEvent::ToolCallDelta { index: 0, partial: ToolCall { id, name, arguments: args } });
        }
        events.push(AssistantMessageEvent::Done { stop_reason: StopReason::Stop, usage: Usage::default() });
        Ok(Self { events })
    }
}

#[async_trait::async_trait]
impl StreamFn for ReplayProvider {
    async fn stream(
        &self,
        _model: &Model,
        _context: &crate::types::AgentContext,
        _options: Option<SimpleStreamOptions>,
    ) -> Result<AssistantMessageEventStream, StreamError> {
        Ok(Box::pin(stream::iter(self.events.clone())))
    }
}
