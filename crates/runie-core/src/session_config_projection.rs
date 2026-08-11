use super::super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionLaneQuery {
    pub lane: Option<String>,
    pub record_type: Option<String>,
    pub run_id: Option<String>,
    pub operation_kind: Option<String>,
    pub after_seq: Option<u64>,
    pub newest_first: bool,
    pub limit: Option<usize>,
}

/// Pi's automatic-compaction threshold settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u64,
    pub keep_recent_tokens: u64,
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reserve_tokens: 20_000,
            keep_recent_tokens: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompactionDecision {
    Disabled,
    WithinBudget {
        available_tokens: u64,
    },
    Required {
        context_tokens: u64,
        threshold_tokens: u64,
    },
}

/// Typed action selected after measuring a context. The action is data only:
/// session and provider actors still own preparation, summarization, and
/// publication respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompactionRecoveryAction {
    Continue,
    Prepare { keep_recent_tokens: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompactionRecoveryPlan {
    pub decision: CompactionDecision,
    pub action: CompactionRecoveryAction,
}

/// Convert the pure threshold decision into the next event-driven operation.
/// Keeping this mapping typed prevents loop/UI callers from duplicating
/// threshold rules or inventing ad-hoc JSON commands.
pub fn plan_compaction_recovery(
    context_tokens: u64,
    context_window: u64,
    settings: CompactionSettings,
) -> CompactionRecoveryPlan {
    let decision = compaction_decision(context_tokens, context_window, settings);
    let action = if decision.required() {
        CompactionRecoveryAction::Prepare {
            keep_recent_tokens: settings.keep_recent_tokens,
        }
    } else {
        CompactionRecoveryAction::Continue
    };
    CompactionRecoveryPlan { decision, action }
}

impl CompactionDecision {
    pub const fn required(self) -> bool {
        matches!(self, Self::Required { .. })
    }

    pub fn terminal_lines(self) -> Vec<String> {
        match self {
            Self::Disabled => vec!["compaction_policy: disabled".into()],
            Self::WithinBudget { available_tokens } => vec![format!(
                "compaction_policy: within_budget available_tokens={available_tokens}"
            )],
            Self::Required {
                context_tokens,
                threshold_tokens,
            } => vec![format!(
                "compaction_policy: required context_tokens={context_tokens} threshold_tokens={threshold_tokens}"
            )],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

/// Pi's conservative four-characters-per-token message estimate.
pub fn estimate_message_tokens(message: &AgentMessage) -> u64 {
    const ESTIMATED_IMAGE_CHARS: u64 = 4_800;
    let chars = match message {
        AgentMessage::User(message) => message
            .content
            .iter()
            .map(|content| match content {
                crate::types::UserContent::Text { text } => pi_text_units(text),
                crate::types::UserContent::Image { .. }
                | crate::types::UserContent::Video { .. }
                | crate::types::UserContent::Audio { .. } => ESTIMATED_IMAGE_CHARS,
            })
            .sum(),
        AgentMessage::Assistant(message) => message
            .content
            .iter()
            .map(|content| match content {
                crate::types::AssistantContent::Text { text }
                | crate::types::AssistantContent::Thinking { text } => pi_text_units(text),
                crate::types::AssistantContent::ToolCall(call) => {
                    pi_text_units(&call.name)
                        + serde_json::to_string(&call.arguments)
                            .map(|value| pi_text_units(&value))
                            .unwrap_or_default()
                }
            })
            .sum(),
        AgentMessage::ToolResult(message) => message
            .content
            .iter()
            .map(|content| match content {
                crate::types::ToolResultContent::Text { text } => pi_text_units(text),
                crate::types::ToolResultContent::Image { .. } => ESTIMATED_IMAGE_CHARS,
            })
            .sum(),
        AgentMessage::CompactionSummary(message) => pi_text_units(&message.summary),
        AgentMessage::Custom(_) => 0,
    };
    chars.saturating_add(3) / 4
}

/// JavaScript's `String.length` counts UTF-16 code units, which is the unit
/// used by Pi's token heuristic rather than Rust's UTF-8 byte length.
fn pi_text_units(text: &str) -> u64 {
    text.encode_utf16().count() as u64
}

/// Estimate context tokens using the latest valid assistant usage and the
/// conservative estimate for messages after it, matching Pi's harness.
pub fn estimate_context_tokens(messages: &[AgentMessage]) -> ContextUsageEstimate {
    let last_usage_index = messages
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, message)| {
            let AgentMessage::Assistant(assistant) = message else {
                return None;
            };
            let usage_tokens = assistant_usage_tokens(assistant);
            (assistant.stop_reason != Some(StopReason::Aborted)
                && assistant.stop_reason != Some(StopReason::Error)
                && usage_tokens > 0)
                .then_some(index)
        });
    let Some(index) = last_usage_index else {
        let trailing_tokens = messages.iter().map(estimate_message_tokens).sum();
        return ContextUsageEstimate {
            tokens: trailing_tokens,
            usage_tokens: 0,
            trailing_tokens,
            last_usage_index: None,
        };
    };
    let usage_tokens = match &messages[index] {
        AgentMessage::Assistant(assistant) => assistant_usage_tokens(assistant),
        _ => 0,
    };
    let trailing_tokens = messages[index + 1..]
        .iter()
        .map(estimate_message_tokens)
        .sum();
    ContextUsageEstimate {
        tokens: usage_tokens + trailing_tokens,
        usage_tokens,
        trailing_tokens,
        last_usage_index: Some(index),
    }
}

fn assistant_usage_tokens(message: &crate::types::AssistantMessage) -> u64 {
    let usage = &message.usage;
    if usage.total_tokens > 0 {
        usage.total_tokens
    } else {
        usage.input + usage.output + usage.cache_read + usage.cache_write
    }
}

/// Return whether Pi's harness should begin automatic compaction.
///
/// The summarizer and publication remain asynchronous actor-owned operations;
/// this function only makes the source-backed threshold decision.
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: CompactionSettings,
) -> bool {
    matches!(
        compaction_decision(context_tokens, context_window, settings),
        CompactionDecision::Required { .. }
    )
}

pub fn compaction_decision(
    context_tokens: u64,
    context_window: u64,
    settings: CompactionSettings,
) -> CompactionDecision {
    if !settings.enabled || context_window == 0 {
        return CompactionDecision::Disabled;
    }
    let threshold_tokens = context_window.saturating_sub(settings.reserve_tokens);
    if context_tokens > threshold_tokens {
        CompactionDecision::Required {
            context_tokens,
            threshold_tokens,
        }
    } else {
        CompactionDecision::WithinBudget {
            available_tokens: threshold_tokens.saturating_sub(context_tokens),
        }
    }
}

/// Pure provider-context boundary after the newest Pi compaction record.
///
/// The summary remains journal metadata until the provider-specific message
/// projector materializes it. Retained messages are carried by the
/// compaction record; ordinary message indices identify only entries written
/// after that boundary, so callers cannot accidentally send the compacted
/// prefix again.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CompactionContextProjection {
    pub summary: String,
    pub tokens_before: u64,
    pub timestamp: i64,
    pub retained_tail: Vec<AgentMessage>,
    pub message_indices: Vec<usize>,
}

impl CompactionContextProjection {
    pub fn terminal_lines(&self) -> Vec<String> {
        vec![
            format!("Compaction summary: {}", self.summary),
            format!("Tokens before: {}", self.tokens_before),
            format!("Retained messages: {}", self.retained_tail.len()),
            format!("Context message indices: {:?}", self.message_indices),
        ]
    }
}

pub fn is_provider_context_message(message: &AgentMessage) -> bool {
    !matches!(
        message,
        AgentMessage::Assistant(assistant)
            if assistant.stop_reason == Some(StopReason::Deferred)
    )
}
