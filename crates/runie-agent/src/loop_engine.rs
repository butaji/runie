use crate::config::AgentConfig;
use crate::events::*;
use crate::tools::AgentTool;
use crate::{Hook, HookDecision};
use runie_ai::Provider;
use runie_core::{Message, ToolSchema, Event as LlmEvent, Context, ToolCall as CoreToolCall};
use runie_tools::ToolRegistry;
use tokio::sync::{mpsc, Mutex};
use futures::StreamExt;
use chrono::Utc;
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum AgentLoopError {
    ProviderError(String),
    ToolError(String),
    SendError(String),
    MaxTurnsExceeded,
}

impl std::fmt::Display for AgentLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLoopError::ProviderError(s) => write!(f, "Provider error: {}", s),
            AgentLoopError::ToolError(s) => write!(f, "Tool error: {}", s),
            AgentLoopError::SendError(s) => write!(f, "Send error: {}", s),
            AgentLoopError::MaxTurnsExceeded => write!(f, "Max turns exceeded"),
        }
    }
}

pub struct AgentLoopConfig {
    pub system_prompt: String,
    pub model: String,
    pub thinking_level: String,
    pub max_turns: usize,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            model: String::new(),
            thinking_level: String::new(),
            max_turns: AgentConfig::default().max_turns,
        }
    }
}

/// Event stream for consuming agent loop events and sending permission decisions.
/// This is returned by the `agent_loop` function and provides a Stream impl
/// for consuming events, along with methods to send permission decisions
/// and retrieve the final result.
pub struct AgentEventStream {
    rx: mpsc::Receiver<AgentEvent>,
    perm_tx: mpsc::Sender<PermissionDecision>,
    result: Option<Vec<AgentMessage>>,
}

impl AgentEventStream {
    /// Send a permission decision back to the agent loop.
    pub async fn send_permission(
        &self,
        decision: PermissionDecision,
    ) -> Result<(), mpsc::error::SendError<PermissionDecision>> {
        self.perm_tx.send(decision).await
    }

    /// Consume the stream and collect the final result (messages from AgentEnd event).
    /// Drains any remaining events and returns the accumulated messages.
    pub async fn result(mut self) -> Vec<AgentMessage> {
        // Drain remaining events to find AgentEnd
        while let Ok(event) = self.rx.try_recv() {
            if let AgentEvent::AgentEnd { messages, .. } = event {
                return messages;
            }
        }
        self.result.unwrap_or_default()
    }
}

impl futures::Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

/// Send an agent event through the unified message channel.
async fn send_event<M: TryFrom<AgentEvent> + Send + 'static>(msg_tx: &mpsc::Sender<M>, event: AgentEvent) {
    if let Ok(msg) = M::try_from(event) {
        if msg_tx.send(msg).await.is_err() {
            tracing::error!("Failed to send agent event");
        }
    }
}

/// Calculate estimated context window usage as a percentage.
fn calculate_context_window_usage(messages: &[AgentMessage], context_window: usize) -> f32 {
    let total_chars: usize = messages.iter()
        .map(|m| format_message_content(&m.content).len())
        .sum();
    let estimated_tokens = total_chars / 4;
    if context_window > 0 {
        (estimated_tokens as f32 / context_window as f32) * 100.0
    } else {
        0.0
    }
}

/// Classify an error into type and recoverability.
fn classify_error(error: &AgentLoopError) -> (String, bool, String) {
    match error {
        AgentLoopError::ProviderError(msg) => (
            "provider".to_string(),
            true,
            format!("Provider error: {}", msg),
        ),
        AgentLoopError::ToolError(msg) => (
            "tool".to_string(),
            true,
            format!("Tool error: {}", msg),
        ),
        AgentLoopError::SendError(msg) => (
            "send".to_string(),
            true,
            format!("Send error: {}", msg),
        ),
        AgentLoopError::MaxTurnsExceeded => (
            "max_turns".to_string(),
            false,
            "Maximum number of turns exceeded".to_string(),
        ),
    }
}

pub async fn run_agent_loop<M: TryFrom<AgentEvent> + Send + 'static>(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    msg_tx: mpsc::Sender<M>,
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> Result<Vec<AgentMessage>, AgentLoopError> {
    let mut messages = initial_messages;
    let mut allowed_tools: HashSet<String> = HashSet::new();

    // Context window size (default 128k for most models)
    let context_window = 128_000;

    // Track token usage across all turns
    let mut total_input_tokens = 0u32;
    let mut total_output_tokens = 0u32;

    // Convert tools to schema
    let tool_schemas: Vec<ToolSchema> = tools.iter().map(|t| ToolSchema {
        name: t.name.clone(),
        description: t.description.clone(),
        parameters: t.parameters.clone(),
    }).collect();

    let mut turn_count = 0;

    loop {
        turn_count += 1;
        if turn_count > config.max_turns {
            let (error_type, recoverable, context) = classify_error(&AgentLoopError::MaxTurnsExceeded);
            send_event(&msg_tx, AgentEvent::Error {
                message: format!("Max turns ({}) exceeded", config.max_turns),
                error_type,
                recoverable,
                context,
            }).await;
            return Err(AgentLoopError::MaxTurnsExceeded);
        }

        // Build LLM messages
        let llm_messages = build_llm_messages(&config.system_prompt, &messages);

        // Start streaming
        let stream = provider.chat(llm_messages, tool_schemas.clone()).await
            .map_err(|e| AgentLoopError::ProviderError(e.to_string()))?;

        // Send message_start event with turn count
        let mut assistant_message = AgentMessage {
            role: "assistant".to_string(),
            content: vec![ContentPart::Text { text: String::new() }],
            timestamp: Utc::now().timestamp_millis(),
            usage: None,
            stop_reason: None,
            error_message: None,
        };

        send_event(&msg_tx, AgentEvent::MessageStart {
            message: assistant_message.clone(),
            turn: turn_count,
        }).await;

        // Process stream
        let mut pending_tool_calls: Vec<(ContentPart, String, String)> = vec![];
        let mut text_content = String::new();

        let mut stream = stream;
        while let Some(event) = stream.next().await {
            match event {
                LlmEvent::MessageDelta { content } => {
                    let delta = content.clone();
                    text_content.push_str(&delta);
                    assistant_message.content = vec![ContentPart::Text { text: text_content.clone() }];
                    send_event(&msg_tx, AgentEvent::MessageUpdate {
                        message: assistant_message.clone(),
                        turn: turn_count,
                        delta,
                    }).await;
                }
                LlmEvent::ToolCallDelta { name, arguments } => {
                    pending_tool_calls.push((
                        ContentPart::ToolUse {
                            id: format!("call_{}", pending_tool_calls.len()),
                            name: name.clone(),
                            input: serde_json::json!(arguments),
                        },
                        name,
                        arguments,
                    ));
                }
                LlmEvent::MessageEnd => {
                    // Finalize any pending tool calls
                    break;
                }
                LlmEvent::Error { message } => {
                    assistant_message.error_message = Some(message);
                    break;
                }
                LlmEvent::Usage { prompt_tokens, completion_tokens, total_tokens: _ } => {
                    // Accumulate token usage
                    total_input_tokens += prompt_tokens as u32;
                    total_output_tokens += completion_tokens as u32;

                    send_event(&msg_tx, AgentEvent::TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens: prompt_tokens + completion_tokens,
                        context_window,
                    }).await;
                }
                _ => {
                    tracing::warn!("Unhandled LLM event variant in agent loop");
                }
            }
        }

        // Send message_end with turn count
        assistant_message.content = vec![ContentPart::Text { text: text_content }];
        send_event(&msg_tx, AgentEvent::MessageEnd {
            message: assistant_message.clone(),
            turn: turn_count,
        }).await;

        messages.push(assistant_message.clone());

        // Execute tool calls
        if pending_tool_calls.is_empty() {
            // No tools, turn is done
            let context_window_usage = calculate_context_window_usage(&messages, context_window);
            let token_usage = TokenUsage {
                input: total_input_tokens,
                output: total_output_tokens,
                cache_read: 0,
                cache_write: 0,
                total_tokens: total_input_tokens + total_output_tokens,
            };
            let _ = context_window_usage; // Used for potential future field
            send_event(&msg_tx, AgentEvent::TurnEnd {
                turn: turn_count,
                message_count: messages.len(),
                tool_results_count: 0,
                token_usage,
            }).await;
            break;
        }

        let mut tool_results = vec![];
        // P2-7 FIX: Idempotency - track seen tool calls to prevent duplicates
        let mut seen_tool_calls: HashSet<String> = HashSet::new();
        for (tool_use, _tool_name, _args_str) in pending_tool_calls {
            if let ContentPart::ToolUse { id, name, input } = &tool_use {
                // P2-7 FIX: Check for duplicate tool call (same name + args in same turn)
                let tool_key = format!("{}:{}", name, serde_json::to_string(input).unwrap_or_default());
                if seen_tool_calls.contains(&tool_key) {
                    tracing::warn!("Duplicate tool call detected and skipped: {} with args {:?}", name, input);
                    continue;
                }
                seen_tool_calls.insert(tool_key);

                let tool_args = serde_json::to_string(input).unwrap_or_default();
                let context_window_usage = calculate_context_window_usage(&messages, context_window);

                send_event(&msg_tx, AgentEvent::ToolExecutionStart {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    tool_args: tool_args.clone(),
                    turn: turn_count,
                }).await;

                // Check if tool is in allowed cache first
                let should_execute = if allowed_tools.contains(name) {
                    send_event(&msg_tx, AgentEvent::PermissionGranted {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                    }).await;
                    true
                } else {
                    // Get tool description for permission request
                    let tool_description = tools.iter()
                        .find(|t| t.name == *name)
                        .map(|t| t.description.clone())
                        .unwrap_or_default();

                    // Send permission request
                    send_event(&msg_tx, AgentEvent::PermissionRequest {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                        tool_description,
                        turn: turn_count,
                        context_window_usage,
                    }).await;

                    // Wait for permission decision by polling shared state
                    let decision = tokio::time::timeout(
                        Duration::from_secs(300), // 5 minute timeout
                        async {
                            loop {
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                let permission = permission_state.lock().await.take();
                                if permission.is_some() {
                                    break permission;
                                }
                            }
                        }
                    ).await;

                    match decision {
                        Ok(Some(PermissionDecision::Allow { tool_call_id: ref tid, ref tool_name, ref tool_args })) if tid == id => {
                            send_event(&msg_tx, AgentEvent::PermissionGranted {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            true
                        }
                        Ok(Some(PermissionDecision::AllowAlways { tool_call_id: ref tid, ref tool_name, ref tool_args })) if tid == id => {
                            // Cache the tool name for future auto-allow
                            allowed_tools.insert(name.clone());
                            send_event(&msg_tx, AgentEvent::PermissionGranted {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            true
                        }
                        Ok(Some(PermissionDecision::Skip { tool_call_id: ref tid, ref tool_name, ref tool_args })) if tid == id => {
                            send_event(&msg_tx, AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            false // Skip this tool but continue with others
                        }
                        Ok(Some(PermissionDecision::Deny { tool_call_id: ref _tid, ref tool_name, ref tool_args })) => {
                            send_event(&msg_tx, AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                                tool_name: tool_name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            false
                        }
                        _ => {
                            // Timeout, mismatch, or deny
                            send_event(&msg_tx, AgentEvent::PermissionDenied {
                                tool_call_id: id.clone(),
                                tool_name: name.clone(),
                                tool_args: tool_args.clone(),
                            }).await;
                            false
                        }
                    }
                };

                if !should_execute {
                    // Add a fake error result
                    let result = ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
                        content: vec![ContentPart::Text { text: "Tool execution denied by user".to_string() }],
                        is_error: true,
                    };

                    let duration_ms = 0u64; // Denied tools don't execute
                    send_event(&msg_tx, AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                        result: result.clone(),
                        duration_ms,
                        turn: turn_count,
                    }).await;

                    messages.push(AgentMessage {
                        role: "tool".to_string(),
                        content: vec![ContentPart::ToolResult {
                            tool_use_id: id.clone(),
                            content: result.content.clone(),
                            is_error: result.is_error,
                        }],
                        timestamp: Utc::now().timestamp_millis(),
                        usage: None,
                        stop_reason: None,
                        error_message: None,
                    });

                    tool_results.push(result);
                    continue;
                }

                // Track execution start time for duration calculation
                let start_time = Instant::now();

                // Find and execute tool
                let tool_call = runie_core::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: input.clone(),
                };
                let ctx = Context::default();

                // Run before hooks
                let mut current_args = input.clone();
                let mut blocked = false;
                let mut block_reason = String::new();
                for hook in &hooks {
                    match hook.before_tool_call(&CoreToolCall { arguments: current_args.clone(), ..tool_call.clone() }, &ctx).await {
                        Ok(HookDecision::Allow) => {},
                        Ok(HookDecision::Block { reason }) => {
                            blocked = true;
                            block_reason = reason;
                            break;
                        }
                        Ok(HookDecision::Modify { args }) => {
                            current_args = args;
                        }
                        Err(e) => {
                            tracing::error!("Hook error: {}", e);
                            blocked = true;
                            block_reason = format!("Hook error: {}", e);
                            break;
                        }
                    }
                }

                if blocked {
                    let blocked_result = ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
                        content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", block_reason) }],
                        is_error: true,
                    };
                    let duration_ms = start_time.elapsed().as_millis() as u64;
                    send_event(&msg_tx, AgentEvent::ToolExecutionEnd {
                        tool_call_id: id.clone(),
                        tool_name: name.clone(),
                        tool_args: tool_args.clone(),
                        result: blocked_result.clone(),
                        duration_ms,
                        turn: turn_count,
                    }).await;
                    tool_results.push(blocked_result);
                    continue;
                }

                let final_input = current_args.clone();

                // P1-3 FIX: Wrap tool execution in panic catch to prevent panics from crashing the agent
                let tool_execution = execute_tool_with_panic_catch(
                    registry.clone(),
                    name,
                    final_input.clone(),
                    hooks.clone(),
                    tool_call.clone(),
                    ctx.clone(),
                ).await;

                let result = match tool_execution {
                    Ok(result) => result,
                    Err(panic_msg) => {
                        // Tool panicked - return error result and trigger rollback
                        tracing::error!("Tool '{}' panicked: {}", name, panic_msg);
                        let panic_result = ToolResult {
                            tool_call_id: id.clone(),
                            tool_name: name.clone(),
                            input: final_input,
                            content: vec![ContentPart::Text {
                                text: format!("Tool '{}' panicked (internal error). State has been rolled back.", name)
                            }],
                            is_error: true,
                        };

                        // Send panic event to notify TUI with error classification
                        send_event(&msg_tx, AgentEvent::Error {
                            message: format!("Tool '{}' panicked: {}", name, panic_msg),
                            error_type: "tool_panic".to_string(),
                            recoverable: true,
                            context: format!("Tool '{}' panicked during execution", name),
                        }).await;

                        panic_result
                    }
                };

                let duration_ms = start_time.elapsed().as_millis() as u64;
                send_event(&msg_tx, AgentEvent::ToolExecutionEnd {
                    tool_call_id: id.clone(),
                    tool_name: name.clone(),
                    tool_args: tool_args.clone(),
                    result: result.clone(),
                    duration_ms,
                    turn: turn_count,
                }).await;

                // Add tool result to messages
                messages.push(AgentMessage {
                    role: "tool".to_string(),
                    content: vec![ContentPart::ToolResult {
                        tool_use_id: id.clone(),
                        content: result.content.clone(),
                        is_error: result.is_error,
                    }],
                    timestamp: Utc::now().timestamp_millis(),
                    usage: None,
                    stop_reason: None,
                    error_message: None,
                });

                tool_results.push(result);
            }
        }

        let context_window_usage = calculate_context_window_usage(&messages, context_window);
        let token_usage = TokenUsage {
            input: total_input_tokens,
            output: total_output_tokens,
            cache_read: 0,
            cache_write: 0,
            total_tokens: total_input_tokens + total_output_tokens,
        };
        let _ = context_window_usage; // Used for potential future field
        send_event(&msg_tx, AgentEvent::TurnEnd {
            turn: turn_count,
            message_count: messages.len(),
            tool_results_count: tool_results.len(),
            token_usage,
        }).await;

        // Continue loop - send updated messages back to LLM
    }

    let final_token_usage = TokenUsage {
        input: total_input_tokens,
        output: total_output_tokens,
        cache_read: 0,
        cache_write: 0,
        total_tokens: total_input_tokens + total_output_tokens,
    };
    send_event(&msg_tx, AgentEvent::AgentEnd {
        messages: messages.clone(),
        total_turns: turn_count,
        final_token_usage,
    }).await;
    Ok(messages)
}

fn build_llm_messages(system_prompt: &str, messages: &[AgentMessage]) -> Vec<Message> {
    let mut llm_msgs = vec![Message::System { content: system_prompt.to_string() }];
    for msg in messages {
        let content = format_message_content(&msg.content);
        if let Some(m) = agent_msg_to_llm(&msg.role, content, &msg.content) {
            llm_msgs.push(m);
        }
    }
    llm_msgs
}

/// P1-3 FIX: Execute tool with panic recovery
/// Wraps tool execution in a catch_unwind to prevent panics from crashing the agent.
/// Returns Ok(result) on success, Err(panic_message) if tool panicked.
///
/// NOTE: This currently catches panics from data preparation only, not from the
/// async tool.execute() call itself. Async panics would need a different isolation
/// mechanism (e.g., a dedicated worker process/thread) to catch properly.
async fn execute_tool_with_panic_catch(
    registry: Arc<ToolRegistry>,
    name: &str,
    input: serde_json::Value,
    hooks: Vec<Arc<dyn Hook>>,
    tool_call: runie_core::ToolCall,
    ctx: Context,
) -> Result<ToolResult, String> {
    let registry_clone = registry.clone();
    let name_clone = name.to_string();
    let input_clone = input.clone();
    let hooks_clone = hooks.clone();
    let tool_call_clone = tool_call.clone();
    let ctx_clone = ctx.clone();

    // First: run data preparation in spawn_blocking with catch_unwind
    // This catches panics from any sync setup code
    let prep_result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            (name_clone, input_clone, registry_clone, hooks_clone, tool_call_clone, ctx_clone)
        }))
    }).await;

    let (name_str, input_final, registry_final, hooks_final, tool_call_final, ctx_final) = match prep_result {
        Ok(Ok(data)) => data,
        Ok(Err(panic_info)) => {
            let panic_msg = extract_panic_message(panic_info);
            return Err(panic_msg);
        }
        Err(join_err) => {
            return Err(format!("Task execution failed: {}", join_err));
        }
    };

    // Execute the tool (async - panics here cannot be caught by catch_unwind
    // since catch_unwind only works with the current stack frame)
    let output_result = if let Some(tool) = registry_final.get(&name_str) {
        tool.execute(input_final.clone()).await
    } else {
        return Ok(ToolResult {
            tool_call_id: tool_call_final.id.clone(),
            tool_name: name_str.clone(),
            input: input_final,
            content: vec![ContentPart::Text { text: format!("Tool '{}' not found", name_str) }],
            is_error: true,
        });
    };

    // Run after hooks
    let final_output = match output_result {
        Ok(mut output) => {
            for hook in &hooks_final {
                match hook.after_tool_call(&tool_call_final, &output, &ctx_final).await {
                    Ok(processed) => output = processed,
                    Err(e) => {
                        tracing::error!("After-hook error: {}", e);
                        output = runie_core::ToolOutput {
                            content: format!("After-hook error: {}", e),
                            metadata: serde_json::Value::Null,
                            terminate: true,
                        };
                    }
                }
            }
            output
        }
        Err(e) => {
            runie_core::ToolOutput {
                content: e.to_string(),
                metadata: serde_json::Value::Null,
                terminate: true,
            }
        }
    };

    Ok(ToolResult {
        tool_call_id: tool_call_final.id.clone(),
        tool_name: name_str.clone(),
        input: input_final,
        content: vec![ContentPart::Text { text: final_output.content }],
        is_error: final_output.terminate,
    })
}

/// Extract panic message from panic payload
fn extract_panic_message(panic_info: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic_info.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

fn format_message_content(parts: &[ContentPart]) -> String {
    parts.iter().map(|part| match part {
        ContentPart::Text { text } => text.clone(),
        ContentPart::ToolUse { name, input, .. } => format!("{}({})", name, input),
        ContentPart::ToolResult { content, .. } => content.iter().map(|c| match c {
            ContentPart::Text { text } => text.clone(),
            _ => String::new(),
        }).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }).collect::<Vec<_>>().join("\n")
}

/// Convenience wrapper that runs the agent loop and returns an event stream.
/// Creates a channel internally and spawns the loop as a task.
pub fn agent_loop(
    initial_messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: Arc<dyn Provider>,
    tools: Vec<AgentTool>,
    registry: Arc<ToolRegistry>,
    hooks: Vec<Arc<dyn Hook>>,
) -> AgentEventStream {
    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(128);
    let (perm_tx, perm_rx) = mpsc::channel::<PermissionDecision>(1);
    let permission_state = Arc::new(Mutex::new(None));
    let permission_state_clone = permission_state.clone();

    // Spawn task that bridges permission_rx to permission_state
    tokio::spawn(async move {
        let mut perm_rx = perm_rx;
        while let Some(decision) = perm_rx.recv().await {
            let mut state = permission_state_clone.lock().await;
            *state = Some(decision);
        }
    });

    tokio::spawn(async move {
        let _ = run_agent_loop(
            initial_messages,
            config,
            provider,
            tools,
            event_tx,
            permission_state,
            registry,
            hooks,
        ).await;
    });

    AgentEventStream {
        rx: event_rx,
        perm_tx,
        result: None,
    }
}

fn agent_msg_to_llm(role: &str, content: String, parts: &[ContentPart]) -> Option<Message> {
    match role {
        "user" => Some(Message::User { content, attachments: Vec::new() }),
        "assistant" => Some(Message::Assistant {
            content,
            tool_calls: Vec::new(),
            thinking: None,
        }),
        "tool" => {
            let tool_call_id = parts.iter().find_map(|part| {
                if let ContentPart::ToolResult { tool_use_id, .. } = part {
                    Some(tool_use_id.clone())
                } else {
                    None
                }
            }).unwrap_or_else(|| {
                tracing::warn!("Tool result missing tool_use_id, using 'unknown'");
                "unknown".to_string()
            });
            Some(Message::ToolResult { tool_call_id, content, is_error: false })
        }
        _ => None,
    }
}
