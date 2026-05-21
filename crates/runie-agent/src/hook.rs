use async_trait::async_trait;
use runie_core::{ToolCall, ToolOutput, Context};
use thiserror::Error;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum HookDecision {
    Allow,
    Block { reason: String },
    Modify { args: Value },
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum HookError {
    #[error("hook error: {0}")]
    Failed(String),
}

#[async_trait]
pub trait Hook: Send + Sync {
    async fn before_tool_call(
        &self,
        tool_call: &ToolCall,
        context: &Context,
    ) -> Result<HookDecision, HookError>;

    async fn after_tool_call(
        &self,
        tool_call: &ToolCall,
        result: &ToolOutput,
        context: &Context,
    ) -> Result<ToolOutput, HookError>;
}

/// A hook that blocks dangerous commands.
pub struct SafetyHook;

#[async_trait]
impl Hook for SafetyHook {
    async fn before_tool_call(
        &self,
        tool_call: &ToolCall,
        _context: &Context,
    ) -> Result<HookDecision, HookError> {
        if tool_call.name == "bash" {
            let command = tool_call.arguments.get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let dangerous = ["rm -rf /", "mkfs", "dd if=/dev/zero", ":(){ :|:& };:"];
            for d in &dangerous {
                if command.contains(d) {
                    return Ok(HookDecision::Block { 
                        reason: format!("Dangerous command detected: {}", d) 
                    });
                }
            }
        }
        Ok(HookDecision::Allow)
    }

    async fn after_tool_call(
        &self,
        _tool_call: &ToolCall,
        result: &ToolOutput,
        _context: &Context,
    ) -> Result<ToolOutput, HookError> {
        Ok(result.clone())
    }
}
