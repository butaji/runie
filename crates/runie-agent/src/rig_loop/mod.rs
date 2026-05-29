//! Rig-based agent loop using rig-core's streaming API.
//!
//! This module provides an alternative to `loop_engine.rs` that uses rig's
//! streaming completion API with macro-based dispatch to avoid generic type issues.

use crate::events::{AgentEvent, AgentMessage, ContentPart, ToolResult};
use crate::permission::PermissionGate;
use crate::{Hook, HookDecision};
use futures::StreamExt;
use runie_ai::RigProvider;
use runie_core::{Message, ToolCall as CoreToolCall, Context, ToolSchema};
use runie_tools::ToolRegistry;
use rig_core::client::CompletionClient;
use rig_core::completion::CompletionModel;
use rig_core::streaming::StreamedAssistantContent;
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod stream;
pub mod tool;

// Re-export for convenience
pub(crate) use stream::process_stream;
pub(crate) use tool::{execute_tool_calls, agent_message_to_core};

#[derive(Debug, Clone)]
pub enum AgentLoopError {
    ProviderError(String),
    ToolError(String),
    SendError(String),
    MaxTurnsExceeded,
    RigError(String),
}

impl std::fmt::Display for AgentLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLoopError::ProviderError(s) => write!(f, "Provider error: {}", s),
            AgentLoopError::ToolError(s) => write!(f, "Tool error: {}", s),
            AgentLoopError::SendError(s) => write!(f, "Send error: {}", s),
            AgentLoopError::MaxTurnsExceeded => write!(f, "Max turns exceeded"),
            AgentLoopError::RigError(s) => write!(f, "Rig error: {}", s),
        }
    }
}

impl std::error::Error for AgentLoopError {}

/// Converts our Message type to rig's message format.
pub(crate) fn convert_message_to_rig(msg: &Message) -> rig_core::completion::Message {
    use rig_core::completion::message::{AssistantContent, ToolCall, ToolFunction, UserContent};
    use rig_core::OneOrMany;

    match msg {
        Message::System { content } => rig_core::completion::Message::System {
            content: content.clone(),
        },
        Message::User { content, attachments: _ } => rig_core::completion::Message::User {
            content: OneOrMany::one(UserContent::Text(rig_core::completion::message::Text {
                text: content.clone(),
            })),
        },
        Message::Assistant { content, tool_calls, thinking: _ } => {
            let mut contents = Vec::new();
            if !content.is_empty() {
                contents.push(AssistantContent::Text(rig_core::completion::message::Text {
                    text: content.clone(),
                }));
            }
            for tc in tool_calls {
                contents.push(AssistantContent::ToolCall(ToolCall {
                    id: tc.id.clone(),
                    call_id: None,
                    function: ToolFunction {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    },
                    signature: None,
                    additional_params: None,
                }));
            }
            if contents.is_empty() {
                contents.push(AssistantContent::Text(rig_core::completion::message::Text {
                    text: String::new(),
                }));
            }
            let content = OneOrMany::many(contents).unwrap_or_else(|_|
                OneOrMany::one(AssistantContent::Text(rig_core::completion::message::Text {
                    text: String::new(),
                }))
            );
            rig_core::completion::Message::Assistant { id: None, content }
        }
        Message::ToolResult { tool_call_id, content, is_error: _ } => {
            rig_core::completion::Message::User {
                content: OneOrMany::one(UserContent::ToolResult(rig_core::completion::message::ToolResult {
                    id: tool_call_id.clone(),
                    call_id: None,
                    content: OneOrMany::one(rig_core::completion::message::ToolResultContent::Text(
                        rig_core::completion::message::Text { text: content.clone() }
                    )),
                }))
            }
        }
    }
}

/// Converts our tool schemas to rig tool definitions.
pub(crate) fn convert_tools_to_rig(tools: &[ToolSchema]) -> Vec<rig_core::completion::request::ToolDefinition> {
    tools.iter().map(|t| {
        rig_core::completion::request::ToolDefinition {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.parameters.clone(),
        }
    }).collect()
}

/// Check if a tool call should be executed based on permission gate.
pub(crate) async fn should_request_permission(
    tool_name: &str,
    tool_id: &str,
    permission_gate: &mut PermissionGate,
    event_tx: &mpsc::Sender<AgentEvent>,
) -> Result<bool, AgentLoopError> {
    use crate::permission::PermissionResult;
    match permission_gate.request_permission(tool_name, tool_id).await {
        PermissionResult::Allowed => {
            event_tx.send(AgentEvent::PermissionGranted { tool_call_id: tool_id.to_string() }).await
                .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
            Ok(true)
        }
        PermissionResult::Skipped | PermissionResult::Denied => {
            event_tx.send(AgentEvent::PermissionDenied { tool_call_id: tool_id.to_string() }).await
                .map_err(|e| AgentLoopError::SendError(e.to_string()))?;
            Ok(false)
        }
    }
}

/// Runs the agent loop using rig's streaming API with macro-based dispatch.
pub async fn run_rig_agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: crate::loop_engine::AgentLoopConfig,
    provider: &RigProvider,
    _tool_schemas: Vec<ToolSchema>,
    event_tx: mpsc::Sender<AgentEvent>,
    permission_gate: PermissionGate,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<(), AgentLoopError> {
    // Convert initial messages to rig format
    let system_prompt = config.system_prompt.clone();
    let max_turns = config.max_turns;

    // Build rig messages from initial messages
    let mut rig_messages: Vec<rig_core::completion::Message> = vec![
        rig_core::completion::Message::System { content: system_prompt }
    ];

    for msg in &initial_messages {
        if let Some(core_msg) = tool::agent_message_to_core(msg) {
            rig_messages.push(convert_message_to_rig(&core_msg));
        }
    }

    let rig_tools = convert_tools_to_rig(&registry.schemas());

    // Use macro to dispatch to concrete provider type and stream
    runie_ai::with_rig_provider!(provider, client, model_name, {
        // Get the model
        let model = client.completion_model(model_name);

        // Build completion request
        if rig_messages.is_empty() {
            return Err(AgentLoopError::ProviderError("No messages provided".to_string()));
        }

        let prompt = rig_messages.last().cloned().unwrap();
        let chat_history = rig_messages.into_iter().rev().skip(1).rev().collect::<Vec<_>>();

        let mut builder = model.completion_request(prompt);
        if !chat_history.is_empty() {
            builder = builder.messages(chat_history);
        }
        if !rig_tools.is_empty() {
            builder = builder.tools(rig_tools);
        }

        let request = builder.build();

        // Stream completion
        let stream = model.stream(request)
            .await
            .map_err(|e| AgentLoopError::RigError(e.to_string()))?;

        // Process stream
        stream::process_stream(
            stream,
            event_tx,
            permission_gate,
            registry,
            hooks,
            max_turns,
        ).await
    })
}
