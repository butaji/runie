//! Event hooks registry for extensibility.
//!
//! Hooks receive a JSON payload and return an `Allow`, `Deny`, or `Modify`
//! decision. The registry calls all handlers registered for an event; the first
//! deny wins, otherwise the last modification wins.

use serde_json::Value;
use std::collections::HashMap;

/// Lifecycle events that can be hooked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PermissionRequest,
    PreCompact,
    PostCompact,
    SessionStart,
    UserPromptSubmit,
    SubagentStart,
    SubagentStop,
    Stop,
}

/// Decision returned by a hook handler.
#[derive(Debug, Clone, PartialEq)]
pub enum HookDecision {
    /// Allow the action to proceed unchanged.
    Allow,
    /// Deny the action with an optional reason.
    Deny { reason: String },
    /// Allow the action but replace the payload.
    Modify { payload: Value },
}

impl HookDecision {
    /// Short string label for logging.
    pub fn label(&self) -> &'static str {
        match self {
            HookDecision::Allow => "allow",
            HookDecision::Deny { .. } => "deny",
            HookDecision::Modify { .. } => "modify",
        }
    }
}

/// A handler that participates in a hook event.
pub trait HookHandler: Send + Sync {
    /// Process the payload and return a decision.
    fn handle(&self, payload: &Value) -> HookDecision;
}

impl<F> HookHandler for F
where
    F: Fn(&Value) -> HookDecision + Send + Sync,
{
    fn handle(&self, payload: &Value) -> HookDecision {
        (self)(payload)
    }
}

/// Registry of hook handlers keyed by event.
#[derive(Default)]
pub struct HookRegistry {
    handlers: HashMap<HookEvent, Vec<Box<dyn HookHandler>>>,
}

impl HookRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for an event.
    pub fn register(&mut self, event: HookEvent, handler: Box<dyn HookHandler>) {
        self.handlers.entry(event).or_default().push(handler);
    }

    /// Emit an event to all registered handlers.
    ///
    /// Returns the folded decision: first `Deny` wins, otherwise the most recent
    /// `Modify` wins, otherwise `Allow`.
    pub fn emit(&self, event: HookEvent, payload: &Value) -> HookDecision {
        let handlers = match self.handlers.get(&event) {
            Some(h) => h,
            None => return HookDecision::Allow,
        };

        let mut decision = HookDecision::Allow;
        for handler in handlers {
            match handler.handle(payload) {
                HookDecision::Deny { reason } => return HookDecision::Deny { reason },
                modify @ HookDecision::Modify { .. } => decision = modify,
                HookDecision::Allow => {}
            }
        }
        decision
    }

    /// Load hooks declared in config.
    ///
    /// Each configured command receives the JSON payload on stdin and must print
    /// `allow`, `deny`, or a JSON object to replace the payload.
    pub fn load_from_config(config: &crate::config::Config) -> Self {
        let mut registry = Self::new();
        for (event_name, commands) in &config.hooks.commands {
            if let Some(event) = parse_event_name(event_name) {
                for cmd in commands {
                    registry.register(event, Box::new(ShellHook::new(cmd.clone())));
                }
            }
        }
        registry
    }
}

fn parse_event_name(name: &str) -> Option<HookEvent> {
    match name.to_ascii_lowercase().as_str() {
        "pretooluse" | "pre_tool_use" => Some(HookEvent::PreToolUse),
        "posttooluse" | "post_tool_use" => Some(HookEvent::PostToolUse),
        "permissionrequest" | "permission_request" => Some(HookEvent::PermissionRequest),
        "precompact" | "pre_compact" => Some(HookEvent::PreCompact),
        "postcompact" | "post_compact" => Some(HookEvent::PostCompact),
        "sessionstart" | "session_start" => Some(HookEvent::SessionStart),
        "userpromptsubmit" | "user_prompt_submit" => Some(HookEvent::UserPromptSubmit),
        "subagentstart" | "subagent_start" => Some(HookEvent::SubagentStart),
        "subagentstop" | "subagent_stop" => Some(HookEvent::SubagentStop),
        "stop" => Some(HookEvent::Stop),
        _ => None,
    }
}

/// Hook handler that runs an external command.
#[derive(Debug, Clone)]
pub struct ShellHook {
    command: String,
}

impl ShellHook {
    /// Create a new shell hook.
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl HookHandler for ShellHook {
    fn handle(&self, payload: &Value) -> HookDecision {
        let input = match serde_json::to_string(payload) {
            Ok(s) => s,
            Err(_) => return HookDecision::Allow,
        };
        crate::async_io::block_in_place_if_runtime(|| run_shell_hook(&self.command, &input))
    }
}

fn run_shell_hook(command: &str, input: &str) -> HookDecision {
    let output = match std::process::Command::new("sh")
        .args(["-c", command])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(input.as_bytes());
            }
            match child.wait_with_output() {
                Ok(o) => o,
                Err(_) => return HookDecision::Allow,
            }
        }
        Err(_) => return HookDecision::Allow,
    };

    if !output.status.success() {
        return HookDecision::Allow;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    match text.to_ascii_lowercase().as_str() {
        "allow" | "" => HookDecision::Allow,
        "deny" => HookDecision::Deny {
            reason: "hook denied".into(),
        },
        _ => match serde_json::from_str::<Value>(&text) {
            Ok(value) => HookDecision::Modify { payload: value },
            Err(_) => HookDecision::Allow,
        },
    }
}

/// Built-in hook that logs the event and returns `Allow`.
pub struct LoggingHook;

impl HookHandler for LoggingHook {
    fn handle(&self, payload: &Value) -> HookDecision {
        tracing::debug!(payload = %payload, "hook event");
        HookDecision::Allow
    }
}

/// Built-in hook that always allows permission requests.
pub struct PermissionHook;

impl HookHandler for PermissionHook {
    fn handle(&self, _payload: &Value) -> HookDecision {
        HookDecision::Allow
    }
}

/// Built-in hook that allows compaction events unchanged.
pub struct CompactionHook;

impl HookHandler for CompactionHook {
    fn handle(&self, _payload: &Value) -> HookDecision {
        HookDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_registry_calls_handler_on_event() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::PreToolUse,
            Box::new(|_payload: &Value| HookDecision::Deny {
                reason: "blocked".into(),
            }),
        );

        let decision = registry.emit(HookEvent::PreToolUse, &Value::Null);
        assert_eq!(
            decision,
            HookDecision::Deny {
                reason: "blocked".into()
            }
        );
    }

    #[test]
    fn hook_can_modify_input() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::UserPromptSubmit,
            Box::new(|_payload: &Value| HookDecision::Modify {
                payload: Value::String("transformed".into()),
            }),
        );

        let decision = registry.emit(HookEvent::UserPromptSubmit, &Value::Null);
        assert_eq!(
            decision,
            HookDecision::Modify {
                payload: Value::String("transformed".into())
            }
        );
    }

    #[test]
    fn hook_can_deny_action() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::PreToolUse,
            Box::new(|_payload: &Value| HookDecision::Deny {
                reason: "no tools".into(),
            }),
        );

        let decision = registry.emit(HookEvent::PreToolUse, &Value::Null);
        assert!(matches!(decision, HookDecision::Deny { .. }));
    }

    #[test]
    fn pre_tool_hook_intercepts_tool_call() {
        let mut registry = HookRegistry::new();
        registry.register(
            HookEvent::PreToolUse,
            Box::new(|payload: &Value| {
                if payload.get("tool").and_then(|v| v.as_str()) == Some("write_file") {
                    HookDecision::Deny {
                        reason: "writes blocked".into(),
                    }
                } else {
                    HookDecision::Allow
                }
            }),
        );

        let payload = serde_json::json!({"tool": "write_file"});
        let decision = registry.emit(HookEvent::PreToolUse, &payload);
        assert_eq!(
            decision,
            HookDecision::Deny {
                reason: "writes blocked".into()
            }
        );

        let payload = serde_json::json!({"tool": "read_file"});
        let decision = registry.emit(HookEvent::PreToolUse, &payload);
        assert_eq!(decision, HookDecision::Allow);
    }

    #[test]
    fn shell_hook_allow() {
        let hook = ShellHook::new("echo allow".into());
        let decision = hook.handle(&serde_json::json!({"x": 1}));
        assert_eq!(decision, HookDecision::Allow);
    }

    #[test]
    fn shell_hook_deny() {
        let hook = ShellHook::new("echo deny".into());
        let decision = hook.handle(&serde_json::json!({"x": 1}));
        assert_eq!(
            decision,
            HookDecision::Deny {
                reason: "hook denied".into()
            }
        );
    }

    #[test]
    fn shell_hook_modify() {
        let hook = ShellHook::new(r#"echo '{"modified": true}'"#.into());
        let decision = hook.handle(&serde_json::json!({"x": 1}));
        assert_eq!(
            decision,
            HookDecision::Modify {
                payload: serde_json::json!({"modified": true})
            }
        );
    }

    #[test]
    fn parse_event_name_handles_snake_case() {
        assert_eq!(
            parse_event_name("pre_tool_use"),
            Some(HookEvent::PreToolUse)
        );
        assert_eq!(parse_event_name("stop"), Some(HookEvent::Stop));
        assert_eq!(parse_event_name("unknown"), None);
    }
}
