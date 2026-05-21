use runie_core::{Message, Event, ToolCall};
use runie_ai::Provider;
use runie_tools::ToolRegistry;
use crate::{AgentState, AgentConfig, ToolExecutor, Hook};
use std::sync::Arc;
use chrono::Utc;
use futures::StreamExt;

pub struct AgentLoop {
    pub provider: Arc<dyn Provider>,
    pub executor: ToolExecutor,
    pub state: AgentState,
    pub config: AgentConfig,
}

impl AgentLoop {
    pub fn new(
        provider: Arc<dyn Provider>,
        registry: Arc<ToolRegistry>,
        hooks: Vec<Arc<dyn Hook>>,
        state: AgentState,
        config: AgentConfig,
    ) -> Self {
        let executor = ToolExecutor::new(registry, hooks, config.tool_execution_mode.clone());
        Self { provider, executor, state, config }
    }

    pub async fn run(&mut self, request: String) -> Result<Vec<Event>, AgentLoopError> {
        let mut events = Vec::new();
        let user_msg_id = self.init_run(request, &mut events);

        for turn in 0..self.config.max_turns {
            let done = self.run_turn(turn, &user_msg_id, &mut events).await?;
            if done { break; }
        }

        events.push(Event::AgentEnd { timestamp: Utc::now() });
        Ok(events)
    }

    fn init_run(&mut self, request: String, events: &mut Vec<Event>) -> String {
        let session_id = self.state.session.id.clone();
        events.push(Event::AgentStart { session_id, timestamp: Utc::now() });
        self.state.add_message(None, Message::User { content: request, attachments: Vec::new() })
    }

    async fn run_turn(&mut self, turn: usize, user_msg_id: &str, events: &mut Vec<Event>) -> Result<bool, AgentLoopError> {
        self.state.turn_count = turn;
        events.push(Event::TurnStart { turn, timestamp: Utc::now() });

        let messages = self.build_messages();
        let tools = self.executor.schemas();
        let mut stream = self.provider.chat(messages, tools).await
            .map_err(|e| AgentLoopError::ProviderError(e.to_string()))?;

        let (assistant_content, tool_calls) = self.collect_stream(&mut stream, events).await?;

        if tool_calls.is_empty() {
            self.state.add_message(Some(user_msg_id.to_string()), Message::Assistant {
                content: assistant_content,
                tool_calls: Vec::new(),
                thinking: None,
            });
            return Ok(true);
        }

        self.execute_tools(tool_calls, user_msg_id, assistant_content, events).await;
        Ok(false)
    }

    async fn collect_stream<S>(&mut self, stream: &mut S, events: &mut Vec<Event>) -> Result<(String, Vec<ToolCall>), AgentLoopError>
    where
        S: futures::Stream<Item = Event> + Unpin,
    {
        let mut assistant_content = String::new();
        let mut tool_calls = Vec::new();
        let mut current_tool_call: Option<ToolCall> = None;

        while let Some(event) = stream.next().await {
            match &event {
                Event::MessageDelta { content } => assistant_content.push_str(content),
                Event::ToolCallDelta { name, arguments } => {
                    Self::accumulate_tool_call(&mut current_tool_call, name, arguments);
                }
                Event::MessageEnd => {
                    if let Some(tc) = current_tool_call.take() {
                        tool_calls.push(tc);
                    }
                }
                Event::Error { message } => {
                    events.push(event.clone());
                    return Err(AgentLoopError::ProviderError(message.clone()));
                }
                _ => {}
            }
            events.push(event);
        }

        Ok((assistant_content, tool_calls))
    }

    fn accumulate_tool_call(current: &mut Option<ToolCall>, name: &str, arguments: &str) {
        if let Some(ref mut tc) = current {
            if tc.name.is_empty() && !name.is_empty() {
                tc.name = name.to_string();
            }
            let current_args = tc.arguments.clone();
            if current_args.is_null() {
                tc.arguments = serde_json::json!(arguments);
            } else if let Some(current_str) = current_args.as_str() {
                tc.arguments = serde_json::json!(format!("{}{}", current_str, arguments));
            }
        } else if !name.is_empty() {
            *current = Some(ToolCall {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                arguments: serde_json::json!(arguments),
            });
        }
    }

    async fn execute_tools(&mut self, tool_calls: Vec<ToolCall>, user_msg_id: &str, assistant_content: String, events: &mut Vec<Event>) {
        let results = self.executor.execute(tool_calls.clone()).await;
        let assistant_msg_id = self.state.add_message(Some(user_msg_id.to_string()), Message::Assistant {
            content: assistant_content,
            tool_calls: tool_calls.clone(),
            thinking: None,
        });
        self.add_tool_results(results, &assistant_msg_id, events);

        if let Some(steering) = self.state.steering_queue.pop_front() {
            self.state.add_message(Some(assistant_msg_id), Message::User {
                content: steering,
                attachments: Vec::new(),
            });
        }
    }

    fn add_tool_results(&mut self, results: Vec<(String, Result<runie_core::ToolOutput, String>)>, assistant_msg_id: &str, events: &mut Vec<Event>) {
        for (tool_call_id, result) in results {
            match result {
                Ok(output) => {
                    events.push(Event::ToolExecutionEnd {
                        tool_call_id: tool_call_id.clone(),
                        result: output.clone(),
                        timestamp: Utc::now(),
                    });
                    self.state.add_message(Some(assistant_msg_id.to_string()), Message::ToolResult {
                        tool_call_id,
                        content: output.content,
                        is_error: false,
                    });
                }
                Err(error) => {
                    events.push(Event::ToolExecutionError {
                        tool_call_id: tool_call_id.clone(),
                        error: error.clone(),
                    });
                    self.state.add_message(Some(assistant_msg_id.to_string()), Message::ToolResult {
                        tool_call_id,
                        content: error,
                        is_error: true,
                    });
                }
            }
        }
    }

    fn build_messages(&self) -> Vec<Message> {
        // Walk the message tree and build chronological list
        // For now, return messages directly from session
        self.state.session.messages.iter().map(|n| n.message.clone()).collect()
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum AgentLoopError {
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("tool execution error: {0}")]
    ToolExecutionError(String),
    #[error("max turns exceeded")]
    MaxTurnsExceeded,
}
