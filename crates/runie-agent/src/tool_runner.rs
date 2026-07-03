//! Shared tool execution helpers for agent turn and headless runners.
//!
//! Tools are executed via the MCP `ToolDef` trait. Each tool is called directly
//! with typed input parsed from the `ParsedToolCall`.

use std::sync::Arc;
use std::time::Duration;

use crate::PermissionGate;
use runie_core::harness_skills::{SkillRegistry, ToolCallCtx, ToolCallPhase, ToolCallResult};
#[cfg(test)]
use runie_core::message::Role;
use runie_core::message::{ChatMessage, Part};
use runie_core::permissions::{PermissionAction, PermissionContext};
#[cfg(feature = "mcp")]
use runie_core::tool::annotations::get_tool_annotations;
use runie_core::tool::ParsedToolCall;
use runie_core::tool::{
    is_builtin_tool, is_cacheable_tool, CacheEntry, ToolContext, ToolOutput, ToolResultCache,
    ToolStatus,
};
use tokio::time::timeout;

/// Default timeout for tool execution (30 seconds).
const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 30;

/// Execute a single parsed tool call, respecting the permission gate and tool-result cache.
///
/// If `cache` is `Some`, read-only tool results are cached by
/// `SHA256(tool_name + canonical_json(args))` and served from cache on repeated calls.
pub async fn execute_tool_call(
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    gate: &PermissionGate,
    cache: Option<Arc<ToolResultCache>>,
) -> ToolOutput {
    let tool_name = &tool_call.name;
    let perm_ctx = build_permission_context(tool_name, &tool_call.args, ctx.working_dir.as_ref());
    match gate.evaluate(&perm_ctx).await {
        PermissionAction::Allow => {
            // Check cache for read-only tools before dispatching.
            if let Some(ref c) = cache {
                if is_cacheable_tool(tool_name) {
                    let key = ToolResultCache::compute_key(tool_name, &tool_call.args);
                    if let Some(entry) = c.get(key) {
                        tracing::debug!(tool = %tool_name, "tool-result cache hit");
                        return cached_output(tool_name, tool_call.args.clone(), entry);
                    }
                }
            }

            let duration = Duration::from_secs(DEFAULT_TOOL_TIMEOUT_SECS);
            let output =
                match timeout(duration, dispatch_tool(tool_name, &tool_call.args, ctx)).await {
                    Ok(o) => o,
                    Err(_) => timeout_error(tool_name),
                };

            // Cache successful read-only results.
            if let (Some(ref c), ToolStatus::Success) = (&cache, &output.status) {
                if is_cacheable_tool(tool_name) {
                    let key = ToolResultCache::compute_key(tool_name, &tool_call.args);
                    let entry = CacheEntry {
                        tool_name: tool_name.clone(),
                        output: output.content.clone(),
                        bytes_transferred: output.bytes_transferred,
                        cached_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                    };
                    c.put(key, entry);
                    tracing::debug!(tool = %tool_name, "tool-result cached");
                }
            }

            output
        }
        PermissionAction::Deny | PermissionAction::Ask => {
            blocked_output(tool_name, tool_call.args.clone())
        }
    }
}

/// Dispatch a tool call by name — calls ToolDef::execute directly.
/// Permission evaluation is handled by the caller (`execute_tool_call`).
///
/// The dispatch is defined in `tool_registry.rs::dispatch_tool_impl`.
/// This function is a wrapper that delegates to it.
async fn dispatch_tool(name: &str, args: &serde_json::Value, ctx: &ToolContext) -> ToolOutput {
    crate::tool_registry::dispatch_tool_impl(name, args, ctx).await
}

/// Check if a tool name is known.
pub fn is_known_tool(name: &str) -> bool {
    is_builtin_tool(name)
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
        #[cfg(feature = "mcp")]
        annotations: get_tool_annotations(tool),
    }
}

fn blocked_output(tool_name: &str, args: serde_json::Value) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_owned(),
        tool_args: args,
        content: format!("Permission denied for tool '{}'", tool_name),
        bytes_transferred: None,
        duration: Duration::from_millis(0),
        status: ToolStatus::Blocked,
    }
}

/// Build a `ToolOutput` from a cached entry.
fn cached_output(
    tool_name: &str,
    args: serde_json::Value,
    entry: runie_core::tool::CacheEntry,
) -> ToolOutput {
    ToolOutput {
        tool_name: tool_name.to_string(),
        tool_args: args,
        content: entry.output,
        bytes_transferred: entry.bytes_transferred,
        duration: Duration::ZERO,
        status: ToolStatus::Success,
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

/// Execute a batch of tool calls with optional event observer and optional cache.
pub async fn execute_tools_with_observer(
    tools: &[ParsedToolCall],
    _cmd_id: &str,
    ctx: &ToolContext,
    gate: &PermissionGate,
    observer: &mut dyn ToolExecutorObserver,
    hooks: Option<&SkillRegistry>,
    cache: Option<Arc<ToolResultCache>>,
) -> Vec<ToolOutput> {
    let mut outputs = Vec::with_capacity(tools.len());
    for tool_call in tools {
        let output =
            execute_single_with_observer(tool_call, ctx, gate, observer, hooks, cache.clone())
                .await;
        outputs.push(output);
    }
    outputs
}

async fn execute_single_with_observer(
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    gate: &PermissionGate,
    observer: &mut dyn ToolExecutorObserver,
    hooks: Option<&SkillRegistry>,
    cache: Option<Arc<ToolResultCache>>,
) -> ToolOutput {
    observer.on_tool_start(&tool_call.name, &tool_call.args);
    let output = if let Some(skills) = hooks {
        execute_with_skill_hooks(tool_call, ctx, gate, skills, cache).await
    } else {
        execute_tool_call(tool_call, ctx, gate, cache).await
    };
    observer.on_tool_end(output.duration.as_secs_f64(), &output.content);
    output
}

async fn execute_with_skill_hooks(
    tool_call: &ParsedToolCall,
    ctx: &ToolContext,
    gate: &PermissionGate,
    skills: &SkillRegistry,
    cache: Option<Arc<ToolResultCache>>,
) -> ToolOutput {
    if let Some(output) = run_skill_before_hook(Some(skills), tool_call) {
        return output;
    }
    let output = execute_tool_call(tool_call, ctx, gate, cache).await;
    fire_skill_after_hook(Some(skills), tool_call, &output);
    output
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

// ── Unified skill hook helpers (shared with turn/tools) ─────────────────────────

/// Run before-hook for skill interception. Returns Some(output) if a skill
/// intercepted the call (skip or abort), None to continue.
pub(crate) fn run_skill_before_hook(
    skills: Option<&SkillRegistry>,
    tool_call: &ParsedToolCall,
) -> Option<ToolOutput> {
    let skills = skills?;
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

/// Fire after-hook for skill notification.
pub(crate) fn fire_skill_after_hook(
    skills: Option<&SkillRegistry>,
    tool_call: &ParsedToolCall,
    output: &ToolOutput,
) {
    if let Some(skills) = skills {
        skills.on_tool_call(&ToolCallCtx {
            tool_name: tool_call.name.clone(),
            tool_input: tool_call.args.clone(),
            phase: ToolCallPhase::After,
            tool_output: Some(output.content.clone()),
            success: Some(output.status == ToolStatus::Success),
        });
    }
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
            matches!(
                p,
                runie_core::message::Part::ToolResult {
                    output,
                    ..
                } if output.contains("[Lines")
            )
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

        execute_tools_with_observer(&tools, "req.0", &ctx, &gate, &mut observer, None, None).await;

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
            execute_tools_with_observer(&tools, "req.0", &ctx, &gate, &mut observer, None, None)
                .await;
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].tool_name, "list_dir");
        assert_eq!(outputs[0].status, ToolStatus::Success);
    }

    // Layer 2: cache hit returns cached output without tool dispatch.
    #[tokio::test]
    async fn tool_cache_hit_returns_cached_output() {
        use runie_core::tool::{CacheEntry, ToolResultCache};
        use std::sync::Arc;

        let cache = ToolResultCache::new(300);
        let tool_call = ParsedToolCall {
            name: "list_dir".to_string(),
            args: serde_json::json!({"path": "."}),
            id: Some("call_1".to_string()),
        };

        // Pre-populate the cache with a known result.
        let key = ToolResultCache::compute_key(&tool_call.name, &tool_call.args);
        cache.put(
            key,
            CacheEntry {
                tool_name: tool_call.name.clone(),
                output: "cached: src\ntests".to_string(),
                bytes_transferred: Some(42),
                cached_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            },
        );

        let gate = PermissionGate::new(
            runie_core::permissions::PermissionManager::default(),
            std::sync::Arc::new(runie_core::permissions::AutoAllowSink),
        );
        let ctx = ToolContext::default();

        let output = execute_tool_call(&tool_call, &ctx, &gate, Some(Arc::clone(&cache))).await;

        assert_eq!(output.status, ToolStatus::Success);
        assert_eq!(output.content, "cached: src\ntests");
        assert_eq!(output.bytes_transferred, Some(42));
        // Cached result has zero duration (served instantly)
        assert_eq!(output.duration, Duration::ZERO);
    }

    // Layer 2: write tools are never cached (not in CACHEABLE_TOOL_NAMES).
    #[tokio::test]
    async fn tool_cache_not_used_for_write_tools() {
        use runie_core::tool::{CacheEntry, ToolResultCache};
        use std::sync::Arc;

        let cache = ToolResultCache::new(300);
        let tool_call = ParsedToolCall {
            name: "write_file".to_string(),
            args: serde_json::json!({"path": "/tmp/cache_test.txt", "content": "test"}),
            id: Some("call_2".to_string()),
        };

        // Pre-populate the cache with a result (should not be used for write tools).
        let key = ToolResultCache::compute_key(&tool_call.name, &tool_call.args);
        cache.put(
            key,
            CacheEntry {
                tool_name: tool_call.name.clone(),
                output: "SHOULD NOT BE USED".to_string(),
                bytes_transferred: None,
                cached_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            },
        );

        let gate = PermissionGate::new(
            runie_core::permissions::PermissionManager::default(),
            std::sync::Arc::new(runie_core::permissions::AutoAllowSink),
        );
        let ctx = ToolContext::default();

        let output = execute_tool_call(&tool_call, &ctx, &gate, Some(Arc::clone(&cache))).await;

        // write_file is not cacheable, so it should actually execute and succeed
        assert_eq!(output.status, ToolStatus::Success);
        assert_ne!(
            output.content, "SHOULD NOT BE USED",
            "write_file should not use the cache"
        );

        let _ = std::fs::remove_file("/tmp/cache_test.txt");
    }
}
