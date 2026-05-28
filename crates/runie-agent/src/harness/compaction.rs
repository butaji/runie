use super::types::*;
use crate::events::*;

pub fn should_compact(
    _messages: &[AgentMessage],
    settings: &CompactionSettings,
    estimated_tokens: u32,
) -> bool {
    if !settings.enabled {
        return false;
    }
    estimated_tokens > settings.reserve_tokens
}

pub fn prepare_compaction(
    messages: &[AgentMessage],
    settings: &CompactionSettings,
) -> CompactionPreparation {
    // Find cut point: keep recent messages within keep_recent_tokens
    // Summarize older messages
    let cut_point = find_cut_point(messages, settings.keep_recent_tokens);
    
    let messages_to_summarize = messages[..cut_point].to_vec();
    let turn_prefix_messages = messages[cut_point..].to_vec();
    
    CompactionPreparation {
        first_kept_entry_id: format!("msg_{}", cut_point),
        messages_to_summarize,
        turn_prefix_messages,
        is_split_turn: false,
        tokens_before: estimate_tokens(messages),
        previous_summary: None,
        settings: settings.clone(),
    }
}

pub struct CompactionPreparation {
    pub first_kept_entry_id: String,
    pub messages_to_summarize: Vec<AgentMessage>,
    pub turn_prefix_messages: Vec<AgentMessage>,
    pub is_split_turn: bool,
    pub tokens_before: u32,
    pub previous_summary: Option<String>,
    pub settings: CompactionSettings,
}

pub(crate) fn find_cut_point(messages: &[AgentMessage], _keep_recent_tokens: u32) -> usize {
    // Simple heuristic: keep last 6 messages
    messages.len().saturating_sub(6)
}

fn estimate_tokens(messages: &[AgentMessage]) -> u32 {
    // Rough estimate: 4 chars per token
    let total_chars: usize = messages.iter()
        .map(|m| m.content.iter().map(|c| match c {
            ContentPart::Text { text } => text.len(),
            _ => 0,
        }).sum::<usize>())
        .sum();
    (total_chars / 4) as u32
}

pub async fn generate_summary(
    messages: &[AgentMessage],
    provider: &dyn runie_ai::Provider,
) -> Result<String, String> {
    let prompt = format!(
        "Summarize the following conversation concisely, preserving key facts and decisions:\n\n{}",
        messages.iter().map(|m| format!("{}: {:?}", m.role, m.content)).collect::<Vec<_>>().join("\n")
    );
    
    let llm_messages = vec![
        runie_core::Message::System { content: "You are a summarization assistant. Condense the conversation while preserving all important facts, decisions, and context.".to_string() },
        runie_core::Message::User { content: prompt, attachments: vec![] },
    ];
    
    provider.chat_simple(llm_messages).await
        .map_err(|e| e.to_string())
}