//! Permission prompt hook for rig-core agent integration.
//!
//! **NOTE**: This hook is designed for migrating to rig's Agent when we replace
//! our custom `run_agent_loop` with rig's agent system. For the current loop,
//! we continue using `PermissionGate` which works with the existing event system.
//!
//! # Migration Path
//! 1. Replace `run_agent_loop` calls with rig `Agent::run` 
//! 2. Use `PermissionPromptHook::new()` to get (hook, event_rx, decision_tx)
//! 3. Route `event_rx` to TUI as permission requests
//! 4. Route TUI decisions back through `decision_tx`
//!
//! This module provides a `PermissionPromptHook` that intercepts tool calls
//! and requests permission from the UI before execution.

use rig_core::agent::{PromptHook, ToolCallHookAction};
use rig_core::completion::CompletionModel;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashSet;

/// Events sent FROM the hook TO the UI for permission requests
#[derive(Debug, Clone)]
pub enum PermissionEvent {
    Request {
        tool_call_id: String,
        tool_name: String,
        tool_args: String,
    },
    Granted {
        tool_call_id: String,
    },
    Denied {
        tool_call_id: String,
    },
}

/// Decisions sent FROM the UI TO the hook
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow {
        tool_call_id: String,
    },
    AllowAlways {
        tool_call_id: String,
    },
    Skip {
        tool_call_id: String,
    },
    Deny {
        tool_call_id: String,
    },
}

#[derive(Clone)]
pub struct PermissionPromptHook {
    event_tx: mpsc::Sender<PermissionEvent>,
    decision_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<PermissionDecision>>>,
    allowed_tools: Arc<tokio::sync::Mutex<HashSet<String>>>,
    timeout_secs: u64,
}

impl PermissionPromptHook {
    pub fn new(
        timeout_secs: u64,
    ) -> (
        Self,
        mpsc::Receiver<PermissionEvent>,
        mpsc::Sender<PermissionDecision>,
    ) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (decision_tx, decision_rx) = mpsc::channel(100);

        let hook = Self {
            event_tx,
            decision_rx: Arc::new(tokio::sync::Mutex::new(decision_rx)),
            allowed_tools: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            timeout_secs,
        };

        (hook, event_rx, decision_tx)
    }
}

impl<M: CompletionModel> PromptHook<M> for PermissionPromptHook {
    fn on_tool_call(
        &self,
        tool_name: &str,
        tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> impl std::future::Future<Output = ToolCallHookAction> + Send {
        let tool_name = tool_name.to_string();
        let args = args.to_string();
        let call_id = tool_call_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let event_tx = self.event_tx.clone();
        let decision_rx = self.decision_rx.clone();
        let allowed_tools = self.allowed_tools.clone();
        let timeout_secs = self.timeout_secs;

        async move {
            // Check if tool is already allowed
            {
                let allowed = allowed_tools.lock().await;
                if allowed.contains(&tool_name) {
                    return ToolCallHookAction::Continue;
                }
            }

            // Send permission request to UI
            let _ = event_tx
                .send(PermissionEvent::Request {
                    tool_call_id: call_id.clone(),
                    tool_name: tool_name.clone(),
                    tool_args: args,
                })
                .await;

            // Wait for decision with timeout
            let timeout = Duration::from_secs(timeout_secs);
            let decision = match tokio::time::timeout(timeout, async {
                let mut rx = decision_rx.lock().await;
                rx.recv().await
            })
            .await
            {
                Ok(Some(decision)) => decision,
                _ => {
                    // Timeout or channel closed - deny by default
                    let _ = event_tx
                        .send(PermissionEvent::Denied {
                            tool_call_id: call_id.clone(),
                        })
                        .await;
                    return ToolCallHookAction::Skip {
                        reason: "Permission timeout - tool execution denied".to_string(),
                    };
                }
            };

            match decision {
                PermissionDecision::Allow { tool_call_id: ref tid }
                    if tid == &call_id =>
                {
                    let _ = event_tx
                        .send(PermissionEvent::Granted {
                            tool_call_id: call_id,
                        })
                        .await;
                    ToolCallHookAction::Continue
                }
                PermissionDecision::AllowAlways { tool_call_id: ref tid }
                    if tid == &call_id =>
                {
                    {
                        let mut allowed = allowed_tools.lock().await;
                        allowed.insert(tool_name);
                    }
                    let _ = event_tx
                        .send(PermissionEvent::Granted {
                            tool_call_id: call_id,
                        })
                        .await;
                    ToolCallHookAction::Continue
                }
                PermissionDecision::Skip { .. } => ToolCallHookAction::Skip {
                    reason: "Tool execution skipped by user".to_string(),
                },
                _ => {
                    let _ = event_tx
                        .send(PermissionEvent::Denied {
                            tool_call_id: call_id,
                        })
                        .await;
                    ToolCallHookAction::Skip {
                        reason: "Tool execution denied by user".to_string(),
                    }
                }
            }
        }
    }
}
