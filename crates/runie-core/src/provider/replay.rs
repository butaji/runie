//! Small deterministic replay provider for recorded SSE traces.

use std::{fs, path::Path, sync::Arc};

use futures::stream;
use parking_lot::Mutex;

use crate::types::{
    AssistantMessage, AssistantMessageEvent, Model, SimpleStreamOptions, StopReason, ToolCall,
    Usage,
};

use super::{
    http::HttpActor,
    stream_fn::{AssistantMessageEventStream, StreamError, StreamFn},
};

/// Replays text/reasoning events from an OpenAI-, Anthropic-, or Gemini-style
/// SSE capture. Provider-specific transport details stay in the fixture; the
/// core only receives its normal `AssistantMessageEvent` stream.
pub struct ReplayProvider {
    events: Vec<AssistantMessageEvent>,
    /// Number of `stream()` calls. The first call replays the recorded trace;
    /// later calls (auto-continue after a tool batch) return a terminating
    /// `Done{stop}` so the loop does not re-replay the same trace forever.
    calls: Mutex<usize>,
}

impl ReplayProvider {
    pub fn from_sse(path: impl AsRef<Path>) -> Result<Self, StreamError> {
        let input = fs::read_to_string(path).map_err(|e| StreamError::Invalid(e.to_string()))?;
        Self::from_sse_body(&input)
    }

    pub async fn from_http(http: Arc<dyn HttpActor>) -> Result<Self, StreamError> {
        Self::from_http_with_options(http, Model::default(), None).await
    }

    pub async fn from_http_with_options(
        http: Arc<dyn HttpActor>,
        model: Model,
        options: Option<SimpleStreamOptions>,
    ) -> Result<Self, StreamError> {
        let response = http
            .post_with_options(String::new(), model, options)
            .await?;
        if response.status >= 400 {
            return Err(StreamError::Api(format!("HTTP {}", response.status)));
        }
        Self::from_sse_body(&response.body)
    }

    fn from_sse_body(input: &str) -> Result<Self, StreamError> {
        let mut events = vec![AssistantMessageEvent::Start {
            partial: AssistantMessage::default(),
        }];
        let mut finished = false;
        let mut tool_calls = std::collections::BTreeMap::<usize, (String, String, String)>::new();
        for line in input.lines() {
            if let Some(raw_error) = line.strip_prefix("error:").map(str::trim_start) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw_error) {
                    return Err(StreamError::Api(value.to_string()));
                }
                return Err(StreamError::Api(raw_error.to_owned()));
            }
            let Some(raw) = line.strip_prefix("data:").map(str::trim_start) else {
                continue;
            };
            if raw == "[DONE]" {
                finished = true;
                break;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
                continue;
            };
            if value.get("type").and_then(|v| v.as_str()) == Some("error") {
                return Err(StreamError::Api(value.to_string()));
            }
            finished |= append_text_events(&value, &mut events);
            collect_tool_calls(&value, &mut tool_calls);
            finished |= has_terminal_marker(&value);
        }
        if !finished {
            return Err(StreamError::Invalid("trace has no terminal event".into()));
        }
        finish_replay_events(&mut events, tool_calls);
        Ok(Self {
            events,
            calls: Mutex::new(0),
        })
    }
}

fn finish_replay_events(
    events: &mut Vec<AssistantMessageEvent>,
    tool_calls: std::collections::BTreeMap<usize, (String, String, String)>,
) {
    for (_, (id, name, arguments)) in tool_calls {
        let args = serde_json::from_str(&arguments)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        events.push(AssistantMessageEvent::ToolCallDelta {
            index: 0,
            partial: ToolCall {
                id,
                name,
                arguments: args,
                thought_signature: None,
            },
        });
    }
    events.push(AssistantMessageEvent::Done {
        stop_reason: StopReason::Stop,
        usage: Usage::default(),
        message: None,
    });
}

fn append_text_events(value: &serde_json::Value, events: &mut Vec<AssistantMessageEvent>) -> bool {
    for (pointer, thinking) in [
        ("/choices/0/delta/content", false),
        ("/delta/text", false),
        ("/choices/0/delta/reasoning_content", true),
        ("/delta/thinking", true),
        ("/delta/reasoning", true),
    ] {
        if let Some(text) = value
            .pointer(pointer)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            events.push(if thinking {
                AssistantMessageEvent::ThinkingDelta {
                    index: 1,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                }
            } else {
                AssistantMessageEvent::TextDelta {
                    index: 0,
                    delta: text.into(),
                    partial: AssistantMessage::default(),
                }
            });
        }
    }
    false
}

fn collect_tool_calls(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    collect_anthropic_tool_call(value, tool_calls);
    collect_openai_tool_calls(value, tool_calls);
}

fn collect_anthropic_tool_call(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    if let Some(partial) = value
        .pointer("/delta/partial_json")
        .and_then(|v| v.as_str())
    {
        tool_calls.entry(0).or_default().2.push_str(partial);
    }
    let Some(block) = value
        .pointer("/content_block")
        .filter(|v| v.get("type").and_then(|x| x.as_str()) == Some("tool_use"))
    else {
        return;
    };
    let entry = tool_calls
        .entry(value.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
        .or_default();
    entry.0 = block
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("trace-tool")
        .into();
    entry.1 = block
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .into();
}

fn collect_openai_tool_calls(
    value: &serde_json::Value,
    tool_calls: &mut std::collections::BTreeMap<usize, (String, String, String)>,
) {
    let Some(calls) = value
        .pointer("/choices/0/delta/tool_calls")
        .and_then(|v| v.as_array())
    else {
        return;
    };
    for call in calls {
        let entry = tool_calls
            .entry(call.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
            .or_default();
        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
            entry.0 = id.into();
        }
        if let Some(name) = call.pointer("/function/name").and_then(|v| v.as_str()) {
            entry.1 = name.into();
        }
        if let Some(args) = call.pointer("/function/arguments").and_then(|v| v.as_str()) {
            entry.2.push_str(args);
        }
    }
}

fn has_terminal_marker(value: &serde_json::Value) -> bool {
    value.get("type").and_then(|v| v.as_str()) == Some("message_stop")
        || value
            .pointer("/delta/stop_reason")
            .is_some_and(|v| !v.is_null())
        || value
            .pointer("/choices/0/finish_reason")
            .is_some_and(|v| !v.is_null())
}

impl ReplayProvider {
    /// Reset the turn counter so a fresh run replays the recorded trace again.
    pub fn reset_turns(&self) {
        *self.calls.lock() = 0;
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
        let mut n = self.calls.lock();
        *n += 1;
        if *n > 1 {
            return Ok(Box::pin(stream::iter(vec![AssistantMessageEvent::Done {
                stop_reason: StopReason::Stop,
                usage: Usage::default(),
                message: None,
            }])));
        }
        Ok(Box::pin(stream::iter(self.events.clone())))
    }
}
