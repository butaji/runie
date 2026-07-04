//! Minimal Anthropic Messages API streaming parser for replay fixtures.
//!
//! This is intentionally not a full provider implementation; it only parses the
//! SSE traces recorded from Anthropic-compatible endpoints (e.g. OpenCode Go
//! `/v1/messages`) into the canonical `ProviderEvent` vocabulary so they can be
//! used in replay tests.

use runie_core::provider_event::{ProviderEvent, StopReason};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct MessageStart {
    #[serde(rename = "type")]
    _type: String,
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    model: String,
    #[serde(rename = "stop_reason")]
    #[allow(dead_code)]
    stop_reason: Option<String>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockStart {
    #[serde(rename = "type")]
    _type: String,
    index: usize,
    content_block: ContentBlock,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    id: Option<String>,
    name: Option<String>,
    #[allow(dead_code)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockDelta {
    #[serde(rename = "type")]
    _type: String,
    index: usize,
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: Option<String>,
    text: Option<String>,
    thinking: Option<String>,
    #[serde(rename = "partial_json")]
    partial_json: Option<String>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockStop {
    #[serde(rename = "type")]
    _type: String,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct MessageDelta {
    #[serde(rename = "type")]
    _type: String,
    delta: Delta,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(rename = "input_tokens")]
    input_tokens: usize,
    #[serde(rename = "output_tokens")]
    output_tokens: usize,
}

/// Replay Anthropic SSE text and return the accumulated `ProviderEvent`s.
pub fn replay_anthropic_sse(text: &str) -> Vec<ProviderEvent> {
    let mut events = Vec::new();
    let mut blocks: HashMap<usize, (String, String)> = HashMap::new();
    let mut finish_reason: Option<StopReason> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with("data:") {
            continue;
        }
        let payload = trimmed["data:".len()..].trim_start();
        if payload.is_empty() {
            continue;
        }

        let event_type = match serde_json::from_str::<serde_json::Value>(payload) {
            Ok(val) => val.get("type").and_then(|t| t.as_str()).map(String::from),
            Err(_) => continue,
        };

        match event_type.as_deref() {
            Some("message_start") => {
                if let Ok(msg) = serde_json::from_str::<MessageStart>(payload) {
                    // Capture usage if provided at stream start.
                    if let Some(u) = msg.message.usage {
                        events.push(ProviderEvent::Usage {
                            input_tokens: u.input_tokens,
                            output_tokens: u.output_tokens,
                        });
                    }
                }
            }
            Some("content_block_start") => {
                if let Ok(block) = serde_json::from_str::<ContentBlockStart>(payload) {
                    let id = block
                        .content_block
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("block_{}", block.index));
                    let block_type = block.content_block.block_type.clone();
                    blocks.insert(block.index, (id.clone(), block_type.clone()));
                    match block_type.as_str() {
                        "text" => events.push(ProviderEvent::TextStart { id }),
                        "thinking" => events.push(ProviderEvent::ThinkingStart { id }),
                        "tool_use" => {
                            let name = block.content_block.name.unwrap_or_default();
                            events.push(ProviderEvent::ToolCallStart { id, name });
                        }
                        _ => {}
                    }
                }
            }
            Some("content_block_delta") => {
                if let Ok(delta) = serde_json::from_str::<ContentBlockDelta>(payload) {
                    let (id, _) = blocks
                        .get(&delta.index)
                        .cloned()
                        .unwrap_or_else(|| (format!("block_{}", delta.index), String::new()));
                    if let Some(text) = delta.delta.text {
                        events.push(ProviderEvent::TextDelta(text));
                    } else if let Some(thinking) = delta.delta.thinking {
                        events.push(ProviderEvent::ThinkingDelta(thinking));
                    } else if let Some(partial) = delta.delta.partial_json {
                        events.push(ProviderEvent::ToolCallInputDelta { id, delta: partial });
                    }
                }
            }
            Some("content_block_stop") => {
                if let Ok(stop) = serde_json::from_str::<ContentBlockStop>(payload) {
                    if let Some((id, block_type)) = blocks.get(&stop.index).cloned() {
                        match block_type.as_str() {
                            "text" => events.push(ProviderEvent::TextEnd { id }),
                            "thinking" => events.push(ProviderEvent::ThinkingEnd { id }),
                            "tool_use" => events.push(ProviderEvent::ToolCallEnd { id }),
                            _ => {}
                        }
                    }
                }
            }
            Some("message_delta") => {
                if let Ok(msg_delta) = serde_json::from_str::<MessageDelta>(payload) {
                    if let Some(reason) = msg_delta.delta.stop_reason {
                        finish_reason = match reason.as_str() {
                            "end_turn" => Some(StopReason::Stop),
                            "tool_use" => Some(StopReason::ToolCalls),
                            "max_tokens" => Some(StopReason::Length),
                            _ => Some(StopReason::Unknown),
                        };
                    }
                    if let Some(u) = msg_delta.usage {
                        events.push(ProviderEvent::Usage {
                            input_tokens: u.input_tokens,
                            output_tokens: u.output_tokens,
                        });
                    }
                }
            }
            Some("message_stop") => {
                events.push(ProviderEvent::Finish {
                    reason: finish_reason.unwrap_or(StopReason::Stop),
                });
            }
            _ => {}
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_testing::anthropic_fixture;

    #[test]
    fn anthropic_simple_replays() {
        let events = replay_anthropic_sse(&anthropic_fixture(
            "opencode_go_minimax_m3_simple.sse",
        ));
        assert!(events.iter().any(|e| matches!(e, ProviderEvent::TextDelta(_))));
        assert!(events.iter().any(|e| matches!(e, ProviderEvent::Finish { .. })));
    }

    #[test]
    fn anthropic_tool_replays() {
        let events = replay_anthropic_sse(&anthropic_fixture(
            "opencode_go_minimax_m3_tool.sse",
        ));
        assert!(events.iter().any(|e| matches!(
            e,
            ProviderEvent::ToolCallStart { name, .. } if name == "get_weather"
        )));
    }
}
