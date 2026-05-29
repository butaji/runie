use super::*;

#[tokio::test]
#[ignore]
async fn test_hook_block_prevents_execution() {
    use crate::loop_engine::agent_loop;

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

    let provider = Arc::new(SimpleToolProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));
    let hook = Arc::new(BlockHook);

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![hook],
    );

    let mut tool_blocked = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::ToolExecutionEnd { result, .. } if result.is_error && result.content.iter().any(|c| matches!(c, ContentPart::Text { text } if text.contains("blocked"))) => {
                tool_blocked = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }

    assert!(tool_blocked, "Tool should be blocked by hook");
}

#[tokio::test]
#[ignore]
async fn test_hook_modify_changes_args() {
    use crate::loop_engine::agent_loop;

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

    let provider = Arc::new(ModifyArgsProvider::new());
    let ws = Workspace::new(PathBuf::from("."));
    let registry = Arc::new(create_default_toolkit(ws));
    let hook = Arc::new(ModifyHook);

    let config = AgentLoopConfig {
        system_prompt: "You are helpful".to_string(),
        model: "test".to_string(),
        thinking_level: "low".to_string(),
        max_turns: 2,
    };

    let messages = vec![AgentMessage {
        role: "user".to_string(),
        content: vec![ContentPart::Text { text: "test".to_string() }],
        timestamp: 0,
        usage: None,
        stop_reason: None,
        error_message: None,
        tool_calls: vec![],
    }];

    let mut stream = agent_loop(
        messages,
        config,
        provider,
        vec![],
        registry,
        vec![hook],
    );

    let mut args_were_modified = false;

    while let Some(event) = stream.next().await {
        match &event {
            AgentEvent::ToolExecutionEnd { result, .. } if result.content.iter().any(|c| matches!(c, ContentPart::Text { text } if text.contains("modified"))) => {
                args_were_modified = true;
                break;
            }
            AgentEvent::AgentEnd { .. } => {
                break;
            }
            _ => {}
        }
    }

    assert!(args_were_modified, "Hook should have modified args");
}
