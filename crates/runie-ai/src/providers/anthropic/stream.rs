//! Anthropic streaming implementation.

use async_stream::stream;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Response;
use runie_core::{Event, ProviderError};

use super::types::AnthropicStreamChunk;

pub async fn build_anthropic_stream(response: Response) -> Result<BoxStream<'static, Event>, ProviderError> {
    let session_id = format!("anthropic-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));

    let stream = stream! {
        yield Event::AgentStart { session_id: session_id.clone(), timestamp: Utc::now() };
        yield Event::TurnStart { turn: 0, timestamp: Utc::now() };
        yield Event::MessageStart { role: "assistant".to_string(), timestamp: Utc::now() };

        let mut text_content = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();
        let mut current_tool_id = String::new();
        let mut in_tool_block = false;

        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        if !line.starts_with("event: ") && !line.starts_with("data: ") { continue; }

                        let (event_type, data) = if let Some(stripped) = line.strip_prefix("event: ") {
                            (stripped.trim(), None)
                        } else if let Some(stripped) = line.strip_prefix("data: ") {
                            ("", Some(stripped.trim()))
                        } else { continue; };

                        if event_type == "message_start" || event_type == "content_block_start"
                           || event_type == "content_block_delta" || event_type == "message_delta"
                           || event_type == "message_stop" || event_type == "content_block_stop" {
                            if let Some(data_str) = data {
                                let chunk: AnthropicStreamChunk = match serde_json::from_str(data_str) {
                                    Ok(c) => c,
                                    Err(_) => continue,
                                };

                                match chunk {
                                    AnthropicStreamChunk::MessageStart(_) => {},
                                    AnthropicStreamChunk::ContentBlockStart(cb) => {
                                        if cb.type_ == "tool_use" {
                                            current_tool_name = cb.name.unwrap_or_default();
                                            current_tool_args.clear();
                                            current_tool_id = format!("call_{}", cb.index);
                                            in_tool_block = true;
                                        }
                                    }
                                    AnthropicStreamChunk::ContentBlockDelta(delta) => {
                                        match delta.type_.as_str() {
                                            "text_delta" => {
                                                if let Some(text) = delta.text {
                                                    text_content.push_str(&text);
                                                    yield Event::MessageDelta { content: text };
                                                }
                                            }
                                            "input_json_delta" => {
                                                if let Some(partial) = delta.partial_json {
                                                    current_tool_args.push_str(&partial);
                                                    yield Event::ToolCallDelta { id: current_tool_id.clone(), name: current_tool_name.clone(), arguments: current_tool_args.clone() };
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    AnthropicStreamChunk::ContentBlockStop => { in_tool_block = false; }
                                    AnthropicStreamChunk::MessageDelta(delta) => {
                                        if let Some(usage) = delta.usage {
                                            yield Event::Usage { prompt_tokens: usage.input_tokens, completion_tokens: usage.output_tokens, total_tokens: usage.input_tokens + usage.output_tokens };
                                        }
                                    }
                                    AnthropicStreamChunk::MessageStop => {
                                        yield Event::MessageEnd;
                                        yield Event::AgentEnd { timestamp: Utc::now() };
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => { yield Event::Error { message: e.to_string() }; break; }
            }
        }
    };

    Ok(Box::pin(stream))
}
