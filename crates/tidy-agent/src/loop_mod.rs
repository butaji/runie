use tidy_core::{Message, Event, ToolCall};
use tidy_ai::Provider;
use tidy_tools::ToolRegistry;
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
        let session_id = self.state.session.id.clone();
        
        events.push(Event::AgentStart { 
            session_id: session_id.clone(), 
            timestamp: Utc::now() 
        });

        // Add user message
        let user_msg_id = self.state.add_message(None, Message::User { 
            content: request, 
            attachments: Vec::new() 
        });

        for turn in 0..self.config.max_turns {
            self.state.turn_count = turn;
            events.push(Event::TurnStart { turn, timestamp: Utc::now() });

            // Build messages from session
            let messages = self.build_messages();
            let tools = self.executor.schemas();

            // Call LLM
            let mut stream = self.provider.chat(messages, tools).await
                .map_err(|e| AgentLoopError::ProviderError(e.to_string()))?;

            // Collect response
            let mut assistant_content = String::new();
            let mut tool_calls = Vec::new();
            let mut current_tool_call: Option<ToolCall> = None;
            
            while let Some(event) = stream.next().await {
                match &event {
                    Event::MessageDelta { content } => assistant_content.push_str(content),
                    Event::ToolCallDelta { name, arguments } => {
                        // Handle partial tool call accumulation
                        if let Some(ref mut tc) = current_tool_call {
                            if tc.name.is_empty() && !name.is_empty() {
                                tc.name = name.clone();
                            }
                            // Accumulate arguments as JSON string
                            let current_args = tc.arguments.clone();
                            if current_args.is_null() {
                                tc.arguments = serde_json::json!(arguments);
                            } else if let Some(current_str) = current_args.as_str() {
                                tc.arguments = serde_json::json!(format!("{}{}", current_str, arguments));
                            }
                        } else if !name.is_empty() {
                            current_tool_call = Some(ToolCall {
                                id: uuid::Uuid::new_v4().to_string(),
                                name: name.clone(),
                                arguments: serde_json::json!(arguments),
                            });
                        }
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

            // If no tool calls, we're done
            if tool_calls.is_empty() {
                let _msg_id = self.state.add_message(Some(user_msg_id.clone()), Message::Assistant {
                    content: assistant_content,
                    tool_calls: Vec::new(),
                    thinking: None,
                });
                break;
            }

            // Execute tool calls
            let results = self.executor.execute(tool_calls.clone()).await;
            
            // Add assistant message with tool calls
            let assistant_msg_id = self.state.add_message(Some(user_msg_id.clone()), Message::Assistant {
                content: assistant_content,
                tool_calls: tool_calls.clone(),
                thinking: None,
            });

            // Add tool results
            for (tool_call_id, result) in results {
                match result {
                    Ok(output) => {
                        events.push(Event::ToolExecutionEnd {
                            tool_call_id: tool_call_id.clone(),
                            result: output.clone(),
                            timestamp: Utc::now(),
                        });
                        self.state.add_message(Some(assistant_msg_id.clone()), Message::ToolResult {
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
                        self.state.add_message(Some(assistant_msg_id.clone()), Message::ToolResult {
                            tool_call_id,
                            content: error,
                            is_error: true,
                        });
                    }
                }
            }

            // Check steering queue
            if let Some(steering) = self.state.steering_queue.pop_front() {
                self.state.add_message(Some(assistant_msg_id.clone()), Message::User {
                    content: steering,
                    attachments: Vec::new(),
                });
            }
        }

        events.push(Event::AgentEnd { timestamp: Utc::now() });
        Ok(events)
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
