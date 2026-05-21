use crate::events::*;
use crate::pi::AgentState;
use tidy_ai::{Provider, UnifiedApi};
use tokio::sync::mpsc;

pub struct AgentLoopConfig {
    pub model: String,
    pub system_prompt: String,
    pub tools: Vec<crate::pi::AgentTool>,
}

pub async fn run_agent_loop(
    messages: Vec<AgentMessage>,
    config: AgentLoopConfig,
    provider: &dyn Provider,
    event_sender: mpsc::UnboundedSender<AgentEvent>,
) -> Result<(), String> {
    // ReAct loop:
    // 1. Send messages to provider
    // 2. Stream response
    // 3. If tool calls, execute them
    // 4. Send tool results back
    // 5. Repeat until no more tool calls
    
    Ok(())
}
