use crate::events::*;
use crate::AgentMessage;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio::sync::mpsc;

/// Request permission for a tool call, waiting for user decision.
/// Returns true if the tool should execute, false otherwise.
pub(crate) async fn request_permission<M: TryFrom<AgentEvent> + Send + 'static>(
    tool_call_id: &str,
    tool_name: &str,
    tool_args: &str,
    tool_description: String,
    context_window_usage: f32,
    turn: usize,
    permission_state: Arc<Mutex<Option<PermissionDecision>>>,
    msg_tx: &mpsc::Sender<M>,
) -> bool {
    // Send permission request
    send_permission_request(
        msg_tx,
        tool_call_id,
        tool_name,
        tool_args,
        tool_description,
        turn,
        context_window_usage,
    ).await;

    // Wait for permission decision by polling shared state
    let decision = timeout(
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

    handle_permission_decision(decision, tool_call_id, tool_name, tool_args, msg_tx).await
}

/// Send a permission request event.
pub(crate) async fn send_permission_request<M: TryFrom<AgentEvent> + Send + 'static>(
    msg_tx: &mpsc::Sender<M>,
    tool_call_id: &str,
    tool_name: &str,
    tool_args: &str,
    tool_description: String,
    turn: usize,
    context_window_usage: f32,
) {
    crate::loop_engine::streaming::send_event(msg_tx, AgentEvent::PermissionRequest {
        tool_call_id: tool_call_id.to_string(),
        tool_name: tool_name.to_string(),
        tool_args: tool_args.to_string(),
        tool_description,
        turn,
        context_window_usage,
    }).await;
}

/// Handle the permission decision.
/// Returns true if tool should execute, false otherwise.
pub(crate) async fn handle_permission_decision<M: TryFrom<AgentEvent> + Send + 'static>(
    decision: Result<Option<PermissionDecision>, tokio::time::error::Elapsed>,
    tool_call_id: &str,
    _tool_name: &str,
    _tool_args: &str,
    _msg_tx: &mpsc::Sender<M>,
) -> bool {
    match decision {
        Ok(Some(PermissionDecision::Allow { tool_call_id: ref tid, .. })) if tid == tool_call_id => {
            true
        }
        Ok(Some(PermissionDecision::AllowAlways { tool_call_id: ref tid, .. })) if tid == tool_call_id => {
            // Cache the tool name for future auto-allow - caller should handle this
            true
        }
        Ok(Some(PermissionDecision::Skip { tool_call_id: ref tid, .. })) if tid == tool_call_id => {
            false // Skip this tool but continue with others
        }
        Ok(Some(PermissionDecision::Deny { .. })) => {
            false
        }
        _ => {
            // Timeout, mismatch, or deny
            false
        }
    }
}

/// Add a denied tool result message to the messages list.
pub(crate) fn add_denied_result(
    messages: &mut Vec<AgentMessage>,
    tool_call_id: &str,
    _tool_name: &str,
    _input: serde_json::Value,
) {
    messages.push(AgentMessage {
        role: "tool".to_string(),
        content: vec![ContentPart::ToolResult {
            tool_use_id: tool_call_id.to_string(),
            content: vec![ContentPart::Text { text: "Tool execution denied by user".to_string() }],
            is_error: true,
        }],
        timestamp: Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    });
}

/// Add a blocked result message to the messages list.
pub(crate) fn add_blocked_result(
    messages: &mut Vec<AgentMessage>,
    tool_call_id: &str,
    _tool_name: &str,
    _input: serde_json::Value,
    block_reason: &str,
) {
    messages.push(AgentMessage {
        role: "tool".to_string(),
        content: vec![ContentPart::ToolResult {
            tool_use_id: tool_call_id.to_string(),
            content: vec![ContentPart::Text { text: format!("Blocked by safety hook: {}", block_reason) }],
            is_error: true,
        }],
        timestamp: Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    });
}

/// Add a tool result message to the messages list.
pub(crate) fn add_tool_result(
    messages: &mut Vec<AgentMessage>,
    tool_call_id: &str,
    result: &ToolResult,
) {
    messages.push(AgentMessage {
        role: "tool".to_string(),
        content: vec![ContentPart::ToolResult {
            tool_use_id: tool_call_id.to_string(),
            content: result.content.clone(),
            is_error: result.is_error,
        }],
        timestamp: Utc::now().timestamp_millis(),
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    });
}
