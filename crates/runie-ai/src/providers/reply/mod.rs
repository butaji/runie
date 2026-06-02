//! ReplyProvider - loads recorded MiniMax API responses for testing.
//! Maps content/reasoning/tool_calls to Event types. Routes by input keyword.

use async_stream::stream;
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::BoxStream;
use runie_core::{Event, Message, ProviderError, ToolSchema};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::Provider;

/// Recorded MiniMax response types for replay
#[derive(Debug, Deserialize)]
pub struct RecordedResponse {
    pub id: Option<String>,
    #[serde(default)]
    pub choices: Option<Vec<RecordedChoice>>,
    pub usage: Option<RecordedUsage>,
    #[serde(default)]
    pub base_resp: Option<RecordedBaseResp>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedBaseResp {
    pub status_code: i32,
    pub status_msg: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordedChoice {
    pub finish_reason: Option<String>,
    pub index: usize,
    #[serde(default)]
    pub delta: Option<RecordedDelta>,
    pub message: Option<RecordedMessage>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedDelta {
    pub content: Option<String>,
    pub role: Option<String>,
    #[serde(rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<RecordedToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<RecordedToolCall>>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedToolCall {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<RecordedFunction>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedFunction {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedUsage {
    #[serde(rename = "total_tokens")]
    pub total_tokens: usize,
    #[serde(rename = "prompt_tokens", default)]
    pub prompt_tokens: Option<usize>,
    #[serde(rename = "completion_tokens", default)]
    pub completion_tokens: Option<usize>,
    #[serde(rename = "completion_tokens_details", default)]
    pub completion_tokens_details: Option<RecordedCompletionDetails>,
}

#[derive(Debug, Deserialize)]
pub struct RecordedCompletionDetails {
    #[serde(rename = "reasoning_tokens")]
    pub reasoning_tokens: Option<usize>,
}

/// Routing keywords for selecting recorded response
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scenario {
    Simple,
    Tool,
    Stream,
    StreamTool,
    Error,
    Context,
    LongReasoning,
}

impl Scenario {
    /// Determine scenario from user input message.
    pub fn from_input(input: &str) -> Self {
        let lower = input.to_lowercase();
        if lower.contains("calculate") || lower.contains("tool") {
            Scenario::Tool
        } else if lower.contains("stream") || lower.contains("count") {
            Scenario::Stream
        } else if lower.contains("bash") || lower.contains("ls") || lower.contains("list") {
            Scenario::StreamTool
        } else if lower.contains("error") || lower.contains("fail") {
            Scenario::Error
        } else if lower.contains("context") || lower.contains("memory") {
            Scenario::Context
        } else if lower.contains("long") || lower.contains("peanut") || lower.contains("explain") {
            Scenario::LongReasoning
        } else {
            Scenario::Simple
        }
    }
}

/// ReplyProvider loads and replays recorded MiniMax responses.
pub struct ReplyProvider {
    model: String,
    simple_response: RecordedResponse,
    tool_response: RecordedResponse,
    stream_chunks: Vec<String>,
    stream_tool_chunks: Vec<String>,
    error_response: RecordedResponse,
    context_chunks: Vec<String>,
    long_reasoning_chunks: Vec<String>,
}

impl ReplyProvider {
    /// Create a new ReplyProvider with fixtures from the given directory.
    pub fn new(fixtures_dir: PathBuf) -> Result<Self, ProviderError> {
        // Load simple response
        let simple_path = fixtures_dir.join("minimax_simple.json");
        let simple_content = fs::read_to_string(&simple_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_simple.json: {}", e)))?;
        let simple_response: RecordedResponse = serde_json::from_str(&simple_content)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse minimax_simple.json: {}", e)))?;

        // Load tool response
        let tool_path = fixtures_dir.join("minimax_tool.json");
        let tool_content = fs::read_to_string(&tool_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_tool.json: {}", e)))?;
        let tool_response: RecordedResponse = serde_json::from_str(&tool_content)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse minimax_tool.json: {}", e)))?;

        // Load streaming response (SSE format)
        let stream_path = fixtures_dir.join("minimax_stream.json");
        let stream_content = fs::read_to_string(&stream_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_stream.json: {}", e)))?;
        let stream_chunks: Vec<String> = stream_content
            .lines()
            .filter(|l| l.starts_with("data: "))
            .map(|l| l[6..].to_string())
            .collect();

        // Load streaming tool response (SSE format)
        let stream_tool_path = fixtures_dir.join("minimax_stream_tool.json");
        let stream_tool_content = fs::read_to_string(&stream_tool_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_stream_tool.json: {}", e)))?;
        let stream_tool_chunks: Vec<String> = stream_tool_content
            .lines()
            .filter(|l| l.starts_with("data: "))
            .map(|l| l[6..].to_string())
            .collect();

        // Load error response
        let error_path = fixtures_dir.join("minimax_error.json");
        let error_content = fs::read_to_string(&error_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_error.json: {}", e)))?;
        let error_response: RecordedResponse = serde_json::from_str(&error_content)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to parse minimax_error.json: {}", e)))?;

        // Load context response (SSE format)
        let context_path = fixtures_dir.join("minimax_context.json");
        let context_content = fs::read_to_string(&context_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_context.json: {}", e)))?;
        let context_chunks: Vec<String> = context_content
            .lines()
            .filter(|l| l.starts_with("data: "))
            .map(|l| l[6..].to_string())
            .collect();

        // Load long reasoning response (SSE format)
        let long_path = fixtures_dir.join("minimax_long_reasoning.json");
        let long_content = fs::read_to_string(&long_path)
            .map_err(|e| ProviderError::InvalidResponse(format!("Failed to read minimax_long_reasoning.json: {}", e)))?;
        let long_reasoning_chunks: Vec<String> = long_content
            .lines()
            .filter(|l| l.starts_with("data: "))
            .map(|l| l[6..].to_string())
            .collect();

        Ok(Self {
            model: "MiniMax-M2.7-highspeed".to_string(),
            simple_response,
            tool_response,
            stream_chunks,
            stream_tool_chunks,
            error_response,
            context_chunks,
            long_reasoning_chunks,
        })
    }

    /// Create ReplyProvider from standard fixtures directory.
    pub fn with_default_fixtures() -> Result<Self, ProviderError> {
        let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("providers")
            .join("reply")
            .join("fixtures");
        Self::new(fixtures_dir)
    }

    fn generate_simple_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: format!("reply-{}", self.simple_response.id.as_deref().unwrap_or("unknown")),
                timestamp: Utc::now(),
            },
            Event::TurnStart {
                turn: 0,
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
        ];

        // Extract reasoning/thinking content
        if let Some(choices) = &self.simple_response.choices {
            if let Some(choice) = choices.first() {
                if let Some(msg) = &choice.message {
                    if let Some(reasoning) = &msg.reasoning_content {
                        events.push(Event::ThinkingDelta {
                            content: reasoning.clone(),
                        });
                    }
                    if let Some(content) = &msg.content {
                        events.push(Event::MessageDelta {
                            content: content.clone(),
                        });
                    }
                }
            }
        }

        // Add usage
        if let Some(usage) = &self.simple_response.usage {
            events.push(Event::Usage {
                prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                completion_tokens: usage.completion_tokens.unwrap_or(0),
                total_tokens: usage.total_tokens,
            });
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn generate_tool_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: format!("reply-{}", self.tool_response.id.as_deref().unwrap_or("unknown")),
                timestamp: Utc::now(),
            },
            Event::TurnStart {
                turn: 0,
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
        ];

        // Extract reasoning/thinking content
        if let Some(choices) = &self.tool_response.choices {
            if let Some(choice) = choices.first() {
                if let Some(msg) = &choice.message {
                    if let Some(reasoning) = &msg.reasoning_content {
                        events.push(Event::ThinkingDelta {
                            content: reasoning.clone(),
                        });
                    }
                    // Extract tool calls
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            let id = tc.id.clone().unwrap_or_default();
                            let name = tc.function.as_ref()
                                .and_then(|f| f.name.clone())
                                .unwrap_or_default();
                            let args = tc.function.as_ref()
                                .and_then(|f| f.arguments.clone())
                                .unwrap_or_default();
                            events.push(Event::ToolCallDelta { id, name, arguments: args });
                        }
                    }
                }
            }
        }

        // Add usage
        if let Some(usage) = &self.tool_response.usage {
            events.push(Event::Usage {
                prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                completion_tokens: usage.completion_tokens.unwrap_or(0),
                total_tokens: usage.total_tokens,
            });
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn generate_stream_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: "reply-stream".to_string(),
                timestamp: Utc::now(),
            },
            Event::TurnStart {
                turn: 0,
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
        ];

        // Track accumulated content for usage calculation
        let mut total_content = String::new();
        let mut total_reasoning = String::new();

        for chunk_json in &self.stream_chunks {
            // Try to parse as a RecordedResponse (non-final chunks)
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(choices) = &chunk.choices {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = &choice.delta {
                            if let Some(reasoning) = &delta.reasoning_content {
                                total_reasoning.push_str(reasoning);
                                events.push(Event::ThinkingDelta {
                                    content: reasoning.clone(),
                                });
                            }
                            if let Some(content) = &delta.content {
                                total_content.push_str(content);
                                events.push(Event::MessageDelta {
                                    content: content.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Add usage from the final chunk if available
        for chunk_json in &self.stream_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(usage) = &chunk.usage {
                    if usage.total_tokens > 0 {
                        events.push(Event::Usage {
                            prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                            completion_tokens: usage.completion_tokens.unwrap_or(0),
                            total_tokens: usage.total_tokens,
                        });
                        break;
                    }
                }
            }
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn generate_stream_tool_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: "reply-stream-tool".to_string(),
                timestamp: Utc::now(),
            },
            Event::TurnStart {
                turn: 0,
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
        ];

        for chunk_json in &self.stream_tool_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(choices) = &chunk.choices {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = &choice.delta {
                            if let Some(reasoning) = &delta.reasoning_content {
                                events.push(Event::ThinkingDelta {
                                    content: reasoning.clone(),
                                });
                            }
                            if let Some(content) = &delta.content {
                                events.push(Event::MessageDelta {
                                    content: content.clone(),
                                });
                            }
                            // Handle tool calls in streaming mode
                            if let Some(tool_calls) = &delta.tool_calls {
                                for tc in tool_calls {
                                    let id = tc.id.clone().unwrap_or_default();
                                    let name = tc.function.as_ref()
                                        .and_then(|f| f.name.clone())
                                        .unwrap_or_default();
                                    let args = tc.function.as_ref()
                                        .and_then(|f| f.arguments.clone())
                                        .unwrap_or_default();
                                    events.push(Event::ToolCallDelta { id, name, arguments: args });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add usage from the final chunk
        for chunk_json in &self.stream_tool_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(usage) = &chunk.usage {
                    if usage.total_tokens > 0 {
                        events.push(Event::Usage {
                            prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                            completion_tokens: usage.completion_tokens.unwrap_or(0),
                            total_tokens: usage.total_tokens,
                        });
                        break;
                    }
                }
            }
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn generate_error_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: "reply-error".to_string(),
                timestamp: Utc::now(),
            },
        ];

        // Error responses have no choices - we only use base_resp for error info
        let error_msg = format!(
            "MiniMax API error: status_code={}, status_msg={}",
            self.error_response.base_resp.as_ref()
                .map(|b| b.status_code)
                .unwrap_or(0),
            self.error_response.base_resp.as_ref()
                .map(|b| b.status_msg.clone())
                .unwrap_or_default()
        );

        events.push(Event::Error {
            message: error_msg,
        });

        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn generate_context_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: "reply-context".to_string(),
                timestamp: Utc::now(),
            },
            Event::TurnStart {
                turn: 0,
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
        ];

        for chunk_json in &self.context_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(choices) = &chunk.choices {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = &choice.delta {
                            if let Some(reasoning) = &delta.reasoning_content {
                                events.push(Event::ThinkingDelta {
                                    content: reasoning.clone(),
                                });
                            }
                            if let Some(content) = &delta.content {
                                events.push(Event::MessageDelta {
                                    content: content.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Add usage from the final chunk
        for chunk_json in &self.context_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(usage) = &chunk.usage {
                    if usage.total_tokens > 0 {
                        events.push(Event::Usage {
                            prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                            completion_tokens: usage.completion_tokens.unwrap_or(0),
                            total_tokens: usage.total_tokens,
                        });
                        break;
                    }
                }
            }
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn generate_long_reasoning_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: "reply-long-reasoning".to_string(),
                timestamp: Utc::now(),
            },
            Event::TurnStart {
                turn: 0,
                timestamp: Utc::now(),
            },
            Event::MessageStart {
                role: "assistant".to_string(),
                timestamp: Utc::now(),
            },
        ];

        for chunk_json in &self.long_reasoning_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(choices) = &chunk.choices {
                    if let Some(choice) = choices.first() {
                        if let Some(delta) = &choice.delta {
                            if let Some(reasoning) = &delta.reasoning_content {
                                events.push(Event::ThinkingDelta {
                                    content: reasoning.clone(),
                                });
                            }
                            if let Some(content) = &delta.content {
                                events.push(Event::MessageDelta {
                                    content: content.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Add usage from the final chunk
        for chunk_json in &self.long_reasoning_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                if let Some(usage) = &chunk.usage {
                    if usage.total_tokens > 0 {
                        events.push(Event::Usage {
                            prompt_tokens: usage.prompt_tokens.unwrap_or(0),
                            completion_tokens: usage.completion_tokens.unwrap_or(0),
                            total_tokens: usage.total_tokens,
                        });
                        break;
                    }
                }
            }
        }

        events.push(Event::MessageEnd);
        events.push(Event::AgentEnd {
            timestamp: Utc::now(),
        });

        events
    }
    fn select_scenario(&self, messages: &[Message]) -> Scenario {
        // Find the last user message
        for msg in messages.iter().rev() {
            if let Message::User { content, .. } = msg {
                return Scenario::from_input(content);
            }
        }
        Scenario::Simple
    }
}

#[async_trait]
impl Provider for ReplyProvider {
    fn name(&self) -> &str {
        "reply"
    }
    fn model(&self) -> &str {
        &self.model
    }
    fn supports_tools(&self) -> bool {
        true
    }
    fn supports_vision(&self) -> bool {
        false
    }
    fn max_context_tokens(&self) -> usize {
        128_000
    }
    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
    ) -> Result<BoxStream<'static, Event>, ProviderError> {
        let scenario = self.select_scenario(&messages);
        let events = match scenario {
            Scenario::Simple => self.generate_simple_events(),
            Scenario::Tool => self.generate_tool_events(),
            Scenario::Stream => self.generate_stream_events(),
            Scenario::StreamTool => self.generate_stream_tool_events(),
            Scenario::Error => self.generate_error_events(),
            Scenario::Context => self.generate_context_events(),
            Scenario::LongReasoning => self.generate_long_reasoning_events(),
        };

        let s = stream! {
            for event in events {
                yield event;
            }
        };

        Ok(Box::pin(s))
    }

    async fn chat_simple(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, ProviderError> {
        let scenario = self.select_scenario(&messages);
        let events = match scenario {
            Scenario::Simple => self.generate_simple_events(),
            Scenario::Tool => self.generate_tool_events(),
            Scenario::Stream => self.generate_stream_events(),
            Scenario::StreamTool => self.generate_stream_tool_events(),
            Scenario::Error => self.generate_error_events(),
            Scenario::Context => self.generate_context_events(),
            Scenario::LongReasoning => self.generate_long_reasoning_events(),
        };

        let mut content = String::new();
        for event in events {
            if let Event::MessageDelta { content: c } = event {
                content.push_str(&c);
            }
        }

        Ok(content)
    }
}

mod tests;
