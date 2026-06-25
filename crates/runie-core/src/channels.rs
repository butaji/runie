//! Channel decoders — state-machine projections from the event stream.
//!
//! Each channel accumulates events into typed output for a specific UI panel.
//! - `TextChannel` — assistant message text for the message feed
//! - `ToolCallChannel` — tool calls for the tool sidebar
//! - `ReasoningChannel` — thinking/reasoning content for the thinking panel

use std::collections::HashMap;

use crate::event::Event;
use crate::message::ToolCall;

/// Output item emitted by a channel decoder.
#[derive(Debug, Clone)]
pub struct ChannelOutput {
    pub id: String,
    pub content: String,
}

impl ChannelOutput {
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// A channel decoder processes events and produces typed output items.
pub trait ChannelDecoder: Send {
    /// The type of output produced by this decoder.
    type Output: Send + Clone;

    /// Process a single event. Returns `Some(output)` if a new output item was produced.
    fn process(&mut self, event: &Event) -> Option<Self::Output>;

    /// Get all output items produced so far.
    fn output(&self) -> &[Self::Output];
}

/// Text channel accumulates text deltas for the message feed.
pub struct TextChannel {
    current_id: Option<String>,
    current_text: String,
    finished: Vec<ChannelOutput>,
}

impl Default for TextChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl TextChannel {
    pub fn new() -> Self {
        Self {
            current_id: None,
            current_text: String::new(),
            finished: Vec::new(),
        }
    }

    fn handle_text_start(&mut self, id: &str) {
        self.current_id = Some(id.to_owned());
        self.current_text.clear();
    }

    fn handle_response_delta(&mut self, id: &str, content: &str) {
        if self.current_id.as_deref() == Some(id) {
            self.current_text.push_str(content);
        }
    }

    fn emit_current(&mut self) -> Option<ChannelOutput> {
        if let Some(id) = self.current_id.take() {
            if !self.current_text.is_empty() {
                let output = ChannelOutput {
                    id,
                    content: std::mem::take(&mut self.current_text),
                };
                self.finished.push(output.clone());
                return Some(output);
            }
        }
        None
    }
}

impl ChannelDecoder for TextChannel {
    type Output = ChannelOutput;

    fn process(&mut self, event: &Event) -> Option<Self::Output> {
        match event {
            Event::TextStart { id } => {
                self.handle_text_start(id);
                None
            }
            Event::ResponseDelta { id, content } => {
                self.handle_response_delta(id, content);
                None
            }
            Event::TextEnd { id } => {
                if self.current_id.as_ref() == Some(id) {
                    self.emit_current()
                } else {
                    None
                }
            }
            Event::Response { id, content } => {
                let output = ChannelOutput {
                    id: id.clone(),
                    content: content.clone(),
                };
                self.finished.push(output.clone());
                Some(output)
            }
            Event::TurnComplete { .. } => self.emit_current(),
            _ => None,
        }
    }

    fn output(&self) -> &[Self::Output] {
        &self.finished
    }
}

/// Tool call state tracking for active tool invocations.
#[derive(Debug, Clone)]
pub enum ToolCallState {
    Input(Vec<String>),
    Waiting,
    Completed(String),
}

/// Tool call channel tracks active and completed tool calls for the tool sidebar.
pub struct ToolCallChannel {
    active: HashMap<String, ToolCallState>,
    completed: Vec<ToolCall>,
}

impl Default for ToolCallChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallChannel {
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
            completed: Vec::new(),
        }
    }
}

impl ChannelDecoder for ToolCallChannel {
    type Output = ToolCall;

    fn process(&mut self, event: &Event) -> Option<Self::Output> {
        match event {
            Event::ToolStart { id, input, .. } => {
                self.active.insert(
                    id.clone(),
                    ToolCallState::Input(vec![serde_json::to_string(input).unwrap_or_default()]),
                );
                None
            }
            Event::ToolEnd { id, .. } => {
                if let Some(state) = self.active.remove(id) {
                    match state {
                        ToolCallState::Input(inputs) => {
                            let args_str = inputs.join("");
                            let args: serde_json::Value =
                                serde_json::from_str(&args_str).unwrap_or_default();
                            let tool_call = ToolCall::new(id.clone(), String::new(), args);
                            self.completed.push(tool_call.clone());
                            return Some(tool_call);
                        }
                        ToolCallState::Waiting => {
                            let tool_call =
                                ToolCall::new(id.clone(), String::new(), serde_json::json!({}));
                            self.completed.push(tool_call.clone());
                            return Some(tool_call);
                        }
                        ToolCallState::Completed(_) => {}
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn output(&self) -> &[Self::Output] {
        &self.completed
    }
}

/// Reasoning channel accumulates thinking content for the thinking panel.
pub struct ReasoningChannel {
    current_id: Option<String>,
    current_text: String,
    finished: Vec<ChannelOutput>,
}

impl Default for ReasoningChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl ReasoningChannel {
    pub fn new() -> Self {
        Self {
            current_id: None,
            current_text: String::new(),
            finished: Vec::new(),
        }
    }
}

impl ChannelDecoder for ReasoningChannel {
    type Output = ChannelOutput;

    fn process(&mut self, event: &Event) -> Option<Self::Output> {
        match event {
            Event::ThinkingStart { id } => {
                self.current_id = Some(id.clone());
                self.current_text.clear();
                None
            }
            Event::ThinkingDelta { id, content } => {
                if self.current_id.as_ref() == Some(id) {
                    self.current_text.push_str(content);
                }
                None
            }
            Event::ThoughtDone { id } | Event::ThinkingEnd { id } => {
                if self.current_id.as_ref() == Some(id) {
                    let output = ChannelOutput {
                        id: id.clone(),
                        content: std::mem::take(&mut self.current_text),
                    };
                    self.finished.push(output.clone());
                    self.current_id = None;
                    return Some(output);
                }
                None
            }
            _ => None,
        }
    }

    fn output(&self) -> &[Self::Output] {
        &self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::EventBus;

    #[test]
    fn text_channel_accumulates_deltas() {
        let mut channel = TextChannel::new();

        channel.process(&Event::TextStart { id: "msg1".into() });
        channel.process(&Event::ResponseDelta {
            id: "msg1".into(),
            content: "Hello ".into(),
        });
        channel.process(&Event::ResponseDelta {
            id: "msg1".into(),
            content: "world!".into(),
        });

        assert_eq!(
            channel.output().len(),
            0,
            "text should not emit until TextEnd"
        );

        let output = channel.process(&Event::TextEnd { id: "msg1".into() });
        assert!(output.is_some(), "TextEnd should produce output");

        let output = output.unwrap();
        assert_eq!(output.content(), "Hello world!");
    }

    #[test]
    fn text_channel_flushes_on_turn_complete() {
        let mut channel = TextChannel::new();

        channel.process(&Event::TextStart { id: "msg1".into() });
        channel.process(&Event::ResponseDelta {
            id: "msg1".into(),
            content: "Incomplete".into(),
        });

        let output = channel.process(&Event::TurnComplete {
            id: "turn1".into(),
            duration_secs: 1.0,
        });
        assert!(
            output.is_some(),
            "TurnComplete should flush incomplete text"
        );
        assert_eq!(output.unwrap().content(), "Incomplete");
    }

    #[test]
    fn tool_call_channel_tracks_active_and_completed() {
        let mut channel = ToolCallChannel::new();

        channel.process(&Event::ToolStart {
            id: "call1".into(),
            name: "read_file".into(),
            input: serde_json::json!({"path": "test.txt"}),
        });

        let output = channel.process(&Event::ToolEnd {
            id: "call1".into(),
            duration_secs: 0.5,
            output: "file contents".into(),
        });

        assert!(output.is_some(), "ToolEnd should produce output");
        assert_eq!(
            channel.output().len(),
            1,
            "completed tool should be in output"
        );
    }

    #[test]
    fn reasoning_channel_collects_thoughts() {
        let mut channel = ReasoningChannel::new();

        channel.process(&Event::ThinkingStart {
            id: "think1".into(),
        });
        channel.process(&Event::ThinkingDelta {
            id: "think1".into(),
            content: "Let me think... ".into(),
        });
        channel.process(&Event::ThinkingDelta {
            id: "think1".into(),
            content: "Done!".into(),
        });

        let output = channel.process(&Event::ThoughtDone {
            id: "think1".into(),
        });
        assert!(output.is_some(), "ThoughtDone should produce output");
        assert_eq!(output.unwrap().content(), "Let me think... Done!");
    }

    #[test]
    fn channel_ignores_irrelevant_events() {
        let mut text_channel = TextChannel::new();

        text_channel.process(&Event::Input('h'));
        text_channel.process(&Event::Submit);
        text_channel.process(&Event::Quit);

        assert_eq!(
            text_channel.output().len(),
            0,
            "unrelated events should be ignored"
        );
    }

    #[test]
    fn event_bus_subscribe_channel_filters_events() {
        let bus = EventBus::<Event>::new(10);
        let (tx, _rx) = std::sync::mpsc::channel();

        bus.subscribe_channel(TextChannel::new(), tx);

        bus.publish(Event::TextStart { id: "msg1".into() });
        bus.publish(Event::ResponseDelta {
            id: "msg1".into(),
            content: "Hello".into(),
        });
        bus.publish(Event::TextEnd { id: "msg1".into() });
        bus.publish(Event::Input('x')); // Should be ignored by text channel
    }
}
