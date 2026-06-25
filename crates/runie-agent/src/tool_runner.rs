//! Shared tool execution helpers for agent turn and headless runners.

#![allow(unused_imports)]
use crate::PermissionGate;
use runie_core::harness_skills::{SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult};
use runie_core::message::{ChatMessage, Part, Role};
use runie_core::permissions::{PermissionAction, PermissionContext};
use runie_core::tool::{ToolContext, ToolOutput, ToolStatus};
use runie_core::tool_parser::ParsedToolCall;
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for tool execution (30 seconds).
const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 30;

/// Execute a single parsed tool call, respecting the permission gate.
pub async fn execute_tool_call(
    registry: &runie_core::tool::ToolRegistry,
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    gate: &PermissionGate,
) -> ToolOutput {
    let tool_name = &tool_call.name;
    match registry.get(tool_name) {
        Some(tool) => {
            let perm_ctx = build_permission_context(tool_name, &tool_call.args, &ctx.working_dir);
            match gate.evaluate(&perm_ctx).await {
                PermissionAction::Allow => {
                    let duration = Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS);
                    match timeout(duration, run_tool(tool, tool_call, ctx)).await {
                        Ok(output) => output,
                        Err(_) => timeout_error(tool_name),
                    }
                }
                PermissionAction::Deny | PermissionAction::Ask => {
                    blocked_output(tool_name, tool_call)
                }
            }
        }
        None => unknown_tool_output(tool_name, tool_call),
    }
}

fn timeout_error(tool_name: &str) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_owned(),
        tool_args: serde_json::json!({}),
        content: format!(
            "Tool execution timed out after {} seconds",
            DEFAULT_TOOL_TIMEOUT_SECS
        ),
        bytes_transferred: None,
        duration: Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS),
        status: ToolStatus::Error,
    }
}

pub fn build_permission_context<'a>(
    tool: &'a str,
    input: &'a serde_json::Value,
    cwd: &'a std::path::Path,
) -> PermissionContext<'a> {
    let path = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(std::path::Path::new);
    PermissionContext {
        tool,
        path,
        input: Some(input),
        cwd: Some(cwd),
    }
}

async fn run_tool(
    tool: &std::sync::Arc<dyn runie_core::tool::Tool>,
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
) -> ToolOutput {
    tool.call(tool_call.args.clone(), ctx)
        .await
        .unwrap_or_else(|e| ToolOutput {
            tool_name: tool_call.name.clone(),
            tool_args: tool_call.args.clone(),
            content: format!("Tool execution failed: {}", e),
            bytes_transferred: None,
            duration: Duration::from_millis(0),
            status: ToolStatus::Error,
        })
}

fn blocked_output(tool_name: &str, tool_call: &ParsedToolCall) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_owned(),
        tool_args: tool_call.args.clone(),
        content: format!("Permission denied for tool '{}'", tool_name),
        bytes_transferred: None,
        duration: Duration::from_millis(0),
        status: ToolStatus::Blocked,
    }
}

fn unknown_tool_output(tool_name: &str, tool_call: &ParsedToolCall) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_owned(),
        tool_args: tool_call.args.clone(),
        content: format!("Error: unknown tool '{}'", tool_name),
        bytes_transferred: None,
        duration: Duration::from_millis(0),
        status: ToolStatus::Error,
    }
}

/// Build a tool-result chat message carrying the matching tool-call id.
pub fn tool_result_message(tool_call: &ParsedToolCall, output: &ToolOutput) -> ChatMessage {
    let id = tool_call.id.clone().unwrap_or_default();
    ChatMessage::tool_result(format!("{} result:\n{}", tool_call.name, output.content))
        .with_tool_call_id(id.clone())
        .with_parts(vec![Part::tool_result(id, &output.content)])
}

/// Observer for tool execution events.
pub trait ToolExecutorObserver {
    fn on_tool_start(&mut self, name: &str, input: &serde_json::Value);
    fn on_tool_end(&mut self, duration_secs: f64, output: &str);
}

/// No-op observer that emits no events.
impl ToolExecutorObserver for () {
    fn on_tool_start(&mut self, _name: &str, _input: &serde_json::Value) {}
    fn on_tool_end(&mut self, _duration_secs: f64, _output: &str) {}
}

/// Execute a batch of tool calls with optional event observer.
pub async fn execute_tools_with_observer(
    tools: &[ParsedToolCall],
    _cmd_id: &str,
    ctx: &ToolContext,
    gate: &PermissionGate,
    observer: &mut dyn ToolExecutorObserver,
    hooks: Option<&SkillRegistry>,
) -> Vec<ToolOutput> {
    let registry = runie_engine::tool::builtin_registry();
    let mut outputs = Vec::with_capacity(tools.len());
    for tool_call in tools {
        let output =
            execute_single_with_observer(tool_call, ctx, &registry, gate, observer, hooks).await;
        outputs.push(output);
    }
    outputs
}

async fn execute_single_with_observer(
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    registry: &runie_core::tool::ToolRegistry,
    gate: &PermissionGate,
    observer: &mut dyn ToolExecutorObserver,
    hooks: Option<&SkillRegistry>,
) -> ToolOutput {
    observer.on_tool_start(&tool_call.name, &tool_call.args);
    let output = if let Some(skills) = hooks {
        execute_with_skill_hooks(tool_call, ctx, registry, gate, skills).await
    } else {
        execute_tool_call(registry, tool_call, ctx, gate).await
    };
    observer.on_tool_end(output.duration.as_secs_f64(), &output.content);
    output
}

async fn execute_with_skill_hooks(
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    registry: &runie_core::tool::ToolRegistry,
    gate: &PermissionGate,
    skills: &SkillRegistry,
) -> ToolOutput {
    if let Some(output) = check_before_hook(skills, tool_call) {
        return output;
    }
    let output = execute_tool_call(registry, tool_call, ctx, gate).await;
    fire_after_hook(skills, tool_call, &output);
    output
}

fn check_before_hook(skills: &SkillRegistry, tool_call: &ParsedToolCall) -> Option<ToolOutput> {
    let tool_ctx = ToolCallCtx {
        tool_name: tool_call.name.clone(),
        tool_input: tool_call.args.clone(),
        phase: ToolCallPhase::Before,
        tool_output: None,
        success: None,
    };
    match skills.on_tool_call(&tool_ctx) {
        ToolCallResult::SkipWithOutput(output) => Some(skip_output(tool_call, output)),
        ToolCallResult::Abort(reason) => Some(abort_output(tool_call, &reason)),
        ToolCallResult::Continue => None,
    }
}

fn skip_output(tool_call: &ParsedToolCall, output: String) -> ToolOutput {
    ToolOutput {
        tool_name: tool_call.name.clone(),
        tool_args: tool_call.args.clone(),
        content: output,
        bytes_transferred: None,
        duration: Duration::from_millis(0),
        status: ToolStatus::Success,
    }
}

fn abort_output(tool_call: &ParsedToolCall, reason: &str) -> ToolOutput {
    ToolOutput {
        tool_name: tool_call.name.clone(),
        tool_args: tool_call.args.clone(),
        content: format!("Tool {} aborted: {}", tool_call.name, reason),
        bytes_transferred: None,
        duration: Duration::from_millis(0),
        status: ToolStatus::Error,
    }
}

fn fire_after_hook(skills: &SkillRegistry, tool_call: &ParsedToolCall, output: &ToolOutput) {
    skills.on_tool_call(&ToolCallCtx {
        tool_name: tool_call.name.clone(),
        tool_input: tool_call.args.clone(),
        phase: ToolCallPhase::After,
        tool_output: Some(output.content.clone()),
        success: Some(output.status == ToolStatus::Success),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tool_timeout_returns_error() {
        async fn slow_op() -> ToolOutput {
            tokio::time::sleep(Duration::from_secs(5)).await;
            ToolOutput {
                tool_name: "slow".to_string(),
                tool_args: serde_json::json!({}),
                content: "done".to_string(),
                bytes_transferred: None,
                duration: Duration::from_secs(5),
                status: ToolStatus::Success,
            }
        }
        let result = timeout(Duration::from_millis(100), slow_op()).await;
        assert!(result.is_err(), "timeout should trigger");
    }

    // Layer 1: shared helper produces correct ChatMessage for tool result.
    #[test]
    fn execute_tool_call_builds_result_message() {
        let tool_call = ParsedToolCall {
            name: "read_file".to_string(),
            args: serde_json::json!({"path": "Cargo.toml"}),
            id: Some("call_1".to_string()),
        };
        let output = ToolOutput {
            tool_name: "read_file".to_string(),
            tool_args: serde_json::json!({"path": "Cargo.toml"}),
            content: "[Lines 1-5]".to_string(),
            bytes_transferred: None,
            duration: Duration::from_millis(10),
            status: ToolStatus::Success,
        };
        let msg = tool_result_message(&tool_call, &output);
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.tool_call_id, Some("call_1".to_string()));
        // Check that the message has a ToolResult part with the output
        let has_tool_result = msg.parts.iter().any(|p| {
            matches!(p, runie_core::message::Part::ToolResult { output, .. }
                if output.contains("[Lines"))
        });
        assert!(
            has_tool_result,
            "Expected ToolResult part with output content"
        );
    }

    // Layer 2: with observer, ToolStart/ToolEnd are emitted.
    #[tokio::test]
    async fn interactive_tool_execution_emits_events() {
        struct TestObserver {
            events: Vec<String>,
        }
        impl ToolExecutorObserver for TestObserver {
            fn on_tool_start(&mut self, name: &str, _input: &serde_json::Value) {
                self.events.push(format!("start:{}", name));
            }
            fn on_tool_end(&mut self, _duration_secs: f64, output: &str) {
                self.events.push(format!("end:{}", output.len()));
            }
        }

        let gate = PermissionGate::new(
            runie_core::permissions::PermissionManager::default(),
            std::sync::Arc::new(runie_core::permissions::AutoAllowSink),
        );
        let tools = vec![ParsedToolCall {
            name: "list_dir".to_string(),
            args: serde_json::json!({"path": "."}),
            id: Some("call_1".to_string()),
        }];
        let ctx = ToolContext::default();
        let mut observer = TestObserver { events: Vec::new() };

        execute_tools_with_observer(&tools, "req.0", &ctx, &gate, &mut observer, None).await;

        assert!(observer
            .events
            .iter()
            .any(|e| e.starts_with("start:list_dir")));
        assert!(observer.events.iter().any(|e| e.starts_with("end:")));
    }

    // Layer 2: without observer (headless), no events emitted.
    #[tokio::test]
    async fn headless_tool_execution_silent() {
        let gate = PermissionGate::new(
            runie_core::permissions::PermissionManager::default(),
            std::sync::Arc::new(runie_core::permissions::AutoAllowSink),
        );
        let tools = vec![ParsedToolCall {
            name: "list_dir".to_string(),
            args: serde_json::json!({"path": "."}),
            id: Some("call_1".to_string()),
        }];
        let ctx = ToolContext::default();
        let mut observer: () = ();

        let outputs =
            execute_tools_with_observer(&tools, "req.0", &ctx, &gate, &mut observer, None).await;
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].tool_name, "list_dir");
        assert_eq!(outputs[0].status, ToolStatus::Success);
    }
}
