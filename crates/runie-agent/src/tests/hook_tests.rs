use super::*;

struct BlockHook;
#[async_trait]
impl Hook for BlockHook {
    async fn before_tool_call(
        &self,
        _tool_call: &ToolCall,
        _context: &Context,
    ) -> Result<HookDecision, crate::hook::HookError> {
        Ok(HookDecision::Block { reason: "blocked by test".to_string() })
    }
    async fn after_tool_call(
        &self,
        _tool_call: &ToolCall,
        result: &ToolOutput,
        _context: &Context,
    ) -> Result<ToolOutput, crate::hook::HookError> {
        Ok(result.clone())
    }
}

async fn run_block_hook_test() -> bool {
    use crate::loop_engine::agent_loop;
    let provider = Arc::new(SimpleToolProvider::new());
    let registry = test_registry();
    let hook = Arc::new(BlockHook);
    let config = test_config(2);
    let messages = vec![test_message("test")];

    let mut stream = agent_loop(messages, config, provider, vec![], registry, vec![hook]);
    let mut tool_blocked = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::ToolExecutionEnd { result, .. }
                if result.is_error && result.content.iter().any(|c| matches!(c, ContentPart::Text { text } if text.contains("blocked"))) =>
            {
                tool_blocked = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }
    tool_blocked
}

#[tokio::test]
#[ignore]
async fn test_hook_block_prevents_execution() {
    let tool_blocked = run_block_hook_test().await;
    assert!(tool_blocked, "Tool should be blocked by hook");
}

struct ModifyHook;
#[async_trait]
impl Hook for ModifyHook {
    async fn before_tool_call(
        &self,
        tool_call: &ToolCall,
        _context: &Context,
    ) -> Result<HookDecision, crate::hook::HookError> {
        if tool_call.arguments.get("original").is_some() {
            Ok(HookDecision::Modify { args: serde_json::json!({"modified": true}) })
        } else {
            Ok(HookDecision::Allow)
        }
    }
    async fn after_tool_call(
        &self,
        _tool_call: &ToolCall,
        result: &ToolOutput,
        _context: &Context,
    ) -> Result<ToolOutput, crate::hook::HookError> {
        Ok(result.clone())
    }
}

async fn run_modify_hook_test() -> bool {
    use crate::loop_engine::agent_loop;
    let provider = Arc::new(ModifyArgsProvider::new());
    let registry = test_registry();
    let hook = Arc::new(ModifyHook);
    let config = test_config(2);
    let messages = vec![test_message("test")];

    let mut stream = agent_loop(messages, config, provider, vec![], registry, vec![hook]);
    let mut args_were_modified = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::ToolExecutionEnd { result, .. }
                if result.content.iter().any(|c| matches!(c, ContentPart::Text { text } if text.contains("modified"))) =>
            {
                args_were_modified = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }
    args_were_modified
}

#[tokio::test]
#[ignore]
async fn test_hook_modify_changes_args() {
    let args_were_modified = run_modify_hook_test().await;
    assert!(args_were_modified, "Hook should have modified args");
}
