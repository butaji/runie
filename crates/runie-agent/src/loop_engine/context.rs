use crate::events::*;
use crate::AgentMessage;
use chrono::Utc;
use runie_ai::Provider;
use runie_core::{Message, ToolCall};
use std::collections::HashMap;
use std::sync::Arc;

/// Log message content for debugging.
pub(crate) fn log_message_content(messages: &[AgentMessage]) {
    for (i, msg) in messages.iter().enumerate() {
        let tool_use_count = msg.content.iter().filter(|p| matches!(p, ContentPart::ToolUse { .. })).count();
        if msg.role == "assistant" && tool_use_count > 0 {
            tracing::info!("[ACTOR:AgentLoop] Message {}: Assistant with {} tool calls", i, tool_use_count);
            for part in &msg.content {
                if let ContentPart::ToolUse { id, name, .. } = part {
                    tracing::info!("[ACTOR:AgentLoop]   Tool call: id={} name={}", id, name);
                }
            }
        } else if msg.role == "tool" {
            let tool_id = msg.content.iter().find_map(|p| {
                if let ContentPart::ToolResult { tool_use_id, .. } = p {
                    Some(tool_use_id.clone())
                } else {
                    None
                }
            }).unwrap_or_default();
            tracing::info!("[ACTOR:AgentLoop] Message {}: ToolResult for id={}", i, tool_id);
        }
    }
}

/// Maximum number of messages to keep in context after compaction
pub(crate) const MAX_CONTEXT_MESSAGES: usize = 50;

/// Compact context when message count exceeds this threshold
pub(crate) const COMPACT_THRESHOLD: usize = 40;

/// Number of recent messages to preserve when compacting (not summarized)
pub(crate) const RECENT_MESSAGES_TO_KEEP: usize = 10;

/// Compact context by summarizing old messages when conversation grows too long.
/// Preserves system message + recent messages + summary of old messages.
pub(crate) async fn compact_context(
    history: &mut Vec<AgentMessage>,
    provider: Arc<dyn Provider>,
) -> Result<(usize, String), String> {
    if history.len() <= MAX_CONTEXT_MESSAGES {
        return Ok((history.len(), String::new()));
    }

    let original_count = history.len();

    // Extract system message if present (first message with role "system")
    let system_msg = history.first().filter(|m| m.role == "system").cloned();

    // Get recent messages to preserve (last N messages)
    let recent_msgs: Vec<AgentMessage> = history.iter()
        .rev()
        .take(RECENT_MESSAGES_TO_KEEP)
        .rev()
        .cloned()
        .collect();

    // Get middle messages to summarize (everything between system and recent)
    let middle_start = if system_msg.is_some() { 1 } else { 0 };
    let middle_end = history.len().saturating_sub(RECENT_MESSAGES_TO_KEEP);
    let middle_msgs: Vec<AgentMessage> = if middle_end > middle_start {
        history[middle_start..middle_end].to_vec()
    } else {
        Vec::new()
    };

    // Summarize middle section if present
    let summary = if !middle_msgs.is_empty() {
        summarize_messages(&middle_msgs, provider).await?
    } else {
        String::new()
    };

    // Rebuild history: system + summary + recent
    let mut new_history = Vec::new();

    if let Some(sys) = system_msg {
        new_history.push(sys);
    }

    // Add summary as a system message
    if !summary.is_empty() {
        new_history.push(AgentMessage {
            role: "system".to_string(),
            content: vec![ContentPart::Text {
                text: format!("Previous conversation summary:\n{}", summary),
            }],
            timestamp: Utc::now().timestamp_millis(),
            usage: None,
            stop_reason: None,
            error_message: None,
            tool_calls: vec![],
        });
    }

    // Add recent messages
    new_history.extend(recent_msgs);

    *history = new_history;

    let compacted_count = history.len();
    let summary_preview = if summary.len() > 100 {
        format!("{}...", &summary[..100])
    } else {
        summary.clone()
    };

    tracing::info!(
        "[COMPACT] Context compacted: {} messages -> {} messages",
        original_count, compacted_count
    );

    Ok((compacted_count, summary_preview))
}

/// Summarize a list of messages using the provider's chat_simple method.
pub(crate) async fn summarize_messages(
    messages: &[AgentMessage],
    provider: Arc<dyn Provider>,
) -> Result<String, String> {
    if messages.is_empty() {
        return Ok(String::new());
    }

    let content = messages.iter()
        .map(|m| {
            let role = &m.role;
            let text = format_message_content(&m.content, &m.tool_calls);
            format!("{}: {}", role, text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let summary_prompt = format!(
        "Summarize the following conversation concisely, preserving key facts, decisions, and important context:\n\n{}",
        content
    );

    let summary_message = Message::User {
        content: summary_prompt,
        attachments: Vec::new(),
    };

    match provider.chat_simple(vec![summary_message]).await {
        Ok(summary) => {
            tracing::debug!("[COMPACT] Generated summary ({} chars)", summary.len());
            Ok(summary)
        }
        Err(e) => {
            tracing::warn!("[COMPACT] Failed to generate summary: {}", e);
            Err(format!("Failed to summarize: {}", e))
        }
    }
}

pub(crate) fn build_llm_messages(system_prompt: &str, messages: &[AgentMessage]) -> Vec<Message> {
    let mut llm_msgs = vec![Message::System { content: system_prompt.to_string() }];
    for msg in messages {
        let content = format_message_content(&msg.content, &msg.tool_calls);
        if let Some(m) = agent_msg_to_llm(&msg.role, content, &msg.content, &msg.tool_calls) {
            llm_msgs.push(m);
        }
    }
    llm_msgs
}

pub(crate) fn format_message_content(parts: &[ContentPart], tool_calls: &[ToolCall]) -> String {
    // Build a map from (name, arguments) to id for looking up tool call IDs
    let tc_map: HashMap<(String, String), String> = tool_calls.iter().map(|tc| {
        let args_str = tc.arguments.to_string();
        ((tc.name.clone(), args_str), tc.id.clone())
    }).collect();

    parts.iter().map(|part| match part {
        ContentPart::Text { text } => text.clone(),
        ContentPart::ToolUse { id, name, input } => {
            let args_str = input.to_string();
            let tc_id = tc_map.get(&(name.clone(), args_str)).cloned().unwrap_or_else(|| id.clone());
            format!("[TC:{}] {}({})", tc_id, name, input)
        }
        ContentPart::ToolResult { content, .. } => content.iter().map(|c| match c {
            ContentPart::Text { text } => text.clone(),
            _ => String::new(),
        }).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }).collect::<Vec<_>>().join("\n")
}

pub(crate) fn agent_msg_to_llm(role: &str, content: String, parts: &[ContentPart], tool_calls: &[ToolCall]) -> Option<Message> {
    match role {
        "user" => Some(Message::User { content, attachments: Vec::new() }),
        "assistant" => Some(Message::Assistant {
            content,
            tool_calls: tool_calls.to_vec(),
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
                tracing::error!("Tool result missing tool_use_id - this indicates a bug in message construction");
                "unknown".to_string()
            });

            // Validate: warn if tool_call_id looks fake (generated by buggy code)
            if tool_call_id == "unknown" || tool_call_id.starts_with("call_") && tool_call_id.chars().count() <= 7 {
                tracing::error!(
                    "INVALID TOOL_CALL_ID '{}' - this will cause 400 Bad Request from LLM API. \
                    Tool result must reference a valid tool_call.id from the assistant message.",
                    tool_call_id
                );
            }

            Some(Message::ToolResult { tool_call_id, content, is_error: false })
        }
        _ => None,
    }
}
