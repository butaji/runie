//! Event generators for ReplyProvider.

use chrono::Utc;
use runie_core::Event;

use super::{RecordedResponse, RecordedToolCall};
use super::helpers::{
    agent_start_events, agent_end_events, add_usage, extract_delta_content,
    extract_delta_tool_calls, append_usage_from_chunks, format_error_message,
};

impl super::ReplyProvider {
    /// Generate events for simple (non-streaming, non-tool) response.
    pub fn generate_simple_events(&self) -> Vec<Event> {
        let mut events = agent_start_events(
            &format!("reply-{}", self.simple_response.id.as_deref().unwrap_or("unknown"))
        );

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

        add_usage(&mut events, &self.simple_response);
        events.extend(agent_end_events());
        events
    }

    /// Generate events for tool-calling response.
    pub fn generate_tool_events(&self) -> Vec<Event> {
        let mut events = agent_start_events(
            &format!("reply-{}", self.tool_response.id.as_deref().unwrap_or("unknown"))
        );

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
                            events.push(extract_delta_tool_calls(tc));
                        }
                    }
                }
            }
        }

        add_usage(&mut events, &self.tool_response);
        events.extend(agent_end_events());
        events
    }

    /// Generate events for streaming response (no tools).
    pub fn generate_stream_events(&self) -> Vec<Event> {
        let mut events = agent_start_events("reply-stream");

        for chunk_json in &self.stream_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                extract_delta_content(&chunk, &mut events);
            }
        }

        append_usage_from_chunks(&mut events, &self.stream_chunks);
        events.extend(agent_end_events());
        events
    }

    /// Generate events for streaming response with tools.
    pub fn generate_stream_tool_events(&self) -> Vec<Event> {
        let mut events = agent_start_events("reply-stream-tool");

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
                                    events.push(extract_delta_tool_calls(tc));
                                }
                            }
                        }
                    }
                }
            }
        }

        append_usage_from_chunks(&mut events, &self.stream_tool_chunks);
        events.extend(agent_end_events());
        events
    }

    /// Generate events for error response.
    pub fn generate_error_events(&self) -> Vec<Event> {
        let mut events = vec![
            Event::AgentStart {
                session_id: "reply-error".to_string(),
                timestamp: Utc::now(),
            },
        ];

        let error_msg = format_error_message(&self.error_response);
        events.push(Event::Error { message: error_msg });
        events.extend(agent_end_events());
        events
    }

    /// Generate events for context (memory) response.
    pub fn generate_context_events(&self) -> Vec<Event> {
        let mut events = agent_start_events("reply-context");

        for chunk_json in &self.context_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                extract_delta_content(&chunk, &mut events);
            }
        }

        append_usage_from_chunks(&mut events, &self.context_chunks);
        events.extend(agent_end_events());
        events
    }

    /// Generate events for long reasoning response.
    pub fn generate_long_reasoning_events(&self) -> Vec<Event> {
        let mut events = agent_start_events("reply-long-reasoning");

        for chunk_json in &self.long_reasoning_chunks {
            if let Ok(chunk) = serde_json::from_str::<RecordedResponse>(chunk_json) {
                extract_delta_content(&chunk, &mut events);
            }
        }

        append_usage_from_chunks(&mut events, &self.long_reasoning_chunks);
        events.extend(agent_end_events());
        events
    }
}
